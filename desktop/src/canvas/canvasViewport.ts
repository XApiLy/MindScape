import type { CanvasPoint, CanvasViewport } from "./graphProjection";

export function clampCanvasZoom(zoom: number) {
  return Math.min(1.55, Math.max(0.42, zoom));
}

export function panCanvasViewport(
  start: CanvasViewport,
  delta: CanvasPoint,
): CanvasViewport {
  return {
    ...start,
    x: start.x + delta.x,
    y: start.y + delta.y,
  };
}

export function zoomCanvasViewportAtPoint(
  viewport: CanvasViewport,
  requestedZoom: number,
  point: CanvasPoint,
): CanvasViewport {
  const zoom = clampCanvasZoom(requestedZoom);
  const worldX = (point.x - viewport.x) / viewport.zoom;
  const worldY = (point.y - viewport.y) / viewport.zoom;
  return {
    zoom,
    x: point.x - worldX * zoom,
    y: point.y - worldY * zoom,
  };
}

export function centerCanvasViewportOnPoint(
  viewport: CanvasViewport,
  surface: { width: number; height: number },
  worldPoint: CanvasPoint,
): CanvasViewport {
  return {
    ...viewport,
    x: surface.width / 2 - worldPoint.x * viewport.zoom,
    y: surface.height / 2 - worldPoint.y * viewport.zoom,
  };
}
