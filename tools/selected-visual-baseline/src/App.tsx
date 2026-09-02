import {
  Suspense,
  lazy,
  memo,
  startTransition,
  useDeferredValue,
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type HTMLAttributes,
  type ReactNode,
} from "react";
import { Glass } from "@selected/glace";
import type { GlassProfile, GlassTone } from "@selected/glace";
import type { SelectedLiquidGlassInstance } from "@selected/ybouane";
import { getRainLayerDescriptor } from "./rain-layer-system";
import { readInitialRainEngine, saveRainPreset, type RainEngine } from "./rain-presets";

const RainEffect = lazy(() => import("@selected/weather-rain"));
const SelectedCloudLayer = lazy(() => import("./SelectedCloudLayer"));

type SurfaceMode = "opaque" | "frosted" | "liquid";
type Region = "sidebar" | "nodes" | "reader" | "composer";
type RegionMode = SurfaceMode | "inherit";
type RainMode = "off" | "drizzle" | "rain" | "storm";
type AtmospherePlacement = "behind" | "surface" | "foreground";

const ENVIRONMENT_FADE_MS = 460;
const RAIN_DRAIN_MS = 1050;
const CLOUD_DISPERSE_DELAY_MS = 420;
const WEATHER_EXIT_MS = 1800;

interface LiquidGlassSettings {
  blurAmount: number;
  refraction: number;
  chromAberration: number;
  edgeHighlight: number;
  specular: number;
  fresnel: number;
  distortion: number;
  cornerRadius: number;
  opacity: number;
  saturation: number;
  tintStrength: number;
  brightness: number;
  shadowOpacity: number;
  shadowSpread: number;
  shadowOffsetY: number;
  zRadius: number;
  bevelMode: number;
}

interface FrostedGlassSettings {
  tone: GlassTone;
  radius: number;
  blur: number;
  fallbackBlur: number;
  refractEnabled: boolean;
  refractScale: number;
  aberration: number;
  bezel: number;
  profile: GlassProfile;
  saturation: number;
  sheen: boolean;
  opacity: number;
  borderOpacity: number;
  highlightOpacity: number;
  shadowOpacity: number;
}

const DEFAULT_LIQUID_SETTINGS: LiquidGlassSettings = {
  // Keep the default close to Ybouane's own demo: the scene should bend
  // locally through the surface instead of becoming a uniformly blurred slab.
  blurAmount: 0.03,
  refraction: 1.08,
  chromAberration: 0.11,
  edgeHighlight: 0.16,
  specular: 0.1,
  fresnel: 1,
  distortion: 0.01,
  cornerRadius: 42,
  opacity: 0.97,
  saturation: 0.02,
  tintStrength: 0.06,
  brightness: 0,
  shadowOpacity: 0.3,
  shadowSpread: 10,
  shadowOffsetY: 1,
  zRadius: 44,
  bevelMode: 0,
};

const DEFAULT_FROSTED_SETTINGS: FrostedGlassSettings = {
  tone: "dark",
  radius: 28,
  blur: 22,
  fallbackBlur: 24,
  refractEnabled: false,
  refractScale: 24,
  aberration: 1,
  bezel: 0.18,
  profile: "bevel",
  saturation: 135,
  sheen: false,
  opacity: 0.34,
  // Glacé's rim is configurable, but the baseline should read as a quiet
  // sheet of blur rather than a bright outlined card.
  borderOpacity: 0,
  highlightOpacity: 0,
  shadowOpacity: 0.24,
};

const WALLPAPER_URL = "/wallpaper-landscape.jpg";

const SURFACE_LABELS: Record<SurfaceMode, string> = {
  opaque: "Classic Opaque",
  frosted: "Glacé 毛玻璃",
  liquid: "Ybouane Liquid Glass",
};

const REGION_LABELS: Record<Region, string> = {
  sidebar: "侧边栏",
  nodes: "节点卡片",
  reader: "阅读区",
  composer: "编辑器",
};

interface SelectedSurfaceProps extends HTMLAttributes<HTMLElement> {
  as?: "aside" | "article" | "section" | "div";
  children: ReactNode;
  mode: SurfaceMode;
  radius?: number;
  liquidSettings?: LiquidGlassSettings;
  frostedSettings?: FrostedGlassSettings;
}

const SelectedSurface = memo(function SelectedSurface({
  as = "section",
  children,
  className = "",
  mode,
  radius = 30,
  liquidSettings = DEFAULT_LIQUID_SETTINGS,
  frostedSettings = DEFAULT_FROSTED_SETTINGS,
  ...props
}: SelectedSurfaceProps) {
  const classes = `selected-surface selected-surface--${mode} ${className}`;
  const liquidConfigValue = useMemo(
    () => JSON.stringify({
      ...liquidSettings,
      cornerRadius: liquidSettings.cornerRadius || radius,
      floating: false,
      button: false,
    }),
    [liquidSettings, radius],
  );
  const frostedStyle = useMemo(() => {
    const isLight = frostedSettings.tone === "light";
    const background = isLight
      ? `rgba(245, 252, 249, ${frostedSettings.opacity})`
      : `rgba(12, 28, 33, ${frostedSettings.opacity})`;
    const border = isLight
      ? `rgba(255, 255, 255, ${frostedSettings.borderOpacity})`
      : `rgba(217, 250, 239, ${frostedSettings.borderOpacity})`;
    const highlight = isLight
      ? `rgba(255, 255, 255, ${frostedSettings.highlightOpacity})`
      : `rgba(236, 255, 249, ${frostedSettings.highlightOpacity})`;
    return {
      "--g-bg": background,
      "--g-border": border,
      "--g-highlight": highlight,
      "--g-shadow": `0 22px 58px rgba(0, 0, 0, ${frostedSettings.shadowOpacity}), 0 3px 14px rgba(0, 0, 0, ${frostedSettings.shadowOpacity * 0.45})`,
    } as CSSProperties;
  }, [frostedSettings]);

  if (mode === "frosted") {
    return (
      <Glass
        as={as}
        tone={frostedSettings.tone}
        className={classes}
        radius={frostedSettings.radius || radius}
        refract={frostedSettings.refractEnabled ? frostedSettings.refractScale : false}
        aberration={frostedSettings.aberration}
        bezel={frostedSettings.bezel}
        profile={frostedSettings.profile}
        blur={frostedSettings.blur}
        fallbackBlur={frostedSettings.fallbackBlur}
        saturation={frostedSettings.saturation}
        sheen={frostedSettings.sheen}
        style={frostedStyle}
        data-source="glace-c8f5a363ab2b"
        data-rain-surface="true"
        {...props}
      >
        <div className="surface-content">{children}</div>
      </Glass>
    );
  }

  const Tag = as;
  return (
    <Tag
      className={classes}
      data-selected-liquid={mode === "liquid" ? "true" : undefined}
      data-config={mode === "liquid" ? liquidConfigValue : undefined}
      data-source={mode === "liquid" ? "ybouane-5ebda520bebd" : "classic-fallback"}
      data-rain-surface="true"
      style={mode === "liquid" ? { borderRadius: liquidSettings.cornerRadius || radius } : undefined}
      {...props}
    >
      <div className="surface-content">{children}</div>
    </Tag>
  );
});

