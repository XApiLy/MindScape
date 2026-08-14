import { useEffect } from "react";
import {
  Background,
  BackgroundVariant,
  Controls,
  MiniMap,
  ReactFlow,
  useReactFlow,
  type NodeTypes,
} from "@xyflow/react";
import { ConversationNodeView } from "./ConversationNode";
import { useWorkspaceStore } from "../../store/workspaceStore";

const nodeTypes: NodeTypes = { conversation: ConversationNodeView };

export function ChatCanvas() {
  const nodes = useWorkspaceStore((state) => state.nodes);
  const edges = useWorkspaceStore((state) => state.edges);
  const selectedNodeId = useWorkspaceStore((state) => state.selectedNodeId);
  const onNodesChange = useWorkspaceStore((state) => state.onNodesChange);
  const onEdgesChange = useWorkspaceStore((state) => state.onEdgesChange);
  const selectNode = useWorkspaceStore((state) => state.selectNode);
  const canvasMode = useWorkspaceStore((state) => state.canvasMode);
  const { fitView } = useReactFlow();

  useEffect(() => {
    const timer = window.setTimeout(() => void fitView({ padding: 0.14, duration: 360 }), 120);
    return () => window.clearTimeout(timer);
  }, [fitView]);

  return (
    <ReactFlow
      className={`mindscape-flow mode-${canvasMode}`}
      nodes={nodes.map((node) => ({ ...node, selected: node.id === selectedNodeId }))}
      edges={edges}
      nodeTypes={nodeTypes}
      onNodesChange={onNodesChange}
      onEdgesChange={onEdgesChange}
      onNodeClick={(_, node) => selectNode(node.id)}
      onPaneClick={() => selectNode(null)}
      minZoom={0.25}
      maxZoom={1.8}
      fitView
      fitViewOptions={{ padding: 0.14 }}
      proOptions={{ hideAttribution: true }}
    >
      <Background variant={BackgroundVariant.Dots} gap={24} size={1} color="#2a2b2d" />
      <MiniMap
        nodeColor={(node) => (node.selected ? "#c8a968" : "#55575b")}
        maskColor="rgba(7, 8, 9, 0.72)"
        className="canvas-minimap"
      />
      <Controls showInteractive={false} className="canvas-controls" />
    </ReactFlow>
  );
}
