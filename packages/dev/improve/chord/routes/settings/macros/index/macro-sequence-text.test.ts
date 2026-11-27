import { expect, test } from "bun:test";
import { sequenceLines, sequenceToText, textToSequence } from "#macro-sequence-text";

test("line breaks in the editor are Enter key presses in the stored sequence", () => {
  for (const [sequence, text] of [
    ["", ""],
    ["1720x864", "1720x864"],
    ["ab\\ncd", "ab\ncd"],
    ["3`d2,i5`ab\\ncd\\n", "3`d2,i5`ab\ncd\n"],
    ["\\n", "\n"],
    ["\\n\\n", "\n\n"],
  ]) {
    expect(sequenceToText(sequence)).toBe(text);
    expect(textToSequence(text)).toBe(sequence);
  }
});

test("other escapes and references are untouched", () => {
  const sequence = "\\up\\left\\down\\right\\1\\=name=\\=n=\\size=\\n'a\\nb'";
  expect(sequenceToText(sequence)).toBe("\\up\\left\\down\\right\\1\\=name=\\=n=\\size=\n'a\nb'");
  expect(textToSequence(sequenceToText(sequence))).toBe(sequence);
});

test("typed \\n notation in the editor is the same Enter key", () => {
  expect(textToSequence("ab\\ncd\nef")).toBe("ab\\ncd\\nef");
});

test("a trailing Enter leaves an empty last line", () => {
  expect(sequenceLines("ab\\ncd\\n")).toEqual(["ab", "cd", ""]);
  expect(sequenceLines("\\n")).toEqual(["", ""]);
  expect(sequenceLines("abc")).toEqual(["abc"]);
});
