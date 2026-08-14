import { useMemo, useState } from "react";
import { ReactFlowProvider } from "@xyflow/react";
import { Header } from "./components/Header";
import { Sidebar } from "./components/Sidebar";
import { ChatCanvas } from "./components/canvas/ChatCanvas";
import { Composer } from "./components/Composer";
import { FocusView } from "./components/FocusView";
import { ImportDialog } from "./components/dialogs/ImportDialog";
import { ProviderDialog } from "./components/dialogs/ProviderDialog";
import { useWorkspaceStore } from "./store/workspaceStore";
import "@xyflow/react/dist/style.css";
import "./App.css";

function WorkspaceShell() {
  const [importOpen, setImportOpen] = useState(false);
  const [providerOpen, setProviderOpen] = useState(false);
  const nodes = useWorkspaceStore((state) => state.nodes);
  const focusedNodeId = useWorkspaceStore((state) => state.focusedNodeId);
  const focusedNode = useMemo(
    () => nodes.find((node) => node.id === focusedNodeId) ?? null,
    [focusedNodeId, nodes],
  );

  return (
    <div className="app-shell">
      <Sidebar onImport={() => setImportOpen(true)} />
      <main className="workspace">
        <Header onProviders={() => setProviderOpen(true)} />
        <section className="canvas-shell" aria-label="会话画布">
          <ChatCanvas />
          <Composer onProviders={() => setProviderOpen(true)} />
        </section>
      </main>

      {focusedNode ? <FocusView node={focusedNode} /> : null}
      <ImportDialog open={importOpen} onClose={() => setImportOpen(false)} />
      <ProviderDialog open={providerOpen} onClose={() => setProviderOpen(false)} />
    </div>
  );
}

function App() {
  return (
    <ReactFlowProvider>
      <WorkspaceShell />
    </ReactFlowProvider>
  );
}

export default App;