interface WeatherLayerProps {
  mode: RainMode;
  intensity: number;
  exiting: boolean;
  muted: boolean;
  lowPerformance: boolean;
  visible: boolean;
  foregroundTargetId: string;
  composition: AtmospherePlacement;
  foregroundStrength: number;
  foregroundClipSelector?: string;
  engine: RainEngine;
  onReady: () => void;
}

interface GlassRefreshScheduler {
  stop(): void;
}

const WeatherLayer = memo(function WeatherLayer({ mode, intensity, exiting, muted, lowPerformance, visible, foregroundTargetId, composition, foregroundStrength, foregroundClipSelector, engine, onReady }: WeatherLayerProps) {
  const isBehind = composition === "behind";
  return (
    <div
      className={`weather-layer ${mode === "off" ? "is-off" : ""}`}
      data-liquid-media-only="true"
      data-source="react-weather-effects-8326628e18cf"
      // React Weather Effects owns the complete wallpaper while it is live.
      // On exit it remains opaque long enough for the last drops to drain;
      // the identical static wallpaper is already waiting underneath.
      // In foreground mode this full-screen compositor is kept mounted for
      // the shared raindrop simulation, but hidden. The visible sibling is
      // the transparent optical pass; leaving this opaque would duplicate
      // the wallpaper behind every refracted glyph.
      data-rain-composition={composition}
      style={{ opacity: visible && !lowPerformance && isBehind ? 1 : 0 }}
      aria-hidden="true"
    >
      {mode === "off" || lowPerformance ? null : (
        <Suspense fallback={null}>
          <RainEffect
            type={mode}
            backgroundImageUrl={WALLPAPER_URL}
            paused={lowPerformance}
            intensity={intensity}
            exiting={exiting}
            exitDuration={RAIN_DRAIN_MS}
            foregroundTargetId={foregroundTargetId}
            composition={composition}
            foregroundStrength={foregroundStrength}
            foregroundClipSelector={foregroundClipSelector}
            engine={engine}
            onReady={onReady}
          />
        </Suspense>
      )}
    </div>
  );
});

function readInitialRainMode(): RainMode {
  const value = sessionStorage.getItem("mindscape-selected-rain");
  return value === "off" || value === "rain" || value === "storm" || value === "drizzle" ? value : "drizzle";
}

function createGlassRefreshScheduler(
  instance: SelectedLiquidGlassInstance,
  getPolicy: () => { muted: boolean; active: boolean },
): GlassRefreshScheduler {
  let timeoutId: number | null = null;
  let cursor = 0;
  let refreshCredit = 0;
  let stopped = false;
  const glasses = Array.from(instance.glassSet);

  // Feed the renderer a staggered 30Hz target per visible panel. Multiple
  // panels are distributed across display frames instead of bursting every
  // glass through the WebGL pipeline at once.
  const tick = () => {
    if (stopped) return;
    const policy = getPolicy();
    let nextDelay = 240;

    if (policy.active && glasses.length > 0 && !instance.isDragging?.()) {
      const targetPanelFps = policy.muted ? 10 : 30;
      refreshCredit += (glasses.length * targetPanelFps) / 60;
      const marksThisFrame = Math.min(glasses.length, Math.floor(refreshCredit));
      refreshCredit -= marksThisFrame;
      for (let index = 0; index < marksThisFrame; index += 1) {
        instance.markChanged(glasses[cursor % glasses.length]);
        cursor += 1;
      }
      nextDelay = 1000 / 60;
    } else if (instance.isDragging?.()) {
      // Do not accumulate a post-drag burst; the engine gives the moved panel
      // one final high-quality render on pointerup.
      refreshCredit = 0;
      nextDelay = 1000 / 60;
    }

    timeoutId = window.setTimeout(tick, nextDelay);
  };

  tick();
  return {
    stop() {
      stopped = true;
      if (timeoutId !== null) window.clearTimeout(timeoutId);
      timeoutId = null;
    },
  };
}

