# Package crash isolation

> [!NOTE]
> This document is largely AI-written, so parts of it may not be fully accurate. It will be replaced with a hand-written version when I get the time.

Package imports, handler factories, actions, timers, and lifecycle callbacks run in
one supervised child process per installed package. Workers have separate process
groups, so a package job-control signal cannot target the GUI's process group. Native add-ons run in that same
child, whose Cocoa main loop services Swift's MainActor. The GUI does not execute
package JavaScript or load package native add-ons.

An abort, segmentation fault, process exit, disconnected IPC stream, or timeout
terminates that worker. A stable supervisor automatically creates a fresh VM and
restores registered handler factories in their original order, preserving the
handler IDs used by the compiled chords. Actions are never replayed. Recovery
resets JS/native state, timers, and sequence return values. Retries use exponential
backoff from 250 ms up to 30 seconds; a healthy minute resets the backoff. A reload
replaces the whole supervisor/VM, including native modules and transitive imports.
Cyclic, excessively deep, or excessive TOML import expansions are rejected before
compilation. GUI source reads are limited to 8 MiB per file and 32 MiB per package,
with at most 10,000 source entries; bin/ and target/ payloads are read only by workers.
Each worker has a bounded queue (64 requests) and IPC messages are limited to 1 MiB.
Setup/health requests have a 15-second deadline; actions and lifecycle callbacks
have a five-minute deadline to allow interactive native UI.

This is fault isolation, not a permissions sandbox: packages retain the user's
filesystem/process access. Intentionally hostile code and machine-wide resource
exhaustion are outside this guarantee.

Run the regression against a newly built executable:

    CARGO_INCREMENTAL=0 cargo build --manifest-path apps/chord-tauri/src-tauri/Cargo.toml --bin chord
    CHORD_PACKAGE_TEST_HOST="$PWD/apps/chord-tauri/src-tauri/target/debug/chord" \
      CARGO_INCREMENTAL=0 cargo test --manifest-path apps/chord-tauri/src-tauri/Cargo.toml \
      --lib package_faults_are_contained -- --ignored --nocapture

The regression deliberately crashes only child processes. It checks top-level
import aborts, factory exits, handler aborts, native SIGSEGV, process-group aborts, infinite loops,
unresolved promises, lifecycle aborts, timer aborts, ordinary exceptions,
cross-request JavaScript object identity, handler context, lifecycle unsubscribe,
package/native path resolution, and continued execution in a healthy worker.

The transport regression intentionally checks a single disposable process; the
supervisor recovery regression additionally verifies background replacement,
stable handler IDs, factory order, queued actions executing once, timeouts,
ordinary exceptions, cancellation, removed handlers, and shutdown without respawn:

    CHORD_PACKAGE_TEST_HOST="$PWD/apps/chord-tauri/src-tauri/target/debug/chord" \
      cargo test --manifest-path apps/chord-tauri/src-tauri/Cargo.toml \
      --lib supervisor_ -- --ignored --nocapture

The original NodeSwift 1.5.1 missing-queue abort has a separate regression.
Build its two deliberately unsafe fixtures against an existing NodeSwift 1.5.1
SwiftPM cache, such as the one `chords-menu` creates when it builds:

    bun apps/chord-tauri/src-tauri/tests/package-isolation/build-legacy-probes.ts \
      ../chords/@keychord/chords-menu/node_modules/.cache/chord/native/swiftpm
    CHORD_PACKAGE_TEST_HOST="$PWD/apps/chord-tauri/src-tauri/target/debug/chord" \
      CHORD_LEGACY_NATIVE_FIXTURE=/tmp/chord-legacy-node-swift-probes \
      CARGO_INCREMENTAL=0 cargo test --manifest-path apps/chord-tauri/src-tauri/Cargo.toml \
      --lib legacy_node_swift_fatal_error_is_contained -- --ignored --nocapture

## Performance comparison

On macOS, run the paired benchmark from the repository root:

    bun apps/chord-tauri/src-tauri/tests/package-isolation/benchmark.ts

It builds the host and test harness in release mode, then runs three fresh harness
processes. Each uses the production supervisor/hidden child CLI for the worker
side. The in-process side uses the previous architecture: the same shared Bun
thread, bounded queue, persistent VM, and a context acquisition for every call.
Both execute the same registration and handler functions. The in-process path is
test-only and is never available to packages in the GUI.

