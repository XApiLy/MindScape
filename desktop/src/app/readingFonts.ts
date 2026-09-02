import type { ReadingFont } from "./readingPreferences";

export type BuiltInReadingFontId =
  | "caveat"
  | "nunito"
  | "lato"
  | "cormorant"
  | "chill-round-m"
  | "xiaolai-sc"
  | "smiley-sans";

export type ReadingFontPreset = {
  id: BuiltInReadingFontId;
  label: string;
  group: "clear" | "personal" | "atmosphere";
  purpose: string;
  coverage: "中文覆盖" | "西文为主";
  weights: string;
  fontFaceFamily: string;
  fallbackSummary: string;
  preview: string;
  disableSynthesis: boolean;
};

export const BUILT_IN_READING_FONT_PRESETS: readonly ReadingFontPreset[] = [
  {
    id: "nunito",
    label: "Nunito 圆润无衬线",
    group: "clear",
    purpose: "适合正文",
    coverage: "西文为主",
    weights: "可变字重 · 含斜体",
    fontFaceFamily: "MindScape Nunito",
    fallbackSummary: "中文回退寒蝉半圆体与系统无衬线",
    preview: "知识让思路更清晰 · MindScape 2026，Aa 09",
    disableSynthesis: false,
  },
  {
    id: "lato",
    label: "Lato 人文无衬线",
    group: "clear",
    purpose: "适合正文",
    coverage: "西文为主",
    weights: "400 / 700 · 含斜体",
    fontFaceFamily: "MindScape Lato",
    fallbackSummary: "中文回退系统无衬线",
    preview: "知识让思路更清晰 · MindScape 2026，Aa 09",
    disableSynthesis: false,
  },
  {
    id: "chill-round-m",
    label: "寒蝉半圆体",
    group: "clear",
    purpose: "适合正文",
    coverage: "中文覆盖",
    weights: "Regular",
    fontFaceFamily: "MindScape ChillRoundM",
    fallbackSummary: "加载失败时回退系统无衬线",
    preview: "知识让思路更清晰 · MindScape 2026，Aa 09",
    disableSynthesis: true,
  },
  {
    id: "xiaolai-sc",
    label: "小赖字体",
    group: "personal",
    purpose: "轻松阅读",
    coverage: "中文覆盖",
    weights: "Regular",
    fontFaceFamily: "MindScape Xiaolai",
    fallbackSummary: "加载失败时回退寒蝉半圆体与系统无衬线",
    preview: "知识让思路更清晰 · MindScape 2026，Aa 09",
    disableSynthesis: true,
  },
  {
    id: "smiley-sans",
    label: "得意黑",
    group: "personal",
    purpose: "个性短文",
    coverage: "中文覆盖",
    weights: "Regular Oblique",
    fontFaceFamily: "MindScape Smiley Sans",
    fallbackSummary: "加载失败时回退系统无衬线",
    preview: "知识让思路更清晰 · MindScape 2026，Aa 09",
    disableSynthesis: true,
  },
  {
    id: "caveat",
    label: "Caveat 手写",
    group: "atmosphere",
    purpose: "适合标题",
    coverage: "西文为主",
    weights: "可变字重",
    fontFaceFamily: "MindScape Caveat",
    fallbackSummary: "中文回退小赖字体与系统无衬线",
    preview: "知识让思路更清晰 · MindScape 2026，Aa 09",
    disableSynthesis: false,
  },
  {
    id: "cormorant",
    label: "Cormorant 衬线",
    group: "atmosphere",
    purpose: "适合标题",
    coverage: "西文为主",
    weights: "可变字重 · 含斜体",
    fontFaceFamily: "MindScape Cormorant",
    fallbackSummary: "中文回退系统宋体与无衬线",
    preview: "知识让思路更清晰 · MindScape 2026，Aa 09",
    disableSynthesis: false,
  },
] as const;

export const BUILT_IN_READING_FONT_IDS = BUILT_IN_READING_FONT_PRESETS.map(
  ({ id }) => id,
) as readonly BuiltInReadingFontId[];

const PRESET_BY_ID = new Map(BUILT_IN_READING_FONT_PRESETS.map((preset) => [preset.id, preset]));

export function isBuiltInReadingFont(font: ReadingFont): font is BuiltInReadingFontId {
  return PRESET_BY_ID.has(font as BuiltInReadingFontId);
}

export function getBuiltInReadingFontPreset(font: ReadingFont) {
  return PRESET_BY_ID.get(font as BuiltInReadingFontId) ?? null;
}

type FontFaceSetLike = {
  load: (font: string, text?: string) => Promise<unknown[]>;
};

export async function loadBuiltInReadingFont(
  font: ReadingFont,
  fontSet: FontFaceSetLike,
): Promise<"available" | "fallback"> {
  const preset = getBuiltInReadingFontPreset(font);
  if (!preset) return "fallback";
  try {
    const loadedFaces = await fontSet.load(`16px "${preset.fontFaceFamily}"`, "MindScape 2026");
    return loadedFaces.length > 0 ? "available" : "fallback";
  } catch {
    return "fallback";
  }
}