export function App() {
  const rootRef = useRef<HTMLElement>(null);
  const liquidInstanceRef = useRef<SelectedLiquidGlassInstance | null>(null);
  const glassRefreshSchedulerRef = useRef<GlassRefreshScheduler | null>(null);
  const atmospherePolicyRef = useRef({ muted: false, active: false });
  const [atmosphereReady, setAtmosphereReady] = useState(false);
  const [cloudReady, setCloudReady] = useState(false);
  const [surfaceMode, setSurfaceMode] = useState<SurfaceMode>("liquid");
  const [regionModes, setRegionModes] = useState<Record<Region, RegionMode>>({
    sidebar: "inherit",
    nodes: "inherit",
    reader: "inherit",
    composer: "inherit",
  });
  const [rainMode, setRainMode] = useState<RainMode>(readInitialRainMode);
  const [weatherRenderMode, setWeatherRenderMode] = useState<RainMode>(() => readInitialRainMode());
  const [weatherMounted, setWeatherMounted] = useState(() => readInitialRainMode() !== "off");
  const [weatherExiting, setWeatherExiting] = useState(false);
  const [focusDispersing, setFocusDispersing] = useState(false);
  const [focusCloudDispersing, setFocusCloudDispersing] = useState(false);
  const [cloudDispersing, setCloudDispersing] = useState(false);
  const [weatherReady, setWeatherReady] = useState(false);
  const [weatherIntensity, setWeatherIntensity] = useState(0.62);
  const [rainEngine, setRainEngine] = useState<RainEngine>(readInitialRainEngine);
  const [rainPresetSaved, setRainPresetSaved] = useState(false);
  const [cloudEnabled, setCloudEnabled] = useState(true);
  const [cloudIntensity, setCloudIntensity] = useState(0.5);
  const [atmospherePlacement, setAtmospherePlacement] = useState<AtmospherePlacement>("behind");
  const rainLayer = useMemo(() => getRainLayerDescriptor(atmospherePlacement), [atmospherePlacement]);
  // Raw React Weather Effects droplets are intentionally restrained when
  // drawn over text. Users can raise this, but the baseline remains legible.
  const [foregroundAtmosphereStrength, setForegroundAtmosphereStrength] = useState(0.32);
  const [liquidSettings, setLiquidSettings] = useState<LiquidGlassSettings>(() => ({ ...DEFAULT_LIQUID_SETTINGS }));
  const [frostedSettings, setFrostedSettings] = useState<FrostedGlassSettings>(() => ({ ...DEFAULT_FROSTED_SETTINGS }));
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [readerOpen, setReaderOpen] = useState(false);
  const [inputFocused, setInputFocused] = useState(false);
  const [reducedMotion, setReducedMotion] = useState(false);
  const [lowPerformance, setLowPerformance] = useState(false);
  const [liquidState, setLiquidState] = useState<"idle" | "loading" | "ready" | "failed">("loading");

  // Keep the control surface immediate while deferring the expensive material
  // redraw until the slider settles into a frame the browser can service.
  const deferredLiquidSettings = useDeferredValue(liquidSettings);
  const deferredFrostedSettings = useDeferredValue(frostedSettings);

  useEffect(() => {
    let timer = 0;
    let cloudTimer = 0;
    const startAtmosphere = () => setAtmosphereReady(true);

    // Keep the first interaction window free of the Weather/Three module
    // parse cost. The layers still arrive automatically; this only stages
    // their startup after the workspace has painted and accepted input.
    timer = window.setTimeout(startAtmosphere, 1200);
    // Drei's dynamic block is intentionally much larger than the weather
    // adapter. Keep it out of the first interaction window and let it arrive
    // after the workspace has had an idle frame to paint.
    cloudTimer = window.setTimeout(() => setCloudReady(true), 1800);

    return () => {
      if (timer) window.clearTimeout(timer);
      if (cloudTimer) window.clearTimeout(cloudTimer);
    };
  }, []);

  const materialFor = (region: Region): SurfaceMode => {
    const selected = regionModes[region] === "inherit" ? surfaceMode : regionModes[region];
    return lowPerformance && selected === "liquid" ? "opaque" : selected;
  };

  const resolvedMaterials = useMemo(
    () => ({
      sidebar: materialFor("sidebar"),
      nodes: materialFor("nodes"),
      reader: materialFor("reader"),
      composer: materialFor("composer"),
    }),
    [lowPerformance, regionModes, surfaceMode],
  );

  const surfaceSignature = `${resolvedMaterials.sidebar}:${resolvedMaterials.nodes}:${resolvedMaterials.reader}:${resolvedMaterials.composer}:${settingsOpen}:${readerOpen}`;
  // Appearance settings are a live preview surface: opening them must not
  // hide the weather that the user is actively tuning.
  const atmosphereMuted = inputFocused || readerOpen;
  const weatherActive = atmosphereReady && rainMode !== "off" && !lowPerformance;
  // Keep the compositor mounted and opaque during focus. The rain renderer
  // drains its existing drops through `exiting`; hiding the whole canvas here
  // would also hide its wallpaper and create the hard cut we are avoiding.
  const weatherVisible = atmosphereReady && weatherMounted && weatherReady && !lowPerformance;
  const [wallpaperMounted, setWallpaperMounted] = useState(true);
  const [wallpaperVisible, setWallpaperVisible] = useState(true);

  useEffect(() => {
    let fadeTimer: number | undefined;
    let cloudTimer: number | undefined;

    if (!atmosphereReady) return undefined;

    if (rainMode === "off" && weatherMounted && weatherRenderMode !== "off" && !lowPerformance) {
      // Do not fade the full-screen compositor. Stop rain emission inside it,
      // let the remaining drops drain, and disperse the cloud field. The
      // static image is placed behind the opaque canvas for a seamless final
      // hand-off after both weather effects have cleared.
      setWeatherExiting(true);
      setCloudDispersing(false);
      setWallpaperMounted(true);
      setWallpaperVisible(true);
      cloudTimer = window.setTimeout(() => setCloudDispersing(true), CLOUD_DISPERSE_DELAY_MS);
      fadeTimer = window.setTimeout(() => {
        setWeatherMounted(false);
        setWeatherRenderMode("off");
        setWeatherReady(false);
        setWeatherExiting(false);
        setCloudDispersing(false);
      }, WEATHER_EXIT_MS);
    } else if (!weatherActive) {
      // Low-performance fallback can use the shorter layer dissolve because
      // it is an explicit runtime policy rather than a weather transition.
      setWeatherExiting(false);
      setCloudDispersing(false);
      setWallpaperMounted(true);
      setWallpaperVisible(true);
      if (weatherMounted) {
        fadeTimer = window.setTimeout(() => {
          setWeatherMounted(false);
          setWeatherRenderMode("off");
          setWeatherReady(false);
        }, ENVIRONMENT_FADE_MS);
      }
    } else if (!weatherMounted || weatherRenderMode === "off") {
      // Mount the new compositor behind the current wallpaper and let it
      // fade in only after its first complete frame is ready.
      setWeatherMounted(true);
      setWeatherRenderMode(rainMode);
      setWeatherReady(false);
      setWeatherExiting(false);
      setCloudDispersing(false);
    } else if (weatherRenderMode !== rainMode) {
      // Changing rain presets keeps the environment visible, but waits for
      // the new texture set before cross-fading to it.
      setWeatherRenderMode(rainMode);
      setWeatherReady(false);
      setWeatherExiting(false);
      setCloudDispersing(false);
    } else {
      setWeatherExiting(false);
      setCloudDispersing(false);
    }

    return () => {
      if (fadeTimer !== undefined) window.clearTimeout(fadeTimer);
      if (cloudTimer !== undefined) window.clearTimeout(cloudTimer);
    };
  }, [atmosphereReady, lowPerformance, rainMode]);

  useEffect(() => {
    // Focus is reversible atmosphere state, not a weather off-switch. Keep
    // the renderer alive so existing drops can finish, then resume emission
    // when the composer loses focus.
    const shouldDisperse = Boolean(atmosphereReady && weatherActive && atmosphereMuted);
    setFocusDispersing(shouldDisperse);
    if (!shouldDisperse) {
      setFocusCloudDispersing(false);
      return undefined;
    }

    const timer = window.setTimeout(() => setFocusCloudDispersing(true), CLOUD_DISPERSE_DELAY_MS);
    return () => window.clearTimeout(timer);
  }, [atmosphereMuted, atmosphereReady, weatherActive]);

  useEffect(() => {
    let frameId = 0;
    let fadeTimer: number | undefined;

    if (weatherVisible && !weatherExiting && atmospherePlacement === "behind") {
      // Only the behind composition owns an opaque weather surface. Surface
      // and global compositions are transparent overlays and must keep the
      // wallpaper mounted underneath so their input chain remains
      // Background -> Components -> Atmosphere.
      setWallpaperMounted(true);
      setWallpaperVisible(false);
      fadeTimer = window.setTimeout(() => setWallpaperMounted(false), ENVIRONMENT_FADE_MS);
    } else {
      // Transparent surface/global compositions always retain the wallpaper;
      // during an exit it also provides the stable hand-off frame.
      setWallpaperMounted(true);
      frameId = window.requestAnimationFrame(() => setWallpaperVisible(true));
    }

    return () => {
      if (frameId) window.cancelAnimationFrame(frameId);
      if (fadeTimer !== undefined) window.clearTimeout(fadeTimer);
    };
  }, [atmospherePlacement, weatherExiting, weatherVisible]);

  const updateLiquidSetting = <K extends keyof LiquidGlassSettings>(key: K, value: LiquidGlassSettings[K]) => {
    setLiquidSettings((current) => ({ ...current, [key]: value }));
  };

  const updateFrostedSetting = <K extends keyof FrostedGlassSettings>(key: K, value: FrostedGlassSettings[K]) => {
    setFrostedSettings((current) => ({ ...current, [key]: value }));
  };

  // Keep the visual cadence while spreading Ybouane's expensive panel
  // renders across frames. A global markChanged() makes all panels render in
  // one burst; targeted marks preserve the same per-panel update interval and
  // keep each frame within a predictable budget.
  useEffect(() => {
    atmospherePolicyRef.current = {
      muted: atmosphereMuted,
      active: (atmosphereReady || cloudReady) && !lowPerformance && (rainMode !== "off" || cloudEnabled),
    };
  }, [atmosphereMuted, atmosphereReady, cloudEnabled, cloudReady, lowPerformance, rainMode]);

  useEffect(() => {
    const root = rootRef.current;
    if (!root) return;

    const glasses = root.querySelectorAll<HTMLElement>(":scope > [data-selected-liquid='true']");
    if (glasses.length === 0) {
      setLiquidState("idle");
      liquidInstanceRef.current?.destroy();
      liquidInstanceRef.current = null;
      return;
    }

    let disposed = false;
    let instance: SelectedLiquidGlassInstance | null = null;
    setLiquidState("loading");
    // Ybouane's DOM raster pass is the heaviest startup task. Start it after
    // the first input window; the live DOM remains usable while its glass
    // canvases warm in the background.
    const timer = window.setTimeout(() => {
      void import("@selected/ybouane").then(({ LiquidGlass }) => LiquidGlass.init({
        root,
        glassElements: glasses,
      })).then((created) => {
        if (disposed) {
          created.destroy();
          return;
        }
        instance = created;
        liquidInstanceRef.current = created;
        glassRefreshSchedulerRef.current?.stop();
        glassRefreshSchedulerRef.current = createGlassRefreshScheduler(
          created,
          () => atmospherePolicyRef.current,
        );
        setLiquidState("ready");
      }).catch((error: unknown) => {
        if (disposed) return;
        console.error("Selected Ybouane LiquidGlass failed to initialize", error);
        setLiquidState("failed");
      });
    }, 1500);

    return () => {
      disposed = true;
      window.clearTimeout(timer);
      glassRefreshSchedulerRef.current?.stop();
      glassRefreshSchedulerRef.current = null;
      instance?.destroy();
      if (liquidInstanceRef.current === instance) liquidInstanceRef.current = null;
    };
  }, [surfaceSignature]);

  useEffect(() => {
    liquidInstanceRef.current?.markChanged();
  }, [atmosphereMuted, atmosphereReady, cloudEnabled, cloudReady, cloudIntensity, rainMode, weatherIntensity]);

  const updateRegionMode = (region: Region, value: RegionMode) => {
    startTransition(() => {
      setRegionModes((current) => ({ ...current, [region]: value }));
    });
  };

  const changeRainMode = (next: RainMode) => {
    sessionStorage.setItem("mindscape-selected-rain", next);
    if (next === "off" && rainMode !== "off" && weatherMounted) {
      // Set this in the interaction itself so clouds never spend one render
      // in the disabled state before their dispersal animation begins.
      setWeatherExiting(true);
      setCloudDispersing(false);
    }
    setRainMode(next);
  };

  const changeRainEngine = (next: RainEngine) => {
    window.sessionStorage.setItem("mindscape-rain-engine-v1", next);
    setRainPresetSaved(false);
    setWeatherReady(false);
    setRainEngine(next);
  };

  const saveCurrentRainPreset = () => {
    saveRainPreset({
      version: 1,
      savedAt: new Date().toISOString(),
      engine: rainEngine,
      mode: rainMode,
      intensity: weatherIntensity,
      placement: atmospherePlacement,
      visibility: foregroundAtmosphereStrength,
      unified: {
        frameRate: 45,
        brightness: 1.02,
        alphaSubtract: 4,
        minRefraction: "48 + intensity * 80",
        maxRefraction: 180,
        sharedMaterial: true,
        componentClip: "svg-rects",
      },
      original: {
        source: "react-weather-effects-master/src/app/rain/rain-renderer.jsx",
        frameRate: 45,
      },
    });
    setRainPresetSaved(true);
    window.setTimeout(() => setRainPresetSaved(false), 1800);
  };

  return (
    <main
      ref={rootRef}
      className={`baseline-shell ${atmosphereMuted ? "is-atmosphere-muted" : ""} ${reducedMotion ? "is-reduced" : ""}`}
      data-atmosphere-placement={atmospherePlacement}
      data-rain-composition={atmospherePlacement}
      data-rain-layer={rainLayer.id}
      data-rain-composite={rainLayer.composite}
    >
      {wallpaperMounted ? <img className={`wallpaper-layer ${wallpaperVisible ? "is-visible" : "is-hidden"}`} src={WALLPAPER_URL} alt="" aria-hidden="true" /> : null}
      {atmosphereReady && weatherMounted ? (
        <WeatherLayer
          mode={weatherRenderMode}
          intensity={weatherIntensity}
          exiting={weatherExiting || focusDispersing}
          muted={false}
          lowPerformance={lowPerformance}
          visible={weatherVisible}
          foregroundTargetId="atmosphere-rain-foreground"
          composition={atmospherePlacement}
          foregroundStrength={foregroundAtmosphereStrength}
          foregroundClipSelector={atmospherePlacement === "surface" ? "[data-rain-surface='true']:not(.settings-panel)" : undefined}
          engine={rainEngine}
          onReady={() => setWeatherReady(true)}
        />
      ) : <div className="weather-layer is-booting" aria-hidden="true" />}
      {cloudReady ? (
        <Suspense fallback={<div className="cloud-layer" aria-hidden="true" />}>
          <SelectedCloudLayer
            // Clouds are an independent atmosphere source. Turning rain off
            // must not unmount an enabled cloud field; otherwise the cloud
            // control cannot be previewed or tuned on its own.
            enabled={cloudEnabled && !lowPerformance}
            dispersing={cloudDispersing || focusCloudDispersing}
            intensity={cloudIntensity}
            muted={atmosphereMuted}
            reducedMotion={reducedMotion}
            lowPerformance={lowPerformance}
            foreground={atmospherePlacement !== "behind"}
            foregroundStrength={foregroundAtmosphereStrength}
            clipToComponents={atmospherePlacement === "surface"}
          />
        </Suspense>
      ) : <div className="cloud-layer is-booting" aria-hidden="true" />}
      <div className="readability-layer" aria-hidden="true" />
      <div
        id="atmosphere-rain-foreground"
        className={`atmosphere-foreground-layer ${atmospherePlacement !== "behind" ? "is-active" : ""}`}
        data-rain-layer={atmospherePlacement === "surface" ? "component-surface" : "global-overlay"}
        data-rain-material="shared-optical"
        data-rain-scope={atmospherePlacement === "surface" ? "element" : "viewport"}
        data-rain-composition={atmospherePlacement}
        // The layer itself stays opaque as a compositor surface; rain
        // visibility is applied inside the shader so optical displacement and
        // specular highlights are not attenuated a second time by CSS.
        style={{ opacity: atmospherePlacement !== "behind" ? 1 : 0, zIndex: rainLayer.order }}
        aria-hidden="true"
      />

      <header className="app-header">
        <div className="brand-lockup">
          <span className="brand-mark">M</span>
          <div>
            <span className="eyebrow">MINDSCAPE / SELECTED BASELINE</span>
            <strong>深度研究</strong>
          </div>
        </div>
        <div className="source-rail" aria-label="当前选型">
          <span>LIQUID · YBOUANE</span>
          <span>FROSTED · GLACÉ</span>
          <span>WEATHER · REACT</span>
          <span>CLOUD · DREI</span>
        </div>
        <div className="header-actions">
          <span className={`engine-state is-${liquidState}`}><i />{liquidState === "ready" ? "Liquid ready" : liquidState}</span>
          <button type="button" className="icon-button" onClick={() => setSettingsOpen((value) => !value)} aria-label="打开外观设置">☼</button>
        </div>
      </header>

      <SelectedSurface as="aside" mode={resolvedMaterials.sidebar} radius={30} className="sidebar-panel" liquidSettings={deferredLiquidSettings} frostedSettings={deferredFrostedSettings}>
        <div className="workspace-identity">
          <span className="workspace-avatar">M</span>
          <div><small>当前空间</small><strong>深度研究</strong></div>
          <button type="button" aria-label="更多空间操作">•••</button>
        </div>
        <button className="new-thread" type="button"><span>＋</span> 新建探索</button>
        <nav className="thread-list" aria-label="探索列表">
          <span className="section-label">正在继续</span>
          <button className="thread is-active" type="button"><strong>如何组织一次深度研究</strong><small>刚刚 · 4 个节点</small></button>
          <button className="thread" type="button"><strong>产品范围的取舍原则</strong><small>昨天 · 12 个节点</small></button>
          <button className="thread" type="button"><strong>天气氛围与注意力</strong><small>周一 · 7 个节点</small></button>
        </nav>
        <div className="sidebar-footer"><i />视觉基线已锁定 <span>?</span></div>
      </SelectedSurface>

      <section className="workspace-heading">
        <span className="eyebrow">当前对话 · 画布视图</span>
        <div><h1>如何组织一次深度研究？</h1><span className="depth-label">深度 02</span></div>
      </section>

      <SelectedSurface as="article" mode={resolvedMaterials.nodes} radius={32} className="node-card node-card--primary" liquidSettings={deferredLiquidSettings} frostedSettings={deferredFrostedSettings}>
        <span className="node-number">01</span>
        <div><small>当前节点</small><h2>从问题开始，而不是从工具开始</h2><p>先让目标、证据与下一步在同一个空间里显形。</p></div>
        <button type="button" onClick={() => setReaderOpen(true)}>进入阅读 ↗</button>
      </SelectedSurface>

      <SelectedSurface as="article" mode={resolvedMaterials.nodes} radius={30} className="node-card node-card--secondary" liquidSettings={deferredLiquidSettings} frostedSettings={deferredFrostedSettings}>
        <span className="node-number">02</span>
        <div><small>分支 · 深入</small><h2>把研究拆成可以继续的问题</h2><p>沿着当前结论向下，保留原始上下文。</p></div>
        <button type="button">继续探索 →</button>
      </SelectedSurface>

      <p className="canvas-note">氛围只在内容之外移动</p>

      <SelectedSurface as="section" mode={resolvedMaterials.composer} radius={30} className="composer-panel" liquidSettings={deferredLiquidSettings} frostedSettings={deferredFrostedSettings}>
        <div className="composer-meta"><span>从节点 02 继续</span><small>{inputFocused ? "天气与云已退让" : "输入时氛围自动退让"}</small></div>
        <textarea
          aria-label="继续提问"
          placeholder="继续推进一个问题…"
          onFocus={() => setInputFocused(true)}
          onBlur={() => setInputFocused(false)}
        />
        <div className="composer-actions"><button type="button">DeepSeek Chat⌄</button><span>Enter 发送 · Shift + Enter 换行</span><button className="send-button" type="button">↑</button></div>
      </SelectedSurface>

      <SelectedSurface as="aside" mode={resolvedMaterials.reader} radius={30} className="reader-panel" liquidSettings={deferredLiquidSettings} frostedSettings={deferredFrostedSettings}>
        <span className="eyebrow">阅读预览</span>
        <h2>视觉层为思考服务</h2>
        <p>壁纸、天气和云属于环境；卡片材质属于信息结构。输入、选字和阅读时，环境主动退到内容之后。</p>
        <div className="reader-fact"><span>选型</span><strong>{SURFACE_LABELS[surfaceMode]}</strong></div>
        <button type="button" onClick={() => setReaderOpen(true)}>聚焦阅读</button>
      </SelectedSurface>

      {settingsOpen ? (
        <SelectedSurface as="aside" mode={resolvedMaterials.reader} radius={28} className="settings-panel" aria-label="外观设置" liquidSettings={deferredLiquidSettings} frostedSettings={deferredFrostedSettings}>
          <div className="settings-title"><div><span className="eyebrow">VIS-008</span><h2>已选视觉基线</h2></div><button type="button" onClick={() => setSettingsOpen(false)} aria-label="关闭设置">×</button></div>

          <fieldset>
            <legend>组件材质</legend>
            <div className="segmented-control">
              {(Object.keys(SURFACE_LABELS) as SurfaceMode[]).map((mode) => (
                <button key={mode} type="button" className={surfaceMode === mode ? "is-selected" : ""} onClick={() => startTransition(() => setSurfaceMode(mode))}>{SURFACE_LABELS[mode]}</button>
              ))}
            </div>
          </fieldset>

          <details className="settings-section" open={surfaceMode === "liquid"}>
            <summary><span>Ybouane Liquid Glass</span><small>WebGL 曲面</small></summary>
            <div className="settings-section-body">
              <div className="settings-section-toolbar"><span>高光、折射与曲率</span><button type="button" className="settings-reset" onClick={() => setLiquidSettings({ ...DEFAULT_LIQUID_SETTINGS })}>恢复默认</button></div>
              <label className="range-row"><span>角半径</span><input type="range" min="16" max="64" step="1" value={liquidSettings.cornerRadius} onChange={(event) => updateLiquidSetting("cornerRadius", Number(event.target.value))} /><output>{liquidSettings.cornerRadius}px</output></label>
              <label className="range-row"><span>折射强度</span><input type="range" min="0" max="1" step="0.01" value={liquidSettings.refraction} onChange={(event) => updateLiquidSetting("refraction", Number(event.target.value))} /><output>{liquidSettings.refraction.toFixed(2)}</output></label>
              <label className="range-row"><span>边缘高光</span><input type="range" min="0" max="1" step="0.01" value={liquidSettings.edgeHighlight} onChange={(event) => updateLiquidSetting("edgeHighlight", Number(event.target.value))} /><output>{liquidSettings.edgeHighlight.toFixed(2)}</output></label>
              <label className="range-row"><span>镜面高光</span><input type="range" min="0" max="1" step="0.01" value={liquidSettings.specular} onChange={(event) => updateLiquidSetting("specular", Number(event.target.value))} /><output>{liquidSettings.specular.toFixed(2)}</output></label>
              <label className="range-row"><span>菲涅耳</span><input type="range" min="0" max="2" step="0.05" value={liquidSettings.fresnel} onChange={(event) => updateLiquidSetting("fresnel", Number(event.target.value))} /><output>{liquidSettings.fresnel.toFixed(2)}</output></label>
              <label className="range-row"><span>背景模糊</span><input type="range" min="0" max="1" step="0.01" value={liquidSettings.blurAmount} onChange={(event) => updateLiquidSetting("blurAmount", Number(event.target.value))} /><output>{liquidSettings.blurAmount.toFixed(2)}</output></label>
              <label className="range-row"><span>色散</span><input type="range" min="0" max="0.2" step="0.01" value={liquidSettings.chromAberration} onChange={(event) => updateLiquidSetting("chromAberration", Number(event.target.value))} /><output>{liquidSettings.chromAberration.toFixed(2)}</output></label>
              <label className="range-row"><span>微扰</span><input type="range" min="0" max="0.12" step="0.01" value={liquidSettings.distortion} onChange={(event) => updateLiquidSetting("distortion", Number(event.target.value))} /><output>{liquidSettings.distortion.toFixed(2)}</output></label>
              <label className="range-row"><span>透明度</span><input type="range" min="0.55" max="1" step="0.01" value={liquidSettings.opacity} onChange={(event) => updateLiquidSetting("opacity", Number(event.target.value))} /><output>{liquidSettings.opacity.toFixed(2)}</output></label>
              <label className="range-row"><span>饱和度</span><input type="range" min="-0.4" max="0.8" step="0.02" value={liquidSettings.saturation} onChange={(event) => updateLiquidSetting("saturation", Number(event.target.value))} /><output>{liquidSettings.saturation.toFixed(2)}</output></label>
              <label className="range-row"><span>曲面深度</span><input type="range" min="8" max="64" step="1" value={liquidSettings.zRadius} onChange={(event) => updateLiquidSetting("zRadius", Number(event.target.value))} /><output>{liquidSettings.zRadius}px</output></label>
              <label className="select-row"><span>曲面形态</span><select value={liquidSettings.bevelMode} onChange={(event) => updateLiquidSetting("bevelMode", Number(event.target.value))}><option value="0">Biconvex 双面凸</option><option value="1">Dome 单面穹顶</option></select></label>
            </div>
          </details>

          <details className="settings-section" open={surfaceMode === "frosted"}>
            <summary><span>Glacé 毛玻璃</span><small>Backdrop blur</small></summary>
            <div className="settings-section-body">
              <div className="settings-section-toolbar"><span>模糊、边缘与主题</span><button type="button" className="settings-reset" onClick={() => setFrostedSettings({ ...DEFAULT_FROSTED_SETTINGS })}>恢复默认</button></div>
              <label className="select-row"><span>色调</span><select value={frostedSettings.tone} onChange={(event) => updateFrostedSetting("tone", event.target.value as GlassTone)}><option value="dark">暗色玻璃</option><option value="light">亮色玻璃</option></select></label>
              <label className="range-row"><span>角半径</span><input type="range" min="12" max="56" step="1" value={frostedSettings.radius} onChange={(event) => updateFrostedSetting("radius", Number(event.target.value))} /><output>{frostedSettings.radius}px</output></label>
              <label className="range-row"><span>实时模糊</span><input type="range" min="0" max="32" step="1" value={frostedSettings.blur} onChange={(event) => updateFrostedSetting("blur", Number(event.target.value))} /><output>{frostedSettings.blur}px</output></label>
              <label className="range-row"><span>回退模糊</span><input type="range" min="0" max="40" step="1" value={frostedSettings.fallbackBlur} onChange={(event) => updateFrostedSetting("fallbackBlur", Number(event.target.value))} /><output>{frostedSettings.fallbackBlur}px</output></label>
              <label className="toggle-row"><span>边缘折射</span><input type="checkbox" checked={frostedSettings.refractEnabled} onChange={(event) => updateFrostedSetting("refractEnabled", event.target.checked)} /></label>
              <label className="range-row"><span>折射位移</span><input type="range" min="0" max="120" step="1" value={frostedSettings.refractScale} disabled={!frostedSettings.refractEnabled} onChange={(event) => updateFrostedSetting("refractScale", Number(event.target.value))} /><output>{frostedSettings.refractScale}px</output></label>
              <label className="range-row"><span>色散</span><input type="range" min="0" max="8" step="0.5" value={frostedSettings.aberration} disabled={!frostedSettings.refractEnabled} onChange={(event) => updateFrostedSetting("aberration", Number(event.target.value))} /><output>{frostedSettings.aberration.toFixed(1)}</output></label>
              <label className="range-row"><span>边缘厚度</span><input type="range" min="0" max="0.5" step="0.01" value={frostedSettings.bezel} disabled={!frostedSettings.refractEnabled} onChange={(event) => updateFrostedSetting("bezel", Number(event.target.value))} /><output>{frostedSettings.bezel.toFixed(2)}</output></label>
              <label className="select-row"><span>边缘轮廓</span><select value={frostedSettings.profile} disabled={!frostedSettings.refractEnabled} onChange={(event) => updateFrostedSetting("profile", event.target.value as GlassProfile)}><option value="convex">Convex 凸面</option><option value="concave">Concave 凹面</option><option value="bevel">Bevel 切面</option></select></label>
              <label className="range-row"><span>饱和度</span><input type="range" min="80" max="240" step="5" value={frostedSettings.saturation} onChange={(event) => updateFrostedSetting("saturation", Number(event.target.value))} /><output>{frostedSettings.saturation}%</output></label>
              <label className="range-row"><span>表面透明度</span><input type="range" min="0.32" max="0.86" step="0.01" value={frostedSettings.opacity} onChange={(event) => updateFrostedSetting("opacity", Number(event.target.value))} /><output>{frostedSettings.opacity.toFixed(2)}</output></label>
              <label className="range-row"><span>边缘描边</span><input type="range" min="0" max="0.3" step="0.01" value={frostedSettings.borderOpacity} onChange={(event) => updateFrostedSetting("borderOpacity", Number(event.target.value))} /><output>{frostedSettings.borderOpacity.toFixed(2)}</output></label>
              <label className="range-row"><span>边缘高光</span><input type="range" min="0" max="0.5" step="0.01" value={frostedSettings.highlightOpacity} onChange={(event) => updateFrostedSetting("highlightOpacity", Number(event.target.value))} /><output>{frostedSettings.highlightOpacity.toFixed(2)}</output></label>
              <label className="range-row"><span>阴影强度</span><input type="range" min="0" max="0.6" step="0.01" value={frostedSettings.shadowOpacity} onChange={(event) => updateFrostedSetting("shadowOpacity", Number(event.target.value))} /><output>{frostedSettings.shadowOpacity.toFixed(2)}</output></label>
              <label className="toggle-row"><span>悬浮高光</span><input type="checkbox" checked={frostedSettings.sheen} onChange={(event) => updateFrostedSetting("sheen", event.target.checked)} /></label>
              <small>默认关闭边缘折射与边缘高光，让 Glacé 保持纯粹的背景模糊；需要时再单独打开。</small>
            </div>
          </details>

          <fieldset>
            <legend>雨滴方案</legend>
            <div className="settings-section-toolbar">
              <span>{rainEngine === "unified" ? "Unified Optical · 当前方案" : "Original Weather Effects · 原仓"}</span>
              <button type="button" className="settings-reset" onClick={saveCurrentRainPreset}>{rainPresetSaved ? "已保存" : "保存当前方案"}</button>
            </div>
            <select value={rainEngine} onChange={(event) => changeRainEngine(event.target.value as RainEngine)}>
              <option value="unified">Unified Optical · 统一光学</option>
              <option value="original">Original Weather Effects · 原仓</option>
            </select>
            <small>统一方案保留当前折射、反射和高光参数；原仓方案保留 react-weather-effects 的原始合成路径。两套方案共用同一层级系统。</small>
          </fieldset>

          <fieldset>
            <legend>雨滴层级</legend>
            <div className="segmented-control segmented-control--inline">
              <button type="button" className={atmospherePlacement === "behind" ? "is-selected" : ""} onClick={() => setAtmospherePlacement("behind")}>组件背后</button>
              <button type="button" className={atmospherePlacement === "surface" ? "is-selected" : ""} onClick={() => setAtmospherePlacement("surface")}>组件表面</button>
              <button type="button" className={atmospherePlacement === "foreground" ? "is-selected" : ""} onClick={() => setAtmospherePlacement("foreground")}>全局最上层</button>
            </div>
            <label className="range-row"><span>雨滴可见度</span><input type="range" min="0" max="1" step="0.01" value={foregroundAtmosphereStrength} onChange={(event) => setForegroundAtmosphereStrength(Number(event.target.value))} /><output>{foregroundAtmosphereStrength.toFixed(2)}</output></label>
            <small>三个目标共享同一雨滴模拟和光学材质：组件表面只增加区域裁剪，全局最上层只改变合成层序，雨滴形状、折射、反射和高光不会改变。</small>
          </fieldset>

          <fieldset>
            <legend>React Weather Effects</legend>
            <select value={rainMode} onChange={(event) => changeRainMode(event.target.value as RainMode)}>
              <option value="off">关闭</option>
              <option value="drizzle">Drizzle · 推荐</option>
              <option value="rain">Rain</option>
              <option value="storm">Storm</option>
            </select>
            <label className="range-row"><span>强度</span><input type="range" min="0.25" max="1" step="0.05" value={weatherIntensity} onChange={(event) => setWeatherIntensity(Number(event.target.value))} /><output>{weatherIntensity.toFixed(2)}</output></label>
            <small>输入聚焦时会先停止新雨滴，残留雨滴自然退场，云层随后向两侧散开；离开输入后反向恢复。</small>
          </fieldset>

          <fieldset>
            <legend>Drei Cloud</legend>
            <label className="toggle-row"><span>云层</span><input type="checkbox" checked={cloudEnabled} onChange={(event) => setCloudEnabled(event.target.checked)} /></label>
            <label className="range-row"><span>强度</span><input type="range" min="0.15" max="0.75" step="0.05" value={cloudIntensity} onChange={(event) => setCloudIntensity(Number(event.target.value))} /><output>{cloudIntensity.toFixed(2)}</output></label>
          </fieldset>

          <fieldset>
            <legend>区域覆盖</legend>
            {(Object.keys(REGION_LABELS) as Region[]).map((region) => (
              <label className="select-row" key={region}><span>{REGION_LABELS[region]}</span><select value={regionModes[region]} onChange={(event) => updateRegionMode(region, event.target.value as RegionMode)}><option value="inherit">跟随全局</option><option value="liquid">Ybouane</option><option value="frosted">Glacé</option><option value="opaque">Opaque</option></select></label>
            ))}
          </fieldset>

          <fieldset>
            <legend>运行策略</legend>
            <label className="toggle-row"><span>减少动态</span><input type="checkbox" checked={reducedMotion} onChange={(event) => setReducedMotion(event.target.checked)} /></label>
            <label className="toggle-row"><span>低性能回退</span><input type="checkbox" checked={lowPerformance} onChange={(event) => setLowPerformance(event.target.checked)} /></label>
          </fieldset>
        </SelectedSurface>
      ) : null}

      {readerOpen ? (
        <>
          <button className="reader-backdrop" type="button" aria-label="退出聚焦阅读" onClick={() => setReaderOpen(false)} />
          <SelectedSurface as="article" mode={resolvedMaterials.reader} radius={34} className="focus-reader" liquidSettings={deferredLiquidSettings} frostedSettings={deferredFrostedSettings}>
            <button className="focus-close" type="button" onClick={() => setReaderOpen(false)}>退出阅读 ×</button>
            <span className="eyebrow">聚焦阅读 · 氛围已退让</span>
            <h2>从问题开始，而不是从工具开始</h2>
            <p>一个好的研究工作区不会不断争夺注意力。环境效果只负责建立情绪和空间感；真正进入阅读、输入与选择时，它必须主动让位。</p>
            <p>本基线中的 Liquid Glass 只由 Ybouane WebGL 管线生成，毛玻璃只由 Glacé 表面生成；天气来自 React Weather Effects，云来自 Drei Cloud。</p>
          </SelectedSurface>
        </>
      ) : null}

      <footer className="baseline-footer">
        <span>VIS-008 · FOUNDER CONFIRMED</span>
        <span>原仓职责已锁定 · 生产实现仍等待 M1 门禁</span>
      </footer>
    </main>
  );
}
