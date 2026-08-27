//! End-to-end tests for `chord-native-host` driven through the shared client. Swift fixtures are
//! compiled at test time with the toolchain from `xcrun`; the tests are skipped (with a message)
//! when no Swift toolchain is available.
#![cfg(target_os = "macos")]

use chord_native_protocol::client::{HostError, HostLogStream, HostProcess, HostSpawnOptions};
use chord_native_protocol::{InvocationContext, InvocationResult, NativeHandlerRegistration};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

const WRAPPER: &str = include_str!("../../chord-native-protocol/swift/ChordEntry.swift");

fn xcrun_find(tool: &str) -> Option<PathBuf> {
    let output = Command::new("xcrun").args(["--find", tool]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(PathBuf::from(String::from_utf8_lossy(&output.stdout).trim()))
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// All compiled fixtures live under one directory, which doubles as the host's allowed cache dir.
fn out_root() -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/native-fixtures");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::canonicalize(dir).unwrap()
}

struct Fixture {
    name: &'static str,
    swift_sources: &'static [&'static str],
    c_sources: &'static [&'static str],
    bridging_header: Option<&'static str>,
    with_wrapper: bool,
    frameworks: &'static [&'static str],
}

impl Fixture {
    const fn swift(name: &'static str) -> Self {
        Self {
            name,
            swift_sources: &[],
            c_sources: &[],
            bridging_header: None,
            with_wrapper: true,
            frameworks: &[],
        }
    }
}

fn build_fixture(fixture: &Fixture) -> Option<PathBuf> {
    static CACHE: OnceLock<Mutex<HashMap<&'static str, PathBuf>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut cache = cache.lock().unwrap();
    if let Some(path) = cache.get(fixture.name) {
        return Some(path.clone());
    }

    let swiftc = xcrun_find("swiftc")?;
    let clang = xcrun_find("clang")?;
    let build_dir = out_root().join(fixture.name);
    let _ = std::fs::remove_dir_all(&build_dir);
    std::fs::create_dir_all(&build_dir).unwrap();

    let mut objects = Vec::new();
    for c in fixture.c_sources {
        let object = build_dir.join(format!("{c}.o"));
        let status = Command::new(&clang)
            .args(["-c", "-fPIC"])
            .arg(fixtures_dir().join(c))
            .arg("-o")
            .arg(&object)
            .status()
            .unwrap();
        assert!(status.success(), "clang failed for {c}");
        objects.push(object);
    }

    let mut sources: Vec<PathBuf> = if fixture.swift_sources.is_empty() {
        vec![fixtures_dir().join(format!("{}.swift", fixture.name))]
    } else {
        fixture
            .swift_sources
            .iter()
            .map(|s| fixtures_dir().join(s))
            .collect()
    };
    if fixture.with_wrapper {
        let wrapper = build_dir.join("ChordEntry.swift");
        std::fs::write(&wrapper, WRAPPER).unwrap();
        sources.push(wrapper);
    }

    let output = build_dir.join(format!("{}.dylib", fixture.name));
    let mut command = Command::new(&swiftc);
    command
        .args(["-parse-as-library", "-emit-library", "-Onone"])
        .arg("-module-name")
        .arg(format!("Fixture_{}", fixture.name));
    if let Some(header) = fixture.bridging_header {
        command
            .arg("-import-objc-header")
            .arg(fixtures_dir().join(header));
    }
    for framework in fixture.frameworks {
        command.arg("-framework").arg(framework);
    }
    command.args(&sources).args(&objects).arg("-o").arg(&output);
    let result = command.output().unwrap();
    assert!(
        result.status.success(),
        "swiftc failed for {}:\n{}",
        fixture.name,
        String::from_utf8_lossy(&result.stderr)
    );

    cache.insert(fixture.name, output.clone());
    Some(output)
}

type Lines = Arc<Mutex<Vec<(HostLogStream, String)>>>;

async fn spawn_host(lines: Option<Lines>) -> HostProcess {
    let sink = lines.map(|lines| {
        Arc::new(move |stream: HostLogStream, line: &str| {
            lines.lock().unwrap().push((stream, line.to_string()));
        }) as chord_native_protocol::client::LogSink
    });
    HostProcess::spawn(HostSpawnOptions {
        binary: PathBuf::from(env!("CARGO_BIN_EXE_chord-native-host")),
        cache_dir: out_root(),
        log_sink: sink,
        ready_timeout: Duration::from_secs(10),
    })
    .await
    .expect("host should start")
}

fn registration(id: &str, library: &Path, handler_arguments: &[&str]) -> NativeHandlerRegistration {
    NativeHandlerRegistration {
        handler_id: id.to_string(),
        library_path: library.to_path_buf(),
        handler_arguments: handler_arguments.iter().map(|s| s.to_string()).collect(),
        package_name: "test-pkg".into(),
        chords_file_pathslug: "chords/macos.toml".into(),
    }
}

async fn load(host: &mut HostProcess, regs: Vec<NativeHandlerRegistration>) {
    let outcome = host
        .load_generation(1, regs, Duration::from_secs(10))
        .await
        .expect("host alive");
    if let Err(errors) = outcome {
        panic!("generation failed to load: {errors:#?}");
    }
}

