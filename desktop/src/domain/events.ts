export type AggregateType = "workspace" | "conversation" | "importSource" | "modelRun";

export type DomainEventType =
  | "workspaceCreated"
  | "conversationCreated"
  | "conversationRenamed"
  | "turnAppended"
  | "turnCompleted"
  | "nodePositionUpdated"
  | "modelRunCreated"
  | "modelRunEventRecorded"
  | "importSourceCreated"
  | "importRevisionCreated"
  | "continuationCreated"
  | "continuationInvalidated";

export type DomainEventEnvelope = {
  contractVersion: string;
  eventId: string;
  sequence: number | null;
  aggregateType: AggregateType;
  aggregateId: string;
  eventType: DomainEventType;
  payload: unknown;
  idempotencyKey: string | null;
  occurredAt: string;
};
