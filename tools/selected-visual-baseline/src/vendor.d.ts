declare module "@selected/glace" {
  import type { ComponentType } from "react";
  export const Glass: ComponentType<Record<string, unknown>>;
  export type GlassTone = "light" | "dark";
  export type GlassProfile = "convex" | "concave" | "bevel";
}

declare module "@selected/glace-css";

declare module "@selected/ybouane" {
  export interface SelectedLiquidGlassInstance {
    destroy(): void;
    markChanged(element?: HTMLElement): void;
    isDragging?(): boolean;
    /** Registered glass panels, exposed by Ybouane for targeted invalidation. */
    glassSet: Set<HTMLElement>;
  }
  export const LiquidGlass: {
    init(options: {
      root: HTMLElement;
      glassElements: NodeListOf<HTMLElement> | HTMLElement[];
      defaults?: Record<string, unknown>;
    }): Promise<SelectedLiquidGlassInstance>;
  };
}

declare module "@selected/drei-cloud" {
  import type { ComponentType } from "react";
  export const Cloud: ComponentType<Record<string, unknown>>;
  export const Clouds: ComponentType<Record<string, unknown>>;
}

declare module "@selected/weather-rain" {
  import type { ComponentType } from "react";
  const RainEffect: ComponentType<{
    type?: string;
    backgroundImageUrl: string;
    paused?: boolean;
    intensity?: number;
    exiting?: boolean;
    exitDuration?: number;
      foregroundTargetId?: string;
      foreground?: boolean;
      composition?: "behind" | "surface" | "foreground";
      foregroundStrength?: number;
      foregroundClipSelector?: string;
      engine?: "unified" | "original";
      onReady?: () => void;
  }>;
  export default RainEffect;
}

declare module "@selected/weather-snow" {
  import type { ComponentType } from "react";
  const SnowEffect: ComponentType<{ type?: string; backgroundImageUrl: string }>;
  export default SnowEffect;
}

declare module "@selected/weather-fog" {
  import type { ComponentType } from "react";
  const FogEffect: ComponentType<{ type?: string; backgroundImageUrl: string }>;
  export default FogEffect;
}
