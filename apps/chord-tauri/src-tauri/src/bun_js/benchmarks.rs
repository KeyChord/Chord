//! Opt-in, paired benchmarks of the production supervisor and the old in-process
//! Bun thread/context handoff. Never expose the in-process path in the GUI.

use super::*;
use crate::bun_js::{execute_package_request, with_test_js};
use crate::models::{ChordKeyboardEvent, HandlerChordAction, Key, KeyEvent};
use keycode::KeyMappingCode;
use serde::Serialize;
use serde_json::{Value, json};
use std::hint::black_box;
use std::time::Instant;

const SOURCE: &str = r#"
import { onAppLaunch } from "chord";
export default function() {
    const state = { calls: 0, launches: 0 };
    state.self = state;
    onAppLaunch("dev.benchmark", () => { state.launches++; });
    return function(mode, payload, length) {
        if (mode === "verify") {
            if (state.calls !== payload) throw new Error(`expected ${payload} calls, got ${state.calls}`);
            state.calls = 0;
            return;
        }
        if (this.focusedAppId !== "dev.benchmark") throw new Error("lost handler context");
        if (this.event && (!(this.event instanceof Event) || this.event.key !== "Backspace"))
            throw new Error("lost keyboard event");
        state.calls++;
        if (mode === "args" && payload.length !== length) throw new Error("lost payload");
        if (mode === "state" && (payload !== state || payload.self !== state))
            throw new Error("lost object identity");
        if (mode === "promise") return Promise.resolve(state);
        if (mode === "timer") return Bun.sleep(1).then(() => state);
        if (mode === "burn") {
            const end = performance.now() + 20;
            while (performance.now() < end) state.calls |= 0;
        }
        return state;
    };
}
"#;

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("chord-benchmark-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("package.json"), r#"{"name":"benchmark-fixture"}"#).unwrap();
        std::fs::write(root.join("handler.js"), SOURCE).unwrap();
        Self(root)
    }

    fn registration(&self) -> Request {
        Request::Register {
            module: self.0.join("handler.js").display().to_string(),
            raw: json!({"name":"benchmark"}),
            pathslug: "benchmark.toml".into(),
            app_id: Some("dev.benchmark".into()),
            args: vec![],
        }
    }

    async fn worker(&self, executable: &Path) -> Arc<PackageWorker> {
        PackageWorker::spawn(
            executable,
            "benchmark-fixture".into(),
            WorkerConfig {
                root: self.0.clone(),
                cwd: self.0.clone(),
            },
            None,
            ACTION_TIMEOUT,
            Arc::default(),
        )
        .await
        .unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[derive(Clone)]
enum Engine {
    InProcess(crate::bun_js::PackageContext),
    Worker(Arc<PackageWorker>),
}

impl Engine {
    async fn request(&self, request: Request) -> Value {
        match self {
            Self::InProcess(package) => {
                let package = package.clone();
                with_test_js(move |ctx| {
                    Box::pin(async move {
                        if matches!(request, Request::Register { .. }) {
                            // The fixture uses lifecycle registration, which requires package
                            // context. No GUI host RPC is performed in this benchmark.
                            ctx.store_userdata(crate::bun_js::AppUserData {
                                package: Some(package),
                            })?;
                        }
                        execute_package_request(ctx, request).await
                    })
                })
                .await
            }
            Self::Worker(worker) => worker.request(request).await,
        }
        .unwrap()
    }
}

fn action(id: &str, mode: &str, keyboard: bool, args: Vec<toml::Value>, count: u32) -> Request {
    let mut event_args = vec![mode.into()];
    event_args.extend(args);
    Request::Invoke {
        action: HandlerChordAction {
            handler_id: id.into(),
            event_args,
            input: "j,12".into(),
            sequence_id: Some("benchmark-interval".into()),
            assign_to: Some("state".into()),
            keyboard_event: keyboard.then(|| {
                ChordKeyboardEvent::new(
                    &KeyEvent::Press(Key(KeyMappingCode::Backspace)),
                    &[Key(KeyMappingCode::ShiftLeft)],
                    false,
                )
            }),
            focused_window_id: None,
        },
        app_id: Some("dev.benchmark".into()),
        count,
    }
}

fn verify(id: &str, calls: usize) -> Request {
    let mut request = action(id, "verify", false, vec![(calls as i64).into()], 1);
    if let Request::Invoke { action, .. } = &mut request {
        action.assign_to = None;
    }
    request
}

#[derive(Serialize)]
struct Distribution {
    samples: usize,
    mean_us: f64,
    p50_us: f64,
    p95_us: f64,
    p99_us: f64,
    max_us: f64,
    operations_per_second: f64,
    // Keep raw observations so tail latency and outliers can be reviewed.
    raw_us: Vec<f64>,
}

fn distribution(mut raw_us: Vec<f64>, operations_per_sample: usize) -> Distribution {
    assert!(!raw_us.is_empty());
    raw_us.sort_by(f64::total_cmp);
    let percentile = |p: f64| raw_us[((raw_us.len() as f64 * p).ceil() as usize).saturating_sub(1)];
    let mean_us = raw_us.iter().sum::<f64>() / raw_us.len() as f64;
    Distribution {
        samples: raw_us.len(),
        mean_us,
        p50_us: percentile(0.50),
        p95_us: percentile(0.95),
        p99_us: percentile(0.99),
        max_us: *raw_us.last().unwrap(),
        operations_per_second: operations_per_sample as f64 * 1_000_000.0 / mean_us,
        raw_us,
    }
}

async fn timed(engine: &Engine, request: &Request) -> f64 {
    // Cloning fixture data is setup, not part of execution on either path.
    let request = request.clone();
    let start = Instant::now();
    black_box(engine.request(request).await);
    start.elapsed().as_secs_f64() * 1_000_000.0
}

async fn paired(
    name: &str,
    engines: &[Engine; 2],
    requests: &[Request; 2],
    samples: usize,
    warmup: usize,
    operations: usize,
) -> Value {
    for _ in 0..warmup {
        for i in 0..2 {
            engines[i].request(requests[i].clone()).await;
        }
    }
    let mut times = [Vec::with_capacity(samples), Vec::with_capacity(samples)];
    // Alternate order on every pair to reduce thermal/scheduler/order bias.
    for sample in 0..samples {
        for i in [sample % 2, 1 - sample % 2] {
            times[i].push(timed(&engines[i], &requests[i]).await);
        }
    }
    let [direct, worker] = times.map(|times| distribution(times, operations));
    println!(
        "{name:28} in-process {:9.2} us  worker {:9.2} us  delta {:+9.2} us (p50)",
        direct.p50_us,
        worker.p50_us,
        worker.p50_us - direct.p50_us
    );
    json!({"name":name, "operations_per_sample":operations,
        "in_process":direct, "worker":worker})
}

async fn batch(engines: &[Engine], requests: &[Request], width: usize) -> f64 {
    let start = Instant::now();
    let mut jobs = tokio::task::JoinSet::new();
    for index in 0..width {
        let engine = engines[index % engines.len()].clone();
        let request = requests[index % requests.len()].clone();
        jobs.spawn(async move { engine.request(request).await });
    }
    while let Some(result) = jobs.join_next().await {
        black_box(result.unwrap());
    }
    start.elapsed().as_secs_f64() * 1_000_000.0
}

#[derive(Clone, Serialize)]
struct Usage {
    cpu_ns: u64,
    rss_bytes: u64,
    footprint_bytes: u64,
}

#[allow(deprecated)] // libc's Mach bindings suffice for this test-only sampler.
fn usage(pid: u32) -> Usage {
    let mut info: libc::rusage_info_v2 = unsafe { std::mem::zeroed() };
    let status = unsafe {
        libc::proc_pid_rusage(
            pid as i32,
            libc::RUSAGE_INFO_V2,
            (&mut info as *mut libc::rusage_info_v2).cast(),
        )
    };
    assert_eq!(status, 0, "failed to sample process {pid}");
    // XNU fill_task_rusage reports CPU time in Mach absolute ticks, not ns.
    // https://github.com/apple-oss-distributions/xnu/blob/main/osfmk/kern/bsd_kern.c
    let mut timebase: libc::mach_timebase_info = unsafe { std::mem::zeroed() };
    assert_eq!(unsafe { libc::mach_timebase_info(&mut timebase) }, 0);
    let cpu_ns = ((info.ri_user_time as u128 + info.ri_system_time as u128)
        * timebase.numer as u128
        / timebase.denom as u128) as u64;
    Usage {
        cpu_ns,
        rss_bytes: info.ri_resident_size,
        footprint_bytes: info.ri_phys_footprint,
    }
}

async fn idle(pids: &[u32], duration: Duration) -> Value {
    let before: Vec<_> = pids.iter().map(|&pid| usage(pid)).collect();
    let start = Instant::now();
    tokio::time::sleep(duration).await;
    let seconds = start.elapsed().as_secs_f64();
    let after: Vec<_> = pids.iter().map(|&pid| usage(pid)).collect();
    let cpu_ms: f64 = before
        .iter()
        .zip(&after)
        .map(|(before, after)| (after.cpu_ns - before.cpu_ns) as f64 / 1_000_000.0)
        .sum();
    json!({"wall_seconds":seconds, "cpu_ms":cpu_ms,
        "cpu_percent_one_core":cpu_ms / (seconds * 10.0),
        "rss_bytes":after.iter().map(|u| u.rss_bytes).sum::<u64>(),
        "footprint_bytes":after.iter().map(|u| u.footprint_bytes).sum::<u64>(),
        "process_cpu_ms":before.iter().zip(&after).map(|(before, after)|
            (after.cpu_ns - before.cpu_ns) as f64 / 1_000_000.0).collect::<Vec<_>>(),
        "processes":after})
}

fn setting(name: &str, default: usize, min: usize, max: usize) -> usize {
    let value = std::env::var(name)
        .map(|s| s.parse::<usize>().unwrap())
        .unwrap_or(default);
    assert!((min..=max).contains(&value), "{name} must be {min}..={max}");
    value
}

/// Run alone in an optimized test harness; never run this alongside fault tests.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "opt-in benchmark; see tests/package-isolation/README.md"]
async fn package_worker_benchmarks() {
    let executable = PathBuf::from(std::env::var_os("CHORD_PACKAGE_TEST_HOST").unwrap());
    let output = PathBuf::from(std::env::var_os("CHORD_BENCH_OUTPUT").unwrap());
    let samples = setting("CHORD_BENCH_SAMPLES", 1000, 20, 100_000);
    let warmup = setting("CHORD_BENCH_WARMUP", 100, 1, 10_000);
    let pool_size = setting("CHORD_BENCH_WORKERS", 4, 2, 32);
    let idle_duration =
        Duration::from_millis(setting("CHORD_BENCH_IDLE_MS", 3000, 1000, 60_000) as u64);
    let baseline_fixture = Fixture::new();
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let socket = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (unused_host, _) = listener.accept().unwrap();
    drop(unused_host);
    drop(listener);
    let direct = Engine::InProcess(crate::bun_js::PackageContext {
        config: WorkerConfig {
            root: baseline_fixture.0.clone(),
            cwd: baseline_fixture.0.clone(),
        },
        host: Arc::new(HostClient(Mutex::new(socket))),
    });
    let before_vm = usage(std::process::id());
    let start = Instant::now();
    let direct_id = direct
        .request(baseline_fixture.registration())
        .await
        .as_str()
        .unwrap()
        .to_string();
    let direct_cold_us = start.elapsed().as_secs_f64() * 1_000_000.0;
    let direct_idle = idle(&[std::process::id()], idle_duration).await;
    let after_vm = usage(std::process::id());

    let mut fixtures = Vec::new();
    let mut pool = Vec::new();
    let mut ids = Vec::new();
    let mut startup = Vec::new();
    for _ in 0..pool_size {
        let fixture = Fixture::new();
        let start = Instant::now();
        let worker = fixture.worker(&executable).await;
        let engine = Engine::Worker(worker);
        let id = engine
            .request(fixture.registration())
            .await
            .as_str()
            .unwrap()
            .to_string();
        startup.push(start.elapsed().as_secs_f64() * 1_000_000.0);
        fixtures.push(fixture);
        pool.push(engine);
        ids.push(id);
    }
    let engines = [direct.clone(), pool[0].clone()];
    let pair_ids = [&direct_id, &ids[0]];
    let mut results = Vec::new();
    for (name, mode, keyboard, args, limited, count) in [
        ("noop", "noop", false, vec![], false, 1),
        ("keyboard_event", "noop", true, vec![], false, 1),
        ("resolved_promise", "promise", true, vec![], false, 1),
        (
            "sequence_object_identity",
            "state",
            true,
            vec!["$state".into()],
            false,
            1,
        ),
        (
            "arguments_1KiB",
            "args",
            true,
            vec!["x".repeat(1024).into(), 1024.into()],
            false,
            1,
        ),
        (
            "arguments_64KiB",
            "args",
            true,
            vec!["x".repeat(65536).into(), 65536.into()],
            true,
            1,
        ),
        ("native_timer_1ms", "timer", true, vec![], true, 1),
        ("repeat_32", "noop", true, vec![], true, 32),
    ] {
        // Prime sequence state, then reset the call counter without deleting its identity.
        for i in 0..2 {
            engines[i]
                .request(action(pair_ids[i], "noop", false, vec![], 1))
                .await;
            engines[i].request(verify(pair_ids[i], 1)).await;
        }
        let n = if limited { samples.min(200) } else { samples };
        let requests = [0, 1].map(|i| action(pair_ids[i], mode, keyboard, args.clone(), count));
        results.push(paired(name, &engines, &requests, n, warmup, count as usize).await);
        for i in 0..2 {
            engines[i]
                .request(verify(pair_ids[i], (n + warmup) * count as usize))
                .await;
        }
    }

    for (name, request) in [
        (
            "release_sequence",
            Request::ReleaseSequences(vec!["expired-interval".into()]),
        ),
        (
            "lifecycle_callback",
            Request::Lifecycle {
                app: crate::app::desktop_app::ObservedApp {
                    pid: 123,
                    bundle_id: "dev.benchmark".into(),
                },
                launch: true,
            },
        ),
    ] {
        results.push(
            paired(
                name,
                &engines,
                &[request.clone(), request],
                samples,
                warmup,
                1,
            )
            .await,
        );
    }

    // Measure encode+decode independently to identify transport versus serialization cost.
    let mut codec = Vec::new();
    for bytes in [0, 1024, 65536] {
        let request = action(
            &direct_id,
            "args",
            true,
            vec!["x".repeat(bytes).into(), (bytes as i64).into()],
            1,
        );
        let mut times = Vec::new();
        let n = if bytes > 1024 {
            samples.min(200)
        } else {
            samples
        };
        for index in 0..(n + warmup) {
            let start = Instant::now();
            let mut frame = Vec::new();
            write_frame(&mut frame, &request).unwrap();
            black_box(read_frame::<Request>(&mut frame.as_slice()).unwrap());
            if index >= warmup {
                times.push(start.elapsed().as_secs_f64() * 1_000_000.0);
            }
        }
        codec.push(json!({"payload_bytes":bytes, "roundtrip":distribution(times, 1)}));
    }

    let batch_samples = samples.min(100);
    for (name, workers, width) in [
        ("burst_32_single_package", 1, 32),
        ("parallel_packages", pool_size, pool_size * 8),
    ] {
        let requests: Vec<_> = ids[..workers]
            .iter()
            .map(|id| action(id, "noop", true, vec![], 1))
            .collect();
        let direct_requests = vec![action(&direct_id, "noop", true, vec![], 1)];
        let mut times = [Vec::new(), Vec::new()];
        for index in 0..batch_samples + 10 {
            for i in [index % 2, 1 - index % 2] {
                let time = if i == 0 {
                    batch(&[direct.clone()], &direct_requests, width).await
                } else {
                    batch(&pool[..workers], &requests, width).await
                };
                if index >= 10 {
                    times[i].push(time);
                }
            }
        }
        let [in_process, worker] = times.map(|t| distribution(t, width));
        println!(
            "{name:28} in-process {:9.0} ops/s  worker {:9.0} ops/s",
            in_process.operations_per_second, worker.operations_per_second
        );
        results.push(json!({"name":name, "operations_per_sample":width, "in_process":in_process, "worker":worker}));
        direct
            .request(verify(&direct_id, (batch_samples + 10) * width))
            .await;
        for index in 0..workers {
            pool[index]
                .request(verify(&ids[index], (batch_samples + 10) * width / workers))
                .await;
        }
    }

    // A 20ms CPU-bound package blocks every package on the old shared Bun thread.
    let mut contention = [Vec::new(), Vec::new()];
    for index in 0..30 {
        for i in [index % 2, 1 - index % 2] {
            let slow_engine = engines[i].clone();
            let slow = action(pair_ids[i], "burn", false, vec![], 1);
            let job = tokio::spawn(async move { slow_engine.request(slow).await });
            tokio::time::sleep(Duration::from_millis(2)).await;
            let fast_engine = if i == 0 { &direct } else { &pool[1] };
            let fast_id = if i == 0 { &direct_id } else { &ids[1] };
            contention[i].push(timed(fast_engine, &action(fast_id, "noop", true, vec![], 1)).await);
            job.await.unwrap();
        }
    }
    let [in_process, worker] = contention.map(|t| distribution(t, 1));
    println!(
        "other_package_busy_20ms       in-process {:9.2} us  worker {:9.2} us (p50)",
        in_process.p50_us, worker.p50_us
    );
    results
        .push(json!({"name":"other_package_busy_20ms", "in_process":in_process, "worker":worker}));
    let pids: Vec<_> = pool
        .iter()
        .map(|engine| match engine {
            Engine::Worker(worker) => worker.process.child.lock().id(),
            _ => unreachable!(),
        })
        .collect();
    // Include parent supervision overhead separately from child CPU/RSS.
    let mut all_pids = vec![std::process::id()];
    all_pids.extend(&pids);
    let isolated_idle = idle(&all_pids, idle_duration).await;
    let embedded_bun_version =
        with_test_js(|ctx| Box::pin(async move { Ok(ctx.eval::<String, _>("Bun.version")?) }))
            .await
            .unwrap();
    let report = json!({
        "schema_version":1, "debug_assertions":cfg!(debug_assertions),
        "embedded_bun_version":embedded_bun_version,
        "arch":std::env::consts::ARCH, "os":std::env::consts::OS,
        "worker_executable":executable, "samples":samples, "warmup":warmup, "workers":pool_size,
        "cold": {"in_process_first_vm_import_factory_us":direct_cold_us,
            "worker_process_vm_import_factory":distribution(startup, 1)},
        "idle": {"in_process":direct_idle, "parent_and_workers":isolated_idle,
            "in_process_vm_incremental_rss_bytes":after_vm.rss_bytes.saturating_sub(before_vm.rss_bytes)},
        "cases":results, "codec":codec,
    });
    std::fs::write(&output, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    println!("Benchmark data: {}", output.display());
    for engine in pool {
        if let Engine::Worker(worker) = engine {
            worker.stop();
        }
    }
    with_test_js(|_ctx| {
        Box::pin(async {
            crate::bun_js::lifecycle::clear_callbacks();
            Ok(())
        })
    })
    .await
    .unwrap();
}
