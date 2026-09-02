import type {
  ImportBundleQueryProjection,
  ImportKnowledgeEntityProposal,
  ImportKnowledgeProposalRequestInput,
  ImportKnowledgeProposalReviewChoice,
  ImportKnowledgeProposalReviewCommandInput,
  KnowledgeScope,
} from "../domain";

export function buildImportKnowledgeProposalRequest(
  bundle: ImportBundleQueryProjection,
  selectedMessageIds: readonly string[],
  targetScope: KnowledgeScope,
  requestId: string,
  requestedAt: string,
): ImportKnowledgeProposalRequestInput {
  return {
    requestId,
    importSourceId: bundle.source.id,
    importRevisionId: bundle.revision.id,
    expectedSourceContentHash: bundle.source.contentHash,
    selectedMessageIds: [...new Set(selectedMessageIds)].sort(),
    targetScope,
    requestedAt,
  };
}

export function buildImportKnowledgeProposalReview(
  proposal: ImportKnowledgeEntityProposal,
  choice: ImportKnowledgeProposalReviewChoice,
  decisionId: string,
  decidedAt: string,
): ImportKnowledgeProposalReviewCommandInput {
  return {
    decisionId,
    requestId: proposal.requestId,
    proposalId: proposal.proposalId,
    expectedProposalRevision: proposal.proposalRevision,
    choice: choice.action === "confirm"
      ? {
          ...choice,
          name: choice.name.trim(),
          aliases: [...new Set(choice.aliases.map((alias) => alias.trim()).filter(Boolean))].sort(),
        }
      : {
          action: "reject",
          reason: choice.reason?.trim() || null,
        },
    decidedAt,
  };
}