async fn invoke(
    host: &mut HostProcess,
    id: &str,
    args: &[&str],
) -> Result<InvocationResult, HostError> {
    host.invoke(
        id,
        args.iter().map(|s| s.to_string()).collect(),
        1,
        InvocationContext {
            focused_app_id: Some("com.apple.Safari".into()),
        },
        Duration::from_secs(10),
    )
    .await
    .map(|outcome| outcome.result)
}

macro_rules! require_swift {
    ($fixture:expr) => {
        match build_fixture(&$fixture) {
            Some(path) => path,
            None => {
                eprintln!("skipping: no Swift toolchain available via xcrun");
                return;
            }
        }
    };
}

#[tokio::test]
async fn noop_succeeds_repeatedly_on_one_host() {
    let noop = require_swift!(Fixture::swift("noop"));
    let mut host = spawn_host(None).await;
    load(&mut host, vec![registration("noop", &noop, &[])]).await;
    let pid = host.pid();
    for _ in 0..5 {
        assert_eq!(
            invoke(&mut host, "noop", &[]).await.unwrap(),
            InvocationResult::Success
        );
    }
    assert_eq!(host.pid(), pid);
    host.shutdown(Duration::from_secs(2)).await;
}

#[tokio::test]
async fn static_and_event_arguments_and_env_reach_the_handler() {
    let echo = require_swift!(Fixture::swift("echo"));
    let mut host = spawn_host(None).await;
    load(&mut host, vec![registration("echo", &echo, &["Safari", "1"])]).await;
    assert_eq!(
        invoke(&mut host, "echo", &["by-letters", "x"]).await.unwrap(),
        InvocationResult::Success
    );
    match invoke(&mut host, "echo", &["by-index", "2"]).await.unwrap() {
        InvocationResult::Thrown { message } => assert!(message.contains("event="), "{message}"),
        other => panic!("expected Thrown, got {other:?}"),
    }
}

#[tokio::test]
async fn thrown_error_is_reported_and_host_survives() {
    let throws = require_swift!(Fixture::swift("throws"));
    let noop = require_swift!(Fixture::swift("noop"));
    let mut host = spawn_host(None).await;
    load(
        &mut host,
        vec![
            registration("throws", &throws, &[]),
            registration("noop", &noop, &[]),
        ],
    )
    .await;
    let pid = host.pid();
    match invoke(&mut host, "throws", &["a"]).await.unwrap() {
        InvocationResult::Thrown { message } => {
            assert!(message.contains("FixtureError"), "{message}");
            assert!(message.contains("expected failure"), "{message}");
        }
        other => panic!("expected Thrown, got {other:?}"),
    }
    assert_eq!(
        invoke(&mut host, "noop", &[]).await.unwrap(),
        InvocationResult::Success
    );
    assert_eq!(host.pid(), pid);
}

#[tokio::test]
async fn fatal_error_kills_only_the_host() {
    let fatal = require_swift!(Fixture::swift("fatal"));
    let noop = require_swift!(Fixture::swift("noop"));
    let mut host = spawn_host(None).await;
    load(&mut host, vec![registration("fatal", &fatal, &[])]).await;
    let first_pid = host.pid();
    match invoke(&mut host, "fatal", &[]).await {
        Err(HostError::Exited {
            status,
            stderr_tail,
        }) => {
            assert!(status.contains("SIG"), "status: {status}");
            assert!(stderr_tail.contains("fixture crash"), "tail: {stderr_tail}");
        }
        other => panic!("expected host exit, got {other:?}"),
    }

    // The parent (this test) is alive; a replacement host works.
    let mut host = spawn_host(None).await;
    assert_ne!(host.pid(), first_pid);
    load(&mut host, vec![registration("noop", &noop, &[])]).await;
    assert_eq!(
        invoke(&mut host, "noop", &[]).await.unwrap(),
        InvocationResult::Success
    );
}

#[tokio::test]
async fn hanging_handler_times_out_and_host_is_killed() {
    let hang = require_swift!(Fixture::swift("hang"));
    let mut host = spawn_host(None).await;
    load(&mut host, vec![registration("hang", &hang, &[])]).await;
    let result = host
        .invoke(
            "hang",
            vec![],
            1,
            InvocationContext::default(),
            Duration::from_millis(500),
        )
        .await;
    assert!(matches!(result, Err(HostError::TimedOut(_))), "{result:?}");
    // After a timeout the host has been killed; a further request fails fast.
    let again = invoke(&mut host, "hang", &[]).await;
    assert!(again.is_err());
}

