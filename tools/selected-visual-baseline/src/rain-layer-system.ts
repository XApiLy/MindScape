export type RainPlacement = "behind" | "surface" | "foreground";
export type RainComposite = "under" | "over";

export interface RainLayerDescriptor {
  id: "environment-underlay" | "component-surface" | "global-overlay";
  composite: RainComposite;
  order: number;
  scope: "viewport" | "element";
}

// Layer order is data, not scattered z-index decisions. A future component
// target can reuse the same descriptor with an element clip without changing
// the rain simulation or material.
export const RAIN_LAYER_ORDER = Object.freeze({
  wallpaper: 0,
  environment: 1,
  clouds: 2,
  readability: 3,
  ui: 70,
  transient: 110,
  componentRain: 90,
  globalRain: 900,
});

export function getRainLayerDescriptor(placement: RainPlacement): RainLayerDescriptor {
  if (placement === "foreground") {
    return {
      id: "global-overlay",
      composite: "over",
      order: RAIN_LAYER_ORDER.globalRain,
      scope: "viewport",
    };
  }
  if (placement === "surface") {
    return {
      id: "component-surface",
      composite: "over",
      order: RAIN_LAYER_ORDER.componentRain,
      scope: "element",
    };
  }
  return {
    id: "environment-underlay",
    composite: "under",
    order: RAIN_LAYER_ORDER.environment,
    scope: "viewport",
  };
}
