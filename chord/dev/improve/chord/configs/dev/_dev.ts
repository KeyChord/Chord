#!/usr/bin/env bun

const mprocs = Bun.spawn(
	[
		`${import.meta.dir}/node_modules/.bin/mprocs`,
		"--config",
		`${import.meta.dir}/mprocs.yaml`,
		...process.argv.slice(2),
	],
	{
		cwd: `${import.meta.dir}/../../../../../..`,
		stderr: "inherit",
		stdin: "inherit",
		stdout: "inherit",
	},
);

process.exitCode = await mprocs.exited;
