import { expect, test } from "bun:test";
import { renderToString } from "react-dom/server";
import { type OfferedUpdate, UpdateCardContent } from "#update-card-content";

const update = { version: "0.2.4" };
const render = (state: OfferedUpdate) =>
  renderToString(<UpdateCardContent update={state} onInstall={() => {}} onRestart={() => {}} />);

test("an available update names the version and offers Update", () => {
  const html = render({ status: "available", update });
  expect(html).toContain("Update available");
  expect(html).toContain("0.2.4");
  expect(html).toContain(">Update<");
  expect(html).not.toContain("Restart");
  expect(html).not.toContain('role="progressbar"');
});

test("a download with a known size shows its percentage", () => {
  const html = render({ status: "downloading", update: { ...update, percent: 42 } });
  expect(html).toContain("Downloading update…");
  expect(html).toContain("42%");
  expect(html).toContain('aria-valuenow="42"');
  expect(html).not.toContain(">Update<");
});

test("a download of unknown size is indeterminate", () => {
  // Rust serializes the missing percentage as null, which the generated type doesn't describe.
  const percent = null as unknown as undefined;
  const html = render({ status: "downloading", update: { ...update, percent } });
  expect(html).toContain('role="progressbar"');
  expect(html).not.toContain("aria-valuenow");
  expect(html).not.toContain("%<");
});

test("installing shows progress and no action", () => {
  const html = render({ status: "installing", update });
  expect(html).toContain("Installing update…");
  expect(html).toContain('role="progressbar"');
  expect(html).not.toContain("<button");
});

test("an installed update offers Restart", () => {
  const html = render({ status: "ready", update });
  expect(html).toContain("Update installed");
  expect(html).toContain(">Restart<");
  expect(html).not.toContain(">Update<");
});
