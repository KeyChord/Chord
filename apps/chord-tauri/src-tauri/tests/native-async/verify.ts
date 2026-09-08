import assert from "node:assert/strict";

const addon = { exports: {} as { probe(fail: boolean): Promise<string> } };
process.dlopen(addon, process.env.CHORD_ASYNC_ADDON!);

export async function verify() {
  assert.equal(await addon.exports.probe(false), "main-thread-ok");
  await assert.rejects(addon.exports.probe(true));
  assert.equal(await addon.exports.probe(false), "main-thread-ok");
  console.log("native-async-ok");
}

export async function fail() {
  await addon.exports.probe(true);
}

if (process.env.CHORD_ASYNC_TOP_LEVEL === "1") {
  await verify();
} else if (process.env.CHORD_ASYNC_TOP_LEVEL === "fail") {
  await fail();
}
