import { ChevronDown, ChevronRight, FileInput, Folder, Layers3, MessageSquare, Plus, RotateCcw } from "lucide-react";
import { useWorkspaceStore } from "../store/workspaceStore";

type SidebarProps = {
  onImport: () => void;
};

export function Sidebar({ onImport }: SidebarProps) {
  const projects = useWorkspaceStore((state) => state.projects);
  const activeConversationId = useWorkspaceStore((state) => state.activeConversationId);
  const setActiveConversation = useWorkspaceStore((state) => state.setActiveConversation);
  const resetDemo = useWorkspaceStore((state) => state.resetDemo);

  return (
    <aside className="sidebar">
      <div className="brand-row">
        <span className="brand-mark" aria-hidden="true" />
        <strong>MindScape</strong>
        <button className="icon-button sidebar-collapse" title="收起侧栏" aria-label="收起侧栏">
          <ChevronRight size={16} />
        </button>
      </div>

      <button className="primary-action" type="button">
        <Plus size={17} />
        新建探索会话
      </button>

      <nav className="sidebar-nav" aria-label="主导航">
        <button className="nav-item active" type="button">
          <Layers3 size={16} />
          沉浸式探索
        </button>
        <button className="nav-item" type="button" onClick={onImport}>
          <FileInput size={16} />
          导入外部会话
        </button>
      </nav>

      <div className="section-label">项目 / 会话</div>
      <div className="project-list">
        {projects.map((project, index) => (
          <section className="project-group" key={project.id}>
            <div className="project-heading">
              {index === 0 ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
              <Folder size={15} />
              <span>{project.title}</span>
              <small>{project.count}</small>
            </div>
            {index === 0
              ? project.conversations.map((conversation) => (
                  <button
                    className={`conversation-link ${activeConversationId === conversation.id ? "active" : ""}`}
                    type="button"
                    key={conversation.id}
                    onClick={() => setActiveConversation(conversation.id)}
                  >
                    <MessageSquare size={14} />
                    <span>{conversation.title}</span>
                    <small>{conversation.updatedAt}</small>
                  </button>
                ))
              : null}
          </section>
        ))}
      </div>

      <div className="sidebar-footer">
        <button className="footer-action" type="button" onClick={resetDemo}>
          <RotateCcw size={15} />
          恢复演示画布
        </button>
        <div className="engine-status">
          <span className="status-dot" />
          <span>
            MindScape Core
            <small>Local workspace</small>
          </span>
          <kbd>Alpha</kbd>
        </div>
      </div>
    </aside>
  );
}
