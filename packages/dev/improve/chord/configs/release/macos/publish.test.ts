import { afterEach, expect, test } from "bun:test";
import { chmodSync, mkdirSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const SHA = "1234567890abcdef";
const workflow = Bun.YAML.parse(
  await Bun.file(
    new URL("../../../../../../../.github/workflows/release.yaml", import.meta.url),
  ).text(),
) as { jobs: { publish: { steps: { name?: string; run?: string }[] } } };
const script = workflow.jobs.publish.steps.find(
  (step) => step.name === "Publish versioned release",
)!.run!;
const directories: string[] = [];

afterEach(() => {
  for (const dir of directories.splice(0)) rmSync(dir, { recursive: true, force: true });
});

type Release = { isDraft: boolean; targetCommitish: string; assets: Record<string, string> };
type State = {
  published: string;
  status: number;
  failUpload: boolean;
  releases: Record<string, Release>;
  calls: { tool: string; args: string[] }[];
};

// Run the actual workflow shell against a local GitHub stand-in. It records uploads and
// refuses to modify published versioned releases, including on retries.
const fakeTool = `#!/usr/bin/env bun
import { basename } from "node:path";
const tool = basename(process.argv[1]);
const args = process.argv.slice(2);
const file = Bun.file("state.json");
const state = await file.json();
state.calls.push({tool, args});
await Bun.write(file, JSON.stringify(state));
const option = (name) => args[args.indexOf(name) + 1];
if (tool === "curl") {
  await Bun.write(option("-o"), JSON.stringify({version: state.published}));
  process.stdout.write(String(state.status));
} else if (tool === "sha256sum") {
  console.log(new Bun.CryptoHasher("sha256").update(await Bun.file(args[0]).arrayBuffer()).digest("hex"));
} else if (tool === "gh") {
  const [, action, tag] = args;
  let release = state.releases[tag];
  if (action === "view") {
    if (!release) process.exit(1);
    console.log(args.includes("--jq") ? Object.keys(release.assets).join("\\n") : JSON.stringify(release));
  } else if (action === "create") {
    if (release) process.exit(1);
    state.releases[tag] = {isDraft: args.includes("--draft"), targetCommitish: option("--target"), assets: {}};
  } else if (action === "upload") {
    if (state.failUpload || !release || (!release.isDraft && tag !== "beta")) process.exit(1);
    for (const asset of args.slice(3).filter(arg => !arg.startsWith("--"))) {
      release.assets[basename(asset)] = await Bun.file(asset).text();
    }
  } else if (action === "edit") {
    if (!release?.isDraft) process.exit(1);
    release.isDraft = false;
    if (args.includes("--latest")) state.published = JSON.parse(release.assets["latest.json"]).version;
  } else {
    throw new Error("Unexpected gh action: " + args.join(" "));
  }
  await Bun.write(file, JSON.stringify(state));
} else {
  throw new Error("Unexpected tool: " + tool);
}
`;

async function fixture(channel = "prod", overrides: Partial<State> = {}) {
  const dir = mkdtempSync(join(tmpdir(), "chord-publish-test-"));
  directories.push(dir);
  mkdirSync(join(dir, "bin"));
  mkdirSync(join(dir, "apps/chord-tauri/src-tauri"), { recursive: true });
  mkdirSync(join(dir, "assets"));
  for (const tool of ["gh", "curl", "sha256sum", "git"]) {
    const file = join(dir, "bin", tool);
    await Bun.write(file, fakeTool);
    chmodSync(file, 0o755);
  }
  const oldRelease = {
    isDraft: false,
    targetCommitish: "old",
    assets: { "Chord-aarch64.app.tar.gz": "old signed bytes" },
  };
  await Bun.write(
    join(dir, "state.json"),
    JSON.stringify({
      published: "0.2.9",
      status: 200,
      failUpload: false,
      releases: { prod: oldRelease, beta: oldRelease },
      calls: [],
      ...overrides,
    }),
  );
  async function stage(version: string) {
    await Bun.write(
      join(dir, "apps/chord-tauri/src-tauri/tauri.conf.json"),
      JSON.stringify({ version }),
    );
    await Bun.write(join(dir, "assets/gatekeeper.txt"), "notarized\n");
    for (const arch of ["aarch64", "x86_64"]) {
      for (const [suffix, contents] of [
        ["dmg", "disk"],
        ["app.tar.gz", "signed archive"],
        ["app.tar.gz.sig", "signature"],
      ]) {
        await Bun.write(
          join(dir, `assets/Chord-${arch}.${suffix}`),
          `${version} ${arch} ${contents}`,
        );
      }
    }
    for (const [platform, installer] of [
      ["linux-x86_64", "Chord-linux-x86_64.AppImage"],
      ["windows-x86_64", "Chord-windows-x86_64-setup.exe"],
    ]) {
      await Bun.write(join(dir, "assets", installer), `${version} ${platform} signed archive`);
      await Bun.write(join(dir, "assets", `${installer}.sig`), `${version} ${platform} signature\n`);
    }
  }
  await stage("0.2.10");
  return {
    stage,
    state: () => Bun.file(join(dir, "state.json")).json() as Promise<State>,
    async run() {
      // CI uses modern Bash; macOS's system Bash predates associative arrays.
      const bash =
        Bun.which("bash", {
          PATH: `/opt/homebrew/bin:/usr/local/bin:${process.env.PATH}`,
        }) ?? "bash";
      const child = Bun.spawn([bash, "-c", script], {
        cwd: dir,
        env: {
          ...process.env,
          PATH: `${dir}/bin:${process.env.PATH}`,
          RELEASE_CHANNEL: channel,
          GITHUB_SHA: SHA,
        },
        stdout: "pipe",
        stderr: "pipe",
      });
      const [code, stdout, stderr] = await Promise.all([
        child.exited,
        new Response(child.stdout).text(),
        new Response(child.stderr).text(),
      ]);
      return { code, output: stdout + stderr };
    },
  };
}

test("successive production versions create distinct releases and keep earlier downloads", async () => {
  const f = await fixture();
  expect(await f.run()).toMatchObject({ code: 0 });
  const first = await f.state();
  const release = first.releases["v0.2.10"]!;
  expect(release.isDraft).toBe(false);
  expect(release.targetCommitish).toBe(SHA);
  const manifest = JSON.parse(release.assets["latest.json"]!);
  expect(manifest.version).toBe("0.2.10");
  expect(Object.keys(manifest.platforms).sort()).toEqual([
    "darwin-aarch64",
    "darwin-x86_64",
    "linux-x86_64",
    "windows-x86_64",
  ]);
  for (const [platform, asset] of Object.entries(manifest.platforms) as [
    string,
    { url: string; signature: string },
  ][]) {
    const name = asset.url.split("/").at(-1)!;
    expect(asset.url).toContain("/releases/download/v0.2.10/");
    expect(release.assets[name]).toBe(`0.2.10 ${platform.replace("darwin-", "")} signed archive`);
    expect(asset.signature).toContain("signature");
  }
  const mutations = first.calls.filter(
    (call) => call.tool === "gh" && ["create", "upload", "edit"].includes(call.args[1]!),
  );
  expect(mutations.map((call) => call.args[1])).toEqual(["create", "upload", "edit"]);
  expect(mutations[0]!.args).toContain("--draft");
  expect(mutations[0]!.args).toContain("Chord 0.2.10");
  expect(mutations[2]!.args).toContain("--latest");
  expect(first.calls.find((call) => call.tool === "curl")!.args.at(-1)).toContain(
    "/releases/latest/download/latest.json",
  );
  await f.stage("0.2.11");
  expect(await f.run()).toMatchObject({ code: 0 });
  const second = await f.state();
  expect(second.releases["v0.2.10"]).toEqual(release);
  expect(second.releases["v0.2.11"]!.isDraft).toBe(false);
  expect(second.releases.prod).toEqual(first.releases.prod);
  expect(second.calls.some((call) => call.tool === "git")).toBe(false);
});

test("beta publishes a versioned prerelease and preserves the installed beta updater endpoint", async () => {
  const f = await fixture("beta");
  expect(await f.run()).toMatchObject({ code: 0 });
  const { releases, calls } = await f.state();
  expect(releases["v0.2.10-beta"]!.isDraft).toBe(false);
  expect(releases.beta!.assets["latest.json"]).toBe(
    releases["v0.2.10-beta"]!.assets["latest.json"],
  );
  expect(releases.beta!.assets["Chord-aarch64.app.tar.gz"]).toBe("old signed bytes");
  const publish = calls.find((call) => call.args[1] === "edit")!;
  expect(publish.args).toContain("--prerelease");
  expect(publish.args).toContain("--latest=false");
});

for (const channel of ["prod", "beta"]) {
  for (const published of ["0.2.10", "0.2.11"]) {
    test(`${channel} refuses a version that isn't above ${published}`, async () => {
      const f = await fixture(channel, { published });
      expect((await f.run()).code).not.toBe(0);
      expect((await f.state()).calls.every((call) => call.tool !== "gh")).toBe(true);
    });
  }
}

test("published tags and drafts from other commits cannot be overwritten", async () => {
  for (const [isDraft, targetCommitish] of [
    [false, SHA],
    [true, "another-commit"],
  ] as const) {
    const f = await fixture("prod", {
      releases: { "v0.2.10": { isDraft, targetCommitish, assets: {} } },
    });
    expect((await f.run()).code).not.toBe(0);
    expect(
      (await f.state()).calls.filter((call) => call.tool === "gh").map((call) => call.args[1]),
    ).toEqual(["view"]);
  }
});

test("an upload failure leaves the release unpublished", async () => {
  const f = await fixture("prod", { failUpload: true });
  expect((await f.run()).code).not.toBe(0);
  expect((await f.state()).releases["v0.2.10"]!.isDraft).toBe(true);
});

test("a draft from the same commit can be resumed", async () => {
  const f = await fixture("prod", {
    releases: { "v0.2.10": { isDraft: true, targetCommitish: SHA, assets: {} } },
  });
  expect(await f.run()).toMatchObject({ code: 0 });
  const state = await f.state();
  expect(state.releases["v0.2.10"]!.isDraft).toBe(false);
  expect(state.calls.some((call) => call.args[1] === "create")).toBe(false);
});

test("an unavailable updater endpoint fails without publishing", async () => {
  const f = await fixture("prod", { status: 503 });
  expect((await f.run()).code).not.toBe(0);
  expect((await f.state()).calls.every((call) => call.tool !== "gh")).toBe(true);
});

for (const channel of ["prod", "beta"]) {
  test(`the first ${channel} release works without an existing updater endpoint`, async () => {
    const f = await fixture(channel, { status: 404, releases: {} });
    expect(await f.run()).toMatchObject({ code: 0 });
    if (channel === "beta") {
      const { releases } = await f.state();
      expect(releases.beta!.assets["latest.json"]).toBe(
        releases["v0.2.10-beta"]!.assets["latest.json"],
      );
    }
  });
}
