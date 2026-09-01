import type { EvidenceRef } from "./evidence";

export type DiscussionLogScope =
  | { type: "project"; workspaceId: string; projectId: string }
  | {
      type: "conversation";
      workspaceId: string;
      conversationId: string;
      focusFrameId: string | null;
    };

export type DiscussionLog = {
  contractVersion: "mindscape.discussion-log.v1";
  id: string;
  scope: DiscussionLogScope;
  title: string;
  bodyMarkdown: string;
  relatedEntityIds: string[];
  evidence: EvidenceRef[];
  revision: number;
  createdAt: string;
  updatedAt: string;
};

export type DiscussionLogProjection = {
  contractVersion: "mindscape.discussion-log-projection.v1";
  log: DiscussionLog;
  relativePath: string;
  contentHash: string;
};

export type DiscussionLogEditCommandResult = {
  projection: DiscussionLogProjection;
  changed: boolean;
};
