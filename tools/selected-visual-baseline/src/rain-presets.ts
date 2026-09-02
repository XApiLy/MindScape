export type RainEngine = "unified" | "original";

export const RAIN_ENGINE_STORAGE_KEY = "mindscape-rain-engine-v1";
export const RAIN_PRESET_STORAGE_KEY = "mindscape-rain-preset-v1";

export interface RainPresetSnapshot {
  version: 1;
  savedAt: string;
  engine: RainEngine;
  mode: "off" | "drizzle" | "rain" | "storm";
  intensity: number;
  placement: "behind" | "surface" | "foreground";
  visibility: number;
  unified: {
    frameRate: 45;
    brightness: 1.02;
    alphaSubtract: 4;
    minRefraction: string;
    maxRefraction: 180;
    sharedMaterial: true;
    componentClip: "svg-rects";
  };
  original: {
    source: "react-weather-effects-master/src/app/rain/rain-renderer.jsx";
    frameRate: 45;
  };
}

export function readInitialRainEngine(): RainEngine {
  if (typeof window === "undefined") return "unified";
  return window.sessionStorage.getItem(RAIN_ENGINE_STORAGE_KEY) === "original" ? "original" : "unified";
}

export function saveRainPreset(snapshot: RainPresetSnapshot): void {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(RAIN_PRESET_STORAGE_KEY, JSON.stringify(snapshot));
  } catch {
    // Storage is a convenience; the running renderer remains authoritative.
  }
}

export function readRainPreset(): RainPresetSnapshot | null {
  if (typeof window === "undefined") return null;
  try {
    const raw = window.localStorage.getItem(RAIN_PRESET_STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Partial<RainPresetSnapshot>;
    return parsed.version === 1 ? parsed as RainPresetSnapshot : null;
  } catch {
    return null;
  }
}
