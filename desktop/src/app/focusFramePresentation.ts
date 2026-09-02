import {
  projectFocusFrameQuery as projectCanvasFocusFrameQuery,
  type CanvasFocusFrameQueryProjection,
  type CanvasFocusedContextState,
} from "../canvas/canvasM2Projection.ts";
import type { FocusFrameLifecycleStatus, FocusFrameQueryProjection } from "../domain/focus.ts";

export type FocusedContextPresentationState = CanvasFocusedContextState;

export type FocusFrameQueryPresentation = {
  lifecycleStatus: FocusFrameLifecycleStatus;
  lifecycleLabel: "当前聚焦" | "已关闭";
  contextState: FocusedContextPresentationState;
  contextLabel: "上下文快照待内核接入" | "已生成快照，暂无知识引用" | "已生成知识上下文";
  revision: number;
  closedAt: string | null;
};

/**
 * Adds Chat-facing labels on top of the canonical canvas projection. The
 * canonical projector owns lifecycle/context state semantics; this helper
 * must not infer them from canvas or React data.
 */
export function projectFocusFrameQuery(
  projection: FocusFrameQueryProjection,
): FocusFrameQueryPresentation | null {
  const projected: CanvasFocusFrameQueryProjection | null = projectCanvasFocusFrameQuery(projection);
  if (!projected) return null;
  const lifecycleStatus = projected.lifecycle.status;
  return {
    lifecycleStatus,
    lifecycleLabel: lifecycleStatus === "active" ? "当前聚焦" : "已关闭",
    contextState: projected.focusedContextState,
    contextLabel: ({
      unavailable: "上下文快照待内核接入",
      availableWithoutKnowledge: "已生成快照，暂无知识引用",
      availableWithKnowledge: "已生成知识上下文",
    } as const)[projected.focusedContextState],
    revision: projected.lifecycle.revision,
    closedAt: projected.lifecycle.closedAt,
  };
}
