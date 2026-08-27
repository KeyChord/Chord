//! `chord native-run` / `chord native-bench`: drive a native handler library through a
//! throwaway `chord-native-host` without launching the app. Useful for package authors testing
//! a freshly built library and as the latency gate for the invocation hot path.

use crate::app::native_host::NativeHostSupervisor;
use crate::constants::{NATIVE_HOST_READY_TIMEOUT, NATIVE_INVOCATION_TIMEOUT};
use anyhow::{Context, Result};
use chord_native_protocol::client::{
    HostError, HostLogStream, HostProcess, HostSpawnOptions, InvokeOutcome,
};
use chord_native_protocol::{InvocationContext, InvocationResult, NativeHandlerRegistration};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

const HANDLER_ID: &str = "cli";

/// Starts a host whose allowed cache dir is the library's own directory and loads the library
/// as the single handler `cli`.
async fn spawn_and_load(library: &Path, handler_arguments: Vec<String>) -> Result<HostProcess> {
    let library = std::fs::canonicalize(library)
        .with_context(|| format!("library {} not found", library.display()))?;
    let cache_dir = library
        .parent()
        .context("library has no parent directory")?
        .to_path_buf();
    let binary = NativeHostSupervisor::host_binary_path()?;

    let mut host = HostProcess::spawn(HostSpawnOptions {
        binary,
        cache_dir,
        log_sink: Some(Arc::new(|stream, line| match stream {
            HostLogStream::Stdout => eprintln!("[native-host stdout] {line}"),
            HostLogStream::Stderr => eprintln!("[native-host stderr] {line}"),
        })),
        ready_timeout: NATIVE_HOST_READY_TIMEOUT,
    })
    .await?;

    let outcome = host
        .load_generation(
            1,
            vec![NativeHandlerRegistration {
                handler_id: HANDLER_ID.into(),
                library_path: library,
                handler_arguments,
                package_name: "cli".into(),
                chords_file_pathslug: "chords/cli.toml".into(),
            }],
            NATIVE_HOST_READY_TIMEOUT,
        )
        .await?;
    if let Err(errors) = outcome {
        anyhow::bail!("failed to load library: {}", errors[0].message);
    }
    Ok(host)
}

pub async fn native_run(
    library: impl AsRef<Path>,
    handler_args: Vec<String>,
    event_args: Vec<String>,
    repeat: u32,
) -> Result<()> {
    let mut host = spawn_and_load(library.as_ref(), handler_args).await?;
    let outcome = host
        .invoke(
            HANDLER_ID,
            event_args,
            repeat,
            InvocationContext::default(),
            NATIVE_INVOCATION_TIMEOUT,
        )
        .await?;
    host.shutdown(Duration::from_secs(2)).await;

    match outcome.result {
        InvocationResult::Success => {
            println!(
                "ok (native {:?}, round trip {:?})",
                outcome.native_duration, outcome.round_trip
            );
            Ok(())
        }
        InvocationResult::Thrown { message } => anyhow::bail!("handler threw: {message}"),
        InvocationResult::InvalidArguments { message } => {
            anyhow::bail!("invalid arguments: {message}")
        }
        InvocationResult::WrapperFailure { message } => {
            anyhow::bail!("wrapper failure: {message}")
        }
    }
}

pub async fn native_bench(library: impl AsRef<Path>, iterations: u32) -> Result<()> {
    let mut host = spawn_and_load(library.as_ref(), Vec::new()).await?;

    for _ in 0..200 {
        invoke_noop(&mut host).await?;
    }

    let mut round_trips = Vec::with_capacity(iterations as usize);
    let mut native = Vec::with_capacity(iterations as usize);
    for _ in 0..iterations {
        let outcome = invoke_noop(&mut host).await?;
        if !matches!(outcome.result, InvocationResult::Success) {
            anyhow::bail!("benchmark handler failed: {:?}", outcome.result);
        }
        round_trips.push(outcome.round_trip);
        native.push(outcome.native_duration);
    }
    host.shutdown(Duration::from_secs(2)).await;

    round_trips.sort();
    native.sort();
    let pct = |samples: &[Duration], p: f64| {
        samples[((samples.len() as f64 * p) as usize).min(samples.len() - 1)]
    };
    println!("native handler round trip over {iterations} invocations:");
    println!(
        "  p50 {:?}   p95 {:?}   p99 {:?}   max {:?}",
        pct(&round_trips, 0.50),
        pct(&round_trips, 0.95),
        pct(&round_trips, 0.99),
        round_trips.last().unwrap()
    );
    println!(
        "  native call only: p50 {:?}   p99 {:?}",
        pct(&native, 0.50),
        pct(&native, 0.99)
    );

    let targets = [
        (50, Duration::from_millis(2)),
        (95, Duration::from_millis(5)),
        (99, Duration::from_millis(10)),
    ];
    let mut ok = true;
    for (p, target) in targets {
        let measured = pct(&round_trips, p as f64 / 100.0);
        if measured > target {
            ok = false;
            println!("  FAIL p{p} {measured:?} exceeds target {target:?}");
        }
    }
    anyhow::ensure!(ok, "latency targets exceeded");
    println!("  latency targets met");
    Ok(())
}

async fn invoke_noop(host: &mut HostProcess) -> Result<InvokeOutcome, HostError> {
    host.invoke(
        HANDLER_ID,
        Vec::new(),
        1,
        InvocationContext::default(),
        NATIVE_INVOCATION_TIMEOUT,
    )
    .await
}
