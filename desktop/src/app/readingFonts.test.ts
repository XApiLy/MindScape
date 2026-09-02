import assert from "node:assert/strict";
import test from "node:test";
import {
  BUILT_IN_READING_FONT_IDS,
  BUILT_IN_READING_FONT_PRESETS,
  getBuiltInReadingFontPreset,
  loadBuiltInReadingFont,
} from "./readingFonts.ts";

test("declares the seven approved built-in presets with stable local aliases and fallbacks", () => {
  assert.equal(BUILT_IN_READING_FONT_PRESETS.length, 7);
  assert.equal(new Set(BUILT_IN_READING_FONT_IDS).size, 7);

  for (const preset of BUILT_IN_READING_FONT_PRESETS) {
    assert.match(preset.fontFaceFamily, /^MindScape /);
    assert.ok(preset.fallbackSummary.length > 0);
    assert.match(preset.preview, /知识/);
    assert.match(preset.preview, /MindScape 2026/);
    assert.equal(/[\\/]|https?:/i.test(preset.fontFaceFamily), false);
    assert.equal(getBuiltInReadingFontPreset(preset.id), preset);
  }
});

test("reports a loaded local face as available and a missing or rejected face as fallback", async () => {
  const requests: string[] = [];
  const available = await loadBuiltInReadingFont("nunito", {
    load: async (font) => {
      requests.push(font);
      return [{}];
    },
  });
  const missing = await loadBuiltInReadingFont("lato", { load: async () => [] });
  const rejected = await loadBuiltInReadingFont("caveat", {
    load: async () => { throw new Error("font decode failed"); },
  });

  assert.equal(available, "available");
  assert.equal(missing, "fallback");
  assert.equal(rejected, "fallback");
  assert.deepEqual(requests, ['16px "MindScape Nunito"']);
});

test("does not attempt to load system presets through the built-in font path", async () => {
  let calls = 0;
  const result = await loadBuiltInReadingFont("sans", {
    load: async () => {
      calls += 1;
      return [{}];
    },
  });

  assert.equal(result, "fallback");
  assert.equal(calls, 0);
});
