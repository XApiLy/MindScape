export type EvidenceTarget =
  | { type: "messageBlock"; messageId: string; contentBlockIndex: number }
  | {
      type: "importContent";
      importSourceId: string;
      importRevisionId: string;
      locator: string;
    }
  | { type: "attachmentContent"; attachmentId: string; locator: string | null }
  | { type: "toolResultBlock"; toolRunId: string; contentBlockIndex: number };

export type EvidenceRef = {
  id: string;
  target: EvidenceTarget;
  contentHash: string | null;
  excerpt: string | null;
  createdAt: string;
};
