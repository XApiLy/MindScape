export type ContentBlock =
  | { type: "text"; text: string }
  | { type: "code"; language: string | null; code: string }
  | { type: "link"; url: string; label: string | null }
  | {
      type: "attachmentRef";
      attachmentId: string;
      mediaType: string | null;
      displayName: string;
    }
  | { type: "toolCallRef"; toolRunId: string }
  | { type: "toolResultRef"; toolRunId: string }
  | { type: "unsupported"; originalType: string; rawJson: unknown };
