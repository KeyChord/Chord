import { type Guest, shellQuote } from "#guest";
import { waitFor } from "#wait";

export interface ChordApp {
  /** For example `/Applications/Chord.app`. */
  path: string;
  bundleId: string;
  /** The main executable. Package workers and the Caps guardian run it with arguments. */
  executable: string;
  /** tauri-plugin-log's file, `~/Library/Logs/<bundle ID>/chords.log`. */
  log: string;
}

const TCC_DB = "/Library/Application Support/com.apple.TCC/TCC.db";

/** Mounts `dmg` in the guest and copies its app to `/Applications`, like a user would. */
export async function installDmg(guest: Guest, dmg: string): Promise<ChordApp> {
  const remote = "/tmp/chord-e2e.dmg";
  await guest.copyTo(dmg, remote);
  const { stdout } = await guest.run(`
    mount=$(mktemp -d)
    hdiutil attach -nobrowse -readonly -noautoopen -mountpoint "$mount" ${remote} >/dev/null
    app=$(find "$mount" -maxdepth 1 -name '*.app' | head -n 1)
    if [ -z "$app" ]; then echo "no .app in the DMG" >&2; exit 1; fi
    target="/Applications/$(basename "$app")"
    rm -rf "$target"
    ditto "$app" "$target"
    hdiutil detach "$mount" >/dev/null
    rm ${remote}
    plist="$target/Contents/Info.plist"
    echo "$target"
    /usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$plist"
    /usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$plist"
  `);
  const [path, bundleId, executable] = stdout.trim().split("\n");
  return {
    path: path!,
    bundleId: bundleId!,
    executable: `${path}/Contents/MacOS/${executable}`,
    log: `${guest.home}/Library/Logs/${bundleId}/chords.log`,
  };
}

/**
 * Grants Accessibility as if the user had switched it on in System Settings: a TCC entry
 * for the bundle ID, pinned to the app's designated code requirement. Writing the system
 * TCC database needs SIP disabled, which the golden image does.
 */
export async function grantAccessibility(guest: Guest, app: ChordApp) {
  await guest.run(`
    if ! csrutil status | grep -q disabled; then
      echo "SIP is enabled in this VM, so its TCC database is read-only; rebuild the golden image" >&2
      exit 1
    fi
    requirement=$(codesign -d -r- ${shellQuote(app.path)} 2>&1 | sed -n 's/^designated => //p')
    if [ -z "$requirement" ]; then echo "cannot read the designated requirement of ${app.path}" >&2; exit 1; fi
    blob=$(mktemp)
    printf '%s' "$requirement" | csreq -r- -b "$blob"
    hex=$(xxd -p "$blob" | tr -d '\\n')
    rm "$blob"
    sudo sqlite3 ${shellQuote(TCC_DB)} "INSERT OR REPLACE INTO access
      (service, client, client_type, auth_value, auth_reason, auth_version, csreq, flags, last_modified)
      VALUES ('kTCCServiceAccessibility', '${app.bundleId}', 0, 2, 4, 1, X'$hex', 0, CAST(strftime('%s', 'now') AS INTEGER))"
  `);
}

/** Approves only the test fixtures, before launch, using Chord's normal persisted identities. */
export async function approvePackages(guest: Guest, app: ChordApp, roots: string[]) {
  const file = `${guest.home}/Library/Application Support/${app.bundleId}/package-trust.json`;
  const store = JSON.parse((await guest.readFile(file)) ?? "{}");
  const approvals = new Set<string>(store.approvedPackages ?? []);
  for (const root of roots) {
    const { stdout } = await guest.run(`cd ${shellQuote(root)} && /bin/pwd -P`);
    approvals.add(JSON.stringify([root, stdout.trim()]));
  }
  store.approvedPackages = [...approvals];
  await guest.writeFile(file, JSON.stringify(store, null, 2));
}

/** The main app process, which runs the executable without arguments. */
export async function appPid(guest: Guest, app: ChordApp): Promise<number | undefined> {
  // pgrep matches an extended regex; channel names such as "Chord (Beta)" contain parentheses.
  const pattern = app.executable.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const { stdout, exitCode } = await guest.run(`pgrep -fx ${shellQuote(pattern)}`, {
    check: false,
  });
  return exitCode === 0 ? Number(stdout.trim().split("\n")[0]) : undefined;
}

async function logSize(guest: Guest, app: ChordApp): Promise<number> {
  const { stdout } = await guest.run(`stat -f %z ${shellQuote(app.log)} 2>/dev/null || echo 0`);
  return Number(stdout.trim());
}

/** Waits for log lines written from now on. `start` runs once the log is being followed. */
export async function waitForLog(
  guest: Guest,
  app: ChordApp,
  until: RegExp[],
  start: () => Promise<unknown>,
  { timeoutMs = 90_000 } = {},
): Promise<string> {
  const offset = await logSize(guest, app);
  const output = guest.waitForOutput(
    `tail -c +${offset + 1} -F ${shellQuote(app.log)} 2>/dev/null`,
    {
      until,
      failOn: [/Accessibility is not granted/],
      timeoutMs,
    },
  );
  try {
    await start();
  } catch (error) {
    output.catch(() => {});
    throw error;
  }
  return await output;
}

/**
 * Launches the app through Launch Services and waits until its event tap runs and it
 * owns keyboard input.
 */
export async function launch(guest: Guest, app: ChordApp) {
  await waitForLog(guest, app, [/Keyboard event tap started/, /Input ownership: active/], () =>
    guest.run(`open ${shellQuote(app.path)}`),
  );
}

/**
 * Quits through the app's own `0q` chord, so it cleans up as it does for a user, and
 * kills it if that does not work; the Caps guardian then restores the Caps key.
 */
export async function quit(guest: Guest, app: ChordApp, { timeoutMs = 15_000 } = {}) {
  const pid = await appPid(guest, app);
  if (pid === undefined) return;
  await guest.run(`${shellQuote(app.executable)} chord 0q`, { check: false });
  try {
    await waitForExit(guest, pid, { timeoutMs });
  } catch {
    await guest.run(`kill -KILL ${pid}`, { check: false });
    await waitForExit(guest, pid);
  }
}

export async function waitForExit(guest: Guest, pid: number, { timeoutMs = 30_000 } = {}) {
  await waitFor(
    `process ${pid} to exit`,
    async () =>
      (await guest.run(`kill -0 ${pid}`, { check: false })).exitCode === 0 ? undefined : true,
    { timeoutMs },
  );
}
