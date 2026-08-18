import assert from "node:assert/strict";
import test from "node:test";
import {
  centerCanvasViewportOnPoint,
  clampCanvasZoom,
  panCanvasViewport,
  zoomCanvasViewportAtPoint,
} from "./canvasViewport.ts";

test("pans without changing zoom", () => {
  assert.deepEqual(
    panCanvasViewport({ x: 10, y: 20, zoom: 0.8 }, { x: -4, y: 7 }),
    { x: 6, y: 27, zoom: 0.8 },
  );
});

test("zooms around the pointer and preserves its world coordinate", () => {
  const before = { x: 80, y: 40, zoom: 0.8 };
  const pointer = { x: 640, y: 360 };
  const worldBefore = {
    x: (pointer.x - before.x) / before.zoom,
    y: (pointer.y - before.y) / before.zoom,
  };
  const after = zoomCanvasViewportAtPoint(before, 1.2, pointer);
  const worldAfter = {
    x: (pointer.x - after.x) / after.zoom,
    y: (pointer.y - after.y) / after.zoom,
  };

  assert.deepEqual(worldAfter, worldBefore);
  assert.equal(clampCanvasZoom(0.1), 0.42);
  assert.equal(clampCanvasZoom(2), 1.55);
});

test("centers a world point without changing zoom", () => {
  assert.deepEqual(
    centerCanvasViewportOnPoint(
      { x: 0, y: 0, zoom: 0.5 },
      { width: 1200, height: 800 },
      { x: 400, y: 300 },
    ),
    { x: 400, y: 250, zoom: 0.5 },
  );
});
