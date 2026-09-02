import assert from "node:assert/strict";
import test from "node:test";
import {
  DEFAULT_READING_PREFERENCES,
  loadReadingPreferences,
  readingPreferencesStorageKey,
  resolveReadingParagraphSpacingPx,
  saveReadingPreferences,
} from "./readingPreferences.ts";

function memoryStorage() {
  const values = new Map<string, string>();
  return {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => { values.set(key, value); },
    values,
  };
}

test("persists reading preferences by local workspace without leaking them into content", () => {
  const storage = memoryStorage();
  const preferences = {
    ...DEFAULT_READING_PREFERENCES,
    font: "serif" as const,
    fontSize: "custom" as const,
    fontSizePx: 19,
    lineHeight: "custom" as const,
    lineHeightValue: 1.9,
    width: "custom" as const,
    readingWidthCh: 84,
    letterSpacingEm: 0.015,
    paragraphSpacingEm: 1.3,
  } as const;

  assert.equal(saveReadingPreferences(storage, "workspace-a", preferences), true);
  assert.deepEqual(loadReadingPreferences(storage, "workspace-a"), preferences);
  assert.deepEqual(loadReadingPreferences(storage, "workspace-b"), DEFAULT_READING_PREFERENCES);
  assert.equal(storage.values.has(readingPreferencesStorageKey("workspace-a")), true);
});

test("persists each built-in reading font without changing the preference envelope", () => {
  const storage = memoryStorage();
  const fonts = [
    "caveat",
    "nunito",
    "lato",
    "cormorant",
    "chill-round-m",
    "xiaolai-sc",
    "smiley-sans",
  ] as const;

  for (const font of fonts) {
    const preferences = { ...DEFAULT_READING_PREFERENCES, font };
    assert.equal(saveReadingPreferences(storage, `workspace-${font}`, preferences), true);
    assert.deepEqual(loadReadingPreferences(storage, `workspace-${font}`), preferences);
  }
});

test("falls back safely for malformed, unknown-version or out-of-range preferences", () => {
  const storage = memoryStorage();
  storage.setItem(readingPreferencesStorageKey("broken"), "not-json");
  storage.setItem(readingPreferencesStorageKey("future"), JSON.stringify({ version: 99, preferences: {} }));
  storage.setItem(readingPreferencesStorageKey("invalid"), JSON.stringify({
    version: 2,
    preferences: {
      font: "remote-url",
      fontSize: "tiny",
      fontSizePx: 99,
      lineHeight: "0",
      lineHeightValue: 0,
      width: "unbounded",
      readingWidthCh: 400,
      letterSpacingEm: 2,
      paragraphSpacingEm: -1,
    },
  }));

  assert.deepEqual(loadReadingPreferences(storage, "broken"), DEFAULT_READING_PREFERENCES);
  assert.deepEqual(loadReadingPreferences(storage, "future"), DEFAULT_READING_PREFERENCES);
  assert.deepEqual(loadReadingPreferences(storage, "invalid"), DEFAULT_READING_PREFERENCES);
});

test("migrates the previous workspace preference envelope and supplies new defaults", () => {
  const storage = memoryStorage();
  storage.setItem("mindscape.reading-preferences.v1:legacy", JSON.stringify({
    version: 1,
    preferences: { font: "xiaolai-sc", fontSize: "large", lineHeight: "spacious", width: "wide" },
  }));

  assert.deepEqual(loadReadingPreferences(storage, "legacy"), {
    ...DEFAULT_READING_PREFERENCES,
    font: "xiaolai-sc",
    fontSize: "large",
    fontSizePx: 15,
    lineHeight: "spacious",
    lineHeightValue: 2,
    width: "wide",
    readingWidthCh: 92,
  });
});

test("reports storage rejection without throwing", () => {
  const storage = {
    getItem: () => null,
    setItem: () => { throw new Error("quota"); },
  };

  assert.equal(saveReadingPreferences(storage, "workspace", DEFAULT_READING_PREFERENCES), false);
});

test("resolves paragraph rhythm against the body size instead of a larger heading", () => {
  assert.equal(resolveReadingParagraphSpacingPx(20, 0.5), 10);
  assert.equal(resolveReadingParagraphSpacingPx(19, 1.3), 24.7);
});
