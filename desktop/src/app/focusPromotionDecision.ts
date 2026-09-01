import type {
  FocusPromotionCandidateSet,
  FocusPromotionDecisionAction,
  FocusPromotionDecisionCommandInput,
  KnowledgeEntity,
} from "../domain";

type FocusPromotionLifecycleVersion = {
  lifecycle: { revision: number };
};

type FocusPromotionDecisionIdentity = {
  decisionId: string;
  promotedEntityId: string;
  decidedAt: string;
};

export function buildFocusPromotionDecisionInput(
  action: FocusPromotionDecisionAction,
  candidateSet: FocusPromotionCandidateSet,
  query: FocusPromotionLifecycleVersion,
  entity: KnowledgeEntity,
  identity: FocusPromotionDecisionIdentity,
): FocusPromotionDecisionCommandInput {
  const targetScope = action === "promote"
    ? {
        type: "conversation" as const,
        workspaceId: entity.scope.workspaceId,
        conversationId: candidateSet.conversationId,
      }
    : null;

  return {
    decisionId: identity.decisionId,
    focusFrameId: candidateSet.focusFrameId,
    candidateRef: entity.id,
    expectedMemoryVersion: candidateSet.memoryVersion,
    expectedLifecycleRevision: query.lifecycle.revision,
    expectedEntityRevision: entity.revision,
    expectedDecisionRevision: 0,
    action,
    targetScope,
    promotedEntityId: action === "promote" ? identity.promotedEntityId : null,
    decidedAt: identity.decidedAt,
  };
}
