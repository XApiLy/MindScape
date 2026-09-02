export const READING_PREFERENCES_VERSION = 2;

export type ReadingFont =
  | "sans"
  | "serif"
  | "accessible"
  | "caveat"
  | "nunito"
  | "lato"
  | "cormorant"
  | "chill-round-m"
  | "xiaolai-sc"
  | "smiley-sans";
export type ReadingFontSize = "small" | "standard" | "large" | "xlarge" | "custom";
export type ReadingLineHeight = "compact" | "comfortable" | "spacious" | "custom";
export type ReadingWidth = "standard" | "wide" | "custom";

export type ReadingPreferences = {
  font: ReadingFont;
  fontSize: ReadingFontSize;
  fontSizePx: number;
  lineHeight: ReadingLineHeight;
  lineHeightValue: number;
  width: ReadingWidth;
  readingWidthCh: number;
  letterSpacingEm: number;
  paragraphSpacingEm: number;
};

type StorageLike = {
  getItem: (key: string) => string | null;
  setItem: (key: string, value: string) => void;
};

export const DEFAULT_READING_PREFERENCES: ReadingPreferences = {
  font: "sans",
  fontSize: "standard",
  fontSizePx: 13,
  lineHeight: "comfortable",
  lineHeightValue: 1.78,
  width: "standard",
  readingWidthCh: 72,
  letterSpacingEm: 0,
  paragraphSpacingEm: 1,
};

export const READING_PREFERENCE_LIMITS = {
  fontSizePx: { min: 12, max: 24, step: 1 },
  lineHeightValue: { min: 1.4, max: 2.2, step: 0.05 },
  readingWidthCh: { min: 52, max: 110, step: 2 },
  letterSpacingEm: { min: -0.02, max: 0.08, step: 0.005 },
  paragraphSpacingEm: { min: 0.5, max: 1.8, step: 0.1 },
} as const;

export const READING_PRESET_VALUES = {
  fontSizePx: { small: 12, standard: 13, large: 15, xlarge: 17 },
  lineHeightValue: { compact: 1.55, comfortable: 1.78, spacious: 2 },
  readingWidthCh: { standard: 72, wide: 92 },
} as const;

const READING_FONTS = new Set<ReadingFont>([
  "sans",
  "serif",
  "accessible",
  "caveat",
  "nunito",
  "lato",
  "cormorant",
  "chill-round-m",
  "xiaolai-sc",
  "smiley-sans",
]);
const READING_FONT_SIZES = new Set<ReadingFontSize>(["small", "standard", "large", "xlarge", "custom"]);
const READING_LINE_HEIGHTS = new Set<ReadingLineHeight>(["compact", "comfortable", "spacious", "custom"]);
const READING_WIDTHS = new Set<ReadingWidth>(["standard", "wide", "custom"]);

const LEGACY_READING_PREFERENCES_VERSION = 1;

function legacyReadingPreferencesStorageKey(workspaceId: string) {
  return `mindscape.reading-preferences.v${LEGACY_READING_PREFERENCES_VERSION}:${workspaceId}`;
}

function safeNumber(
  value: unknown,
  limits: { min: number; max: number },
  fallback: number,
) {
  return typeof value === "number" && Number.isFinite(value) && value >= limits.min && value <= limits.max
    ? value
    : fallback;
}

export function readingPreferencesStorageKey(workspaceId: string) {
  return `mindscape.reading-preferences.v${READING_PREFERENCES_VERSION}:${workspaceId}`;
}

export function resolveReadingParagraphSpacingPx(
  fontSizePx: number,
  paragraphSpacingEm: number,
) {
  return Math.round(fontSizePx * paragraphSpacingEm * 1_000) / 1_000;
}

