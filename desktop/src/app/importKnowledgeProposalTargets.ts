import type { CanvasFocusFrameQueryProjection } from "../canvas/canvasM2Projection";
import type { Conversation, KnowledgeScope } from "../domain";

export type ImportKnowledgeProposalTargetOption = {
  id: string;
  label: string;
  scope: KnowledgeScope;
};

/**
 * Builds the user-visible knowledge destinations from current kernel projections.
 *
 * The UI always offers the selected conversation, then active non-mainline frames
 * that belong to that same conversation. Scope remains kernel-authored input; this
 * helper only prevents stale workspace projections from becoming selectable.
 */
export function buildImportKnowledgeProposalTargets(
  conversation: Pick<Conversation, "id" | "workspaceId">,
  focusFrameQueries: Iterable<CanvasFocusFrameQueryProjection>,
): ImportKnowledgeProposalTargetOption[] {
  const targets: ImportKnowledgeProposalTargetOption[] = [
    {
      id: `conversation:${conversation.id}`,
      label: "当前会话 · 确认后进入知识检索",
      scope: {
        type: "conversation",
        workspaceId: conversation.workspaceId,
        conversationId: conversation.id,
      },
    },
  ];
  const seenFocusFrameIds = new Set<string>();

  for (const query of focusFrameQueries) {
    const frame = query.lifecycle.focusFrame;
    if (
      query.lifecycle.status !== "active"
      || frame.conversationId !== conversation.id
      || frame.memoryScope.branchKind === "mainline"
      || seenFocusFrameIds.has(frame.id)
    ) {
      continue;
    }

    seenFocusFrameIds.add(frame.id);
    targets.push({
      id: `focusFrame:${frame.id}`,
      label: `活动分支 · ${frame.objective}`,
      scope: {
        type: "focusFrame",
        workspaceId: conversation.workspaceId,
        conversationId: conversation.id,
        focusFrameId: frame.id,
      },
    });
  }

  return targets;
}