The suite records p50/p95/p99, throughput, raw timing samples, JSON/TOML codec
cost, fresh VM/import/factory startup, and idle CPU/RSS/physical footprint. Cases
cover keyboard events, object identity, resolved promises, a native Bun timer,
1 KiB/64 KiB arguments, repeats, a 32-request burst, parallel packages, cleanup,
and lifecycle callbacks. A busy-package case times a handler in another package
while the first runs a 20 ms CPU loop. Fixture assertions check that calls really
execute and preserve arguments/context/state.

Warmup is excluded, measurement order alternates between paths, and summaries
take the median across independent runs. The per-sample clone of fixture data is
excluded from sequential latency; IPC serialization, scheduling, execution, and
response delivery are included. Burst timings include task creation and queueing.
The worker retains its production scheduling priority and health checks.

Options: --runs 3, --samples 1000, --workers 4, --output /path/to/results.
CARGO_TARGET_DIR controls the Cargo cache; output defaults to its
package-worker-benchmarks/ directory. CHORD_BENCH_WARMUP (100) and
CHORD_BENCH_IDLE_MS (3000) configure warmup and idle observation time. Expensive
cases use at most 200 samples, burst cases at most 100. Each run writes raw JSON;
summary.json records hardware, OS/toolchain, paired medians and resource samples.

Run on an otherwise quiet machine and compare release builds. These are handler
runtime benchmarks, not full keyboard-to-screen timings: they exclude keyboard
capture/matching, GUI host RPC, and native UI rendering. Fresh startup keeps the
OS filesystem/dyld caches warm; it is not a reboot-cold launch. RSS includes shared
library pages, so summing worker RSS overstates unique RAM; physical footprint is
recorded separately. The parent-and-workers measurement also contains the idle
in-process baseline VM. Timing thresholds are intentionally not unit-test gates
because scheduling, background work, and hardware affect the results.

### Reference measurement

Measured on 2026-10-12: Apple M2 Max, 64 GB RAM, macOS 26.5.2, Rust 1.98,
embedded Bun 1.4.2. Three release runs, 1000 samples (200 for expensive cases),
100 warmup calls, four workers. [reference-results.json](./reference-results.json)
contains the medians, tail latencies, throughput and resource counters.

| Case | In-process p50 | Worker p50 | Worker p95 |
| --- | ---: | ---: | ---: |
| No keyboard event | 10.3 µs | 36.7 µs | 72.6 µs |
| Keyboard event | 21.8 µs | 49.5 µs | 79.6 µs |
| Resolved promise | 22.8 µs | 54.1 µs | 99.0 µs |
| 1 KiB arguments | 24.2 µs | 53.5 µs | 93.8 µs |
| 64 KiB arguments | 107.2 µs | 180.9 µs | 221.9 µs |
| 1 ms Bun timer | 1162.0 µs | 1252.7 µs | 1398.3 µs |
| Another package doing 20 ms CPU work | 16719.9 µs | 186.2 µs | 257.2 µs |

The benchmark identified TOML serialization/parsing on every invocation as a
large-payload bottleneck. Ordinary values now use structured JSON; dates and
non-finite values retain a TOML fallback. JSON uses exact float round trips, and
framing writes the header and payload together. Before these changes, the 64 KiB
worker case was 508.9 µs and its codec alone was 324.3 µs. Afterward those were
180.9 µs and 37.3 µs. Both paths still preserve arguments and JS object identity.

A 32-request burst reached about 52k actions/s in-process and 21k in one worker;
four workers reached about 44k actions/s. Process isolation costs roughly 36 MiB
of physical footprint per fixture worker, versus 25 MiB for the shared baseline
VM process. Over three-second idle windows, the parent plus four workers used a
median 1.30% of one CPU core, versus 0.0065% for the baseline process. These
resource figures exclude a full GUI and real packages' additional allocations.

Fresh worker startup/import/factory took a median 112 ms per worker. The first
in-process VM/import/factory observation varied across runs (median 193 ms).
The baseline VM initializes before the worker pool and OS caches remain warm;
these startup observations do not establish that worker startup is faster or
measure complete app startup. The shared VM initializes once, whereas each
package worker initializes separately.