#[tokio::test]
async fn printed_output_goes_to_logs_not_the_protocol_stream() {
    let prints = require_swift!(Fixture::swift("prints"));
    let lines: Lines = Arc::default();
    let mut host = spawn_host(Some(Arc::clone(&lines))).await;
    load(&mut host, vec![registration("prints", &prints, &[])]).await;
    for _ in 0..3 {
        assert_eq!(
            invoke(&mut host, "prints", &[]).await.unwrap(),
            InvocationResult::Success
        );
    }
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let captured = lines.lock().unwrap().clone();
        let stdout = captured
            .iter()
            .any(|(s, l)| *s == HostLogStream::Stdout && l == "hello from swift stdout");
        let stderr = captured
            .iter()
            .any(|(s, l)| *s == HostLogStream::Stderr && l == "hello from swift stderr");
        if stdout && stderr {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "log lines not captured: {captured:?}"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn repeat_invokes_the_handler_n_times() {
    let counter = require_swift!(Fixture::swift("counter"));
    let file = out_root().join("counter.log");
    let _ = std::fs::remove_file(&file);
    let mut host = spawn_host(None).await;
    load(
        &mut host,
        vec![registration(
            "counter",
            &counter,
            &[file.to_str().unwrap()],
        )],
    )
    .await;
    let outcome = host
        .invoke(
            "counter",
            vec![],
            5,
            InvocationContext::default(),
            Duration::from_secs(10),
        )
        .await
        .unwrap();
    assert_eq!(outcome.result, InvocationResult::Success);
    assert_eq!(std::fs::read_to_string(&file).unwrap().lines().count(), 5);
}

#[tokio::test]
async fn c_companion_code_links_into_the_handler() {
    let with_c = require_swift!(Fixture {
        c_sources: &["add.c"],
        bridging_header: Some("add.h"),
        ..Fixture::swift("with_c")
    });
    let mut host = spawn_host(None).await;
    load(&mut host, vec![registration("with_c", &with_c, &[])]).await;
    assert_eq!(
        invoke(&mut host, "with_c", &[]).await.unwrap(),
        InvocationResult::Success
    );
}

#[tokio::test]
async fn appkit_is_usable_from_the_host_main_thread() {
    let appkit = require_swift!(Fixture {
        frameworks: &["AppKit"],
        ..Fixture::swift("appkit")
    });
    let mut host = spawn_host(None).await;
    load(&mut host, vec![registration("appkit", &appkit, &[])]).await;
    assert_eq!(
        invoke(&mut host, "appkit", &[]).await.unwrap(),
        InvocationResult::Success
    );
}

#[tokio::test]
async fn missing_entrypoint_is_a_load_failure() {
    let no_symbol = require_swift!(Fixture {
        with_wrapper: false,
        ..Fixture::swift("no_symbol")
    });
    let noop = require_swift!(Fixture::swift("noop"));
    let mut host = spawn_host(None).await;
    let outcome = host
        .load_generation(
            1,
            vec![
                registration("noop", &noop, &[]),
                registration("bad", &no_symbol, &[]),
            ],
            Duration::from_secs(10),
        )
        .await
        .unwrap();
    let errors = outcome.err().expect("generation should fail");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].handler_id, "bad");
    assert!(
        errors[0].message.contains("chord_native_run_v1"),
        "{}",
        errors[0].message
    );
}

#[tokio::test]
async fn libraries_outside_the_cache_dir_are_rejected() {
    let noop = require_swift!(Fixture::swift("noop"));
    let outside = std::env::temp_dir().join("chord-native-outside.dylib");
    std::fs::copy(&noop, &outside).unwrap();
    let mut host = spawn_host(None).await;
    let errors = host
        .load_generation(
            1,
            vec![registration("outside", &outside, &[])],
            Duration::from_secs(10),
        )
        .await
        .unwrap()
        .err()
        .expect("should fail");
    assert!(errors[0].message.contains("outside"), "{}", errors[0].message);
}

#[tokio::test]
async fn embedded_nul_in_event_argument_is_invalid() {
    let noop = require_swift!(Fixture::swift("noop"));
    let mut host = spawn_host(None).await;
    load(&mut host, vec![registration("noop", &noop, &[])]).await;
    match invoke(&mut host, "noop", &["a\0b"]).await.unwrap() {
        InvocationResult::InvalidArguments { message } => {
            assert!(message.contains("NUL"), "{message}")
        }
        other => panic!("expected InvalidArguments, got {other:?}"),
    }
}

#[tokio::test]
async fn hot_path_latency_smoke() {
    let noop = require_swift!(Fixture::swift("noop"));
    let mut host = spawn_host(None).await;
    load(&mut host, vec![registration("noop", &noop, &[])]).await;
    for _ in 0..200 {
        invoke(&mut host, "noop", &[]).await.unwrap();
    }
    let mut samples = Vec::with_capacity(2000);
    for _ in 0..2000 {
        let outcome = host
            .invoke(
                "noop",
                vec![],
                1,
                InvocationContext::default(),
                Duration::from_secs(5),
            )
            .await
            .unwrap();
        samples.push(outcome.round_trip);
    }
    samples.sort();
    let pct = |p: f64| samples[((samples.len() as f64 * p) as usize).min(samples.len() - 1)];
    eprintln!(
        "noop round trip (debug build): p50={:?} p95={:?} p99={:?}",
        pct(0.50),
        pct(0.95),
        pct(0.99)
    );
    assert!(
        pct(0.99) < Duration::from_millis(50),
        "p99 = {:?}",
        pct(0.99)
    );
}
