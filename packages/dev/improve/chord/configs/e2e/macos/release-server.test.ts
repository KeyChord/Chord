import { expect, test } from "bun:test";
import { pointDownloadsAt } from "./release-server.ts";

const PUBLISHED = "https://github.com/KeyChord/Chord/releases/latest/download/Chord-aarch64.dmg";

test("download links point at the local DMG, in the page and at the moment of the click", () => {
  const html = pointDownloadsAt(`<head></head><a href="${PUBLISHED}">Download</a>`, "/local.dmg");
  expect(html).not.toContain(PUBLISHED);
  expect(html).toContain('<a href="/local.dmg">');
  expect(html).toContain('addEventListener("click"');
});

test("other links are left alone", () => {
  const html = pointDownloadsAt(
    '<head></head><a href="https://github.com/KeyChord/Chord">Source</a>',
    "/local.dmg",
  );
  expect(html).toContain('href="https://github.com/KeyChord/Chord"');
});

test("a page without a head still gets the click handler", () => {
  expect(pointDownloadsAt("<a>x</a>", "/local.dmg")).toContain("<script>");
});