export function normalizeReadingPreferences(value: unknown): ReadingPreferences {
  if (!value || typeof value !== "object") return { ...DEFAULT_READING_PREFERENCES };
  const candidate = value as Partial<ReadingPreferences>;
  const fontSize = READING_FONT_SIZES.has(candidate.fontSize as ReadingFontSize)
    ? candidate.fontSize as ReadingFontSize
    : DEFAULT_READING_PREFERENCES.fontSize;
  const lineHeight = READING_LINE_HEIGHTS.has(candidate.lineHeight as ReadingLineHeight)
    ? candidate.lineHeight as ReadingLineHeight
    : DEFAULT_READING_PREFERENCES.lineHeight;
  const width = READING_WIDTHS.has(candidate.width as ReadingWidth)
    ? candidate.width as ReadingWidth
    : DEFAULT_READING_PREFERENCES.width;
  const presetFontSize = fontSize === "custom"
    ? DEFAULT_READING_PREFERENCES.fontSizePx
    : READING_PRESET_VALUES.fontSizePx[fontSize];
  const presetLineHeight = lineHeight === "custom"
    ? DEFAULT_READING_PREFERENCES.lineHeightValue
    : READING_PRESET_VALUES.lineHeightValue[lineHeight];
  const presetReadingWidth = width === "custom"
    ? DEFAULT_READING_PREFERENCES.readingWidthCh
    : READING_PRESET_VALUES.readingWidthCh[width];
  return {
    font: READING_FONTS.has(candidate.font as ReadingFont)
      ? candidate.font as ReadingFont
      : DEFAULT_READING_PREFERENCES.font,
    fontSize,
    fontSizePx: safeNumber(
      candidate.fontSizePx,
      READING_PREFERENCE_LIMITS.fontSizePx,
      presetFontSize,
    ),
    lineHeight,
    lineHeightValue: safeNumber(
      candidate.lineHeightValue,
      READING_PREFERENCE_LIMITS.lineHeightValue,
      presetLineHeight,
    ),
    width,
    readingWidthCh: safeNumber(
      candidate.readingWidthCh,
      READING_PREFERENCE_LIMITS.readingWidthCh,
      presetReadingWidth,
    ),
    letterSpacingEm: safeNumber(
      candidate.letterSpacingEm,
      READING_PREFERENCE_LIMITS.letterSpacingEm,
      DEFAULT_READING_PREFERENCES.letterSpacingEm,
    ),
    paragraphSpacingEm: safeNumber(
      candidate.paragraphSpacingEm,
      READING_PREFERENCE_LIMITS.paragraphSpacingEm,
      DEFAULT_READING_PREFERENCES.paragraphSpacingEm,
    ),
  };
}

export function loadReadingPreferences(storage: StorageLike, workspaceId: string) {
  try {
    const raw = storage.getItem(readingPreferencesStorageKey(workspaceId));
    if (!raw) {
      const legacyRaw = storage.getItem(legacyReadingPreferencesStorageKey(workspaceId));
      if (!legacyRaw) return { ...DEFAULT_READING_PREFERENCES };
      const legacyEnvelope = JSON.parse(legacyRaw) as { version?: unknown; preferences?: unknown };
      if (legacyEnvelope.version !== LEGACY_READING_PREFERENCES_VERSION) {
        return { ...DEFAULT_READING_PREFERENCES };
      }
      return normalizeReadingPreferences(legacyEnvelope.preferences);
    }
    const envelope = JSON.parse(raw) as { version?: unknown; preferences?: unknown };
    if (envelope.version !== READING_PREFERENCES_VERSION) {
      return { ...DEFAULT_READING_PREFERENCES };
    }
    return normalizeReadingPreferences(envelope.preferences);
  } catch {
    return { ...DEFAULT_READING_PREFERENCES };
  }
}

export function saveReadingPreferences(
  storage: StorageLike,
  workspaceId: string,
  preferences: ReadingPreferences,
) {
  try {
    storage.setItem(readingPreferencesStorageKey(workspaceId), JSON.stringify({
      version: READING_PREFERENCES_VERSION,
      preferences: normalizeReadingPreferences(preferences),
    }));
    return true;
  } catch {
    return false;
  }
}
