export type ChatTimelineEntry<TNode, TRun> =
  | { kind: "node"; node: TNode }
  | { kind: "run"; run: TRun };

export function projectChatTimeline<
  TNode extends { id: string },
  TRun extends { nodeId: string },
>(nodes: readonly TNode[], run: TRun | null): ChatTimelineEntry<TNode, TRun>[] {
  let runProjected = false;
  const entries = nodes.map<ChatTimelineEntry<TNode, TRun>>((node) => {
    if (run && node.id === run.nodeId) {
      runProjected = true;
      return { kind: "run", run };
    }
    return { kind: "node", node };
  });

  if (run && !runProjected) entries.push({ kind: "run", run });
  return entries;
}
