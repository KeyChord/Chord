#!/usr/bin/env bun

import { prepareDevSession } from "#dev-session";

const args = process.argv.slice(2);
const separator = args.indexOf("--");
const launcherArgs = separator < 0 ? args : args.slice(0, separator);
// Help and version must remain read-only, even when combined with --force.
const informational = launcherArgs.some((arg) => ["--help", "-h", "--version", "-V"].includes(arg));
const force = launcherArgs.includes("--force");
const mprocsArgs = [
  ...launcherArgs.filter((arg) => arg !== "--force"),
  ...(separator < 0 ? [] : args.slice(separator)),
];
const mprocsBin = `${import.meta.dir}/node_modules/.bin/mprocs`;
const serverPort = 1422;

if (!informational) {
  try {
    await prepareDevSession({ force, mprocsBin, serverPort, servicePorts: [1420, 1421] });
  } catch (error) {
    console.error(`[dev] ${error instanceof Error ? error.message : error}`);
    process.exit(1);
  }
}

const mprocs = Bun.spawn(
  [
    mprocsBin,
    "--config",
    `${import.meta.dir}/mprocs.yaml`,
    "--server",
    `127.0.0.1:${serverPort}`,
    ...mprocsArgs,
  ],
  {
    cwd: `${import.meta.dir}/../../../../../..`,
    stderr: "inherit",
    stdin: "inherit",
    stdout: "inherit",
  },
);

process.exitCode = await mprocs.exited;
