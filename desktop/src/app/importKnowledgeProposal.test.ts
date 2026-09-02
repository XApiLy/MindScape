import assert from "node:assert/strict";
import test from "node:test";
import {
  buildImportKnowledgeProposalRequest,
  buildImportKnowledgeProposalReview,
} from "./importKnowledgeProposal.ts";
import type {
  ImportBundleQueryProjection,
  ImportKnowledgeEntityProposal,
} from "../domain/index.ts";

const bundle: ImportBundleQueryProjection = {
  source: {
    id: "source-1",
    conversationId: "conversation-1",
    platform: "generic",
    originalFileName: "notes.md",
    contentHash: "sha256:source",
    storageRef: "aa/source",
    createdAt: "2026-09-01T00:00:00Z",
  },
  revision: {
    id: "revision-1",
    importSourceId: "source-1",
    adapterId: "generic-markdown",
    adapterVersion: "1",
    status: "parsed",
    createdAt: "2026-09-01T00:00:00Z",
  },
  messages: [],
  report: {
    importRevisionId: "revision-1",
    conversationCount: 1,
    messageCount: 0,
    attachmentCount: 0,
    toolRecordCount: 0,
    fieldRecovery: [],
    warnings: [],
    errors: [],
  },
};

const proposal: ImportKnowledgeEntityProposal = {
  contractVersion: "mindscape.import-knowledge-proposal.v1",
  proposalId: "proposal-1",
  requestId: "request-1",
  importSourceId: "source-1",
  importRevisionId: "revision-1",
  conversationId: "conversation-1",
  suggestedKind: "decision",
  suggestedName: "Keep evidence visible",
  suggestedAliases: ["Evidence"],
  targetScope: {
    type: "focusFrame",
    workspaceId: "workspace-1",
    conversationId: "conversation-1",
    focusFrameId: "focus-1",
  },
  evidence: [],
  generator: { kind: "model", generatorId: "extractor", generatorVersion: "1" },
  proposalRevision: 3,
  proposedAt: "2026-09-01T00:01:00Z",
};

test("proposal request preserves scope and normalizes explicit message selection", () => {
  assert.deepEqual(
    buildImportKnowledgeProposalRequest(
      bundle,
      ["message-b", "message-a", "message-b"],
      proposal.targetScope,
      "request-1",
      "2026-09-01T00:01:00Z",
    ),
    {
      requestId: "request-1",
      importSourceId: "source-1",
      importRevisionId: "revision-1",
      expectedSourceContentHash: "sha256:source",
      selectedMessageIds: ["message-a", "message-b"],
      targetScope: proposal.targetScope,
      requestedAt: "2026-09-01T00:01:00Z",
    },
  );
});

test("proposal confirmation normalizes editable text without accepting evidence or entity identity", () => {
  const input = buildImportKnowledgeProposalReview(
    proposal,
    { action: "confirm", kind: "topic", name: "  Verified topic  ", aliases: [" B ", "A", "A"] },
    "decision-1",
    "2026-09-01T00:02:00Z",
  );

  assert.deepEqual(input.choice, {
    action: "confirm",
    kind: "topic",
    name: "Verified topic",
    aliases: ["A", "B"],
  });
  assert.equal("entityId" in input, false);
  assert.equal("evidence" in input, false);
});

test("proposal rejection keeps an exact retryable command and normalizes an empty reason", () => {
  assert.deepEqual(
    buildImportKnowledgeProposalReview(
      proposal,
      { action: "reject", reason: "   " },
      "decision-reject",
      "2026-09-01T00:03:00Z",
    ).choice,
    { action: "reject", reason: null },
  );
});
