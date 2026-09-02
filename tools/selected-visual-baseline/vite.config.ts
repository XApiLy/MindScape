import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import type { Plugin } from "vite";
import react from "@vitejs/plugin-react";

const toolRoot = path.dirname(fileURLToPath(import.meta.url));
const workspaceRoot = path.resolve(toolRoot, "../..");
const rainEffectPath = path.resolve(workspaceRoot, "example/react-weather-effects-master/src/app/rain/RainEffect.jsx").replaceAll("\\", "/");

function nextImageShapeCompat(): Plugin {
  return {
    name: "mindscape-next-image-shape-compat",
    enforce: "pre",
    transform(code, id) {
      if (id.split("?")[0].replaceAll("\\", "/") !== rainEffectPath) return null;
      const transformed = code
        .replace(
          "import DropColor from './img/drop-color.png';",
          "import DropColorUrl from './img/drop-color.png?url';\nconst DropColor = { src: DropColorUrl };",
        )
        .replace(
          "import DropAlpha from './img/drop-alpha.png';",
          "import DropAlphaUrl from './img/drop-alpha.png?url';\nconst DropAlpha = { src: DropAlphaUrl };",
        )
        .replace(
          "import DropShine from './img/drop-shine.png';",
          "import DropShineUrl from './img/drop-shine.png?url';\nconst DropShine = { src: DropShineUrl };",
        );
      return { code: transformed, map: null };
    },
  };
}

export default defineConfig({
  plugins: [nextImageShapeCompat(), react()],
  resolve: {
    dedupe: ["react", "react-dom", "three", "@react-three/fiber", "gsap", "html-to-image"],
    alias: {
      "@selected/glace": path.resolve(workspaceRoot, "example/liquid glass/glace-main/src/index.ts"),
      "@selected/glace-css": path.resolve(workspaceRoot, "example/liquid glass/glace-main/src/styles.css"),
      "@selected/ybouane": path.resolve(workspaceRoot, "example/liquid glass/liquidglass-main/src/index.ts"),
      "@selected/drei-cloud": path.resolve(workspaceRoot, "example/drei-master/src/core/Cloud.tsx"),
      "@selected/weather-rain": path.resolve(workspaceRoot, "example/react-weather-effects-master/src/app/rain/RainEffect.jsx"),
      "@selected/weather-snow": path.resolve(workspaceRoot, "example/react-weather-effects-master/src/app/snow/SnowEffect.jsx"),
      "@selected/weather-fog": path.resolve(workspaceRoot, "example/react-weather-effects-master/src/app/fog/FogEffect.jsx")
    }
  },
  server: {
    fs: { allow: [workspaceRoot] },
    strictPort: true
  },
  preview: { strictPort: true }
});
