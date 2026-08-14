export type BranchType =
  | "continues"
  | "deepens"
  | "diverges"
  | "reframes"
  | "importedFrom";

export type RunState =
  | "pending"
  | "streaming"
  | "completed"
  | "cancelled"
  | "failed";

export type MessageRole = "system" | "user" | "assistant" | "imported";
