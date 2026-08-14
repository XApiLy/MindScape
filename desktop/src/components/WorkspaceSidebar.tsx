import {
  ChevronLeft,
  ChevronRight,
  Database,
  MessageSquare,
  Plus,
  Settings2,
} from "lucide-react";
import type { ConversationSummary, Workspace } from "../domain";

type WorkspaceSidebarProps = {
  open: boolean;
  workspace: Workspace;
  conversations: ConversationSummary[];
  selectedConversationId: string | null;
  onToggle: () => void;
  onCreateConversation: () => void;
  onSelectConversation: (conversationId: string) => void;
  onOpenSettings: () => void;
};

export function WorkspaceSidebar({
  open,
  workspace,
  conversations,
  selectedConversationId,
  onToggle,
  onCreateConversation,
  onSelectConversation,
  onOpenSettings,
}: WorkspaceSidebarProps) {
  if (!open) {
    return (
      <button className="sidebar-restore" type="button" onClick={onToggle} aria-label="展开会话侧栏">
        <ChevronRight aria-hidden="true" />
      </button>
    );
  }

  return (
    <aside className="workspace-sidebar" aria-label="工作区和会话">
      <div className="sidebar-header">
        <div className="brand-lockup">
          <span className="brand-mark" aria-hidden="true">M</span>
          <div>
            <strong>MindScape</strong>
            <span>Conversation workspace</span>
          </div>
        </div>
        <button className="icon-button" type="button" onClick={onToggle} aria-label="折叠会话侧栏">
          <ChevronLeft aria-hidden="true" />
        </button>
      </div>

      <button className="new-conversation-button" type="button" onClick={onCreateConversation}>
        <Plus aria-hidden="true" />
        新建会话
      </button>

      <div className="workspace-label">
        <Database aria-hidden="true" />
        <div>
          <span>本地工作区</span>
          <strong>{workspace.name}</strong>
        </div>
      </div>

      <div className="sidebar-section-label">最近会话</div>
      <nav className="conversation-list" aria-label="最近会话">
        {conversations.length === 0 ? (
          <p className="sidebar-empty">还没有会话，从一个问题开始。</p>
        ) : (
          conversations.map((conversation) => {
            const active = conversation.id === selectedConversationId;
            return (
              <button
                className={`conversation-row${active ? " is-active" : ""}`}
                key={conversation.id}
                type="button"
                onClick={() => onSelectConversation(conversation.id)}
              >
                <MessageSquare aria-hidden="true" />
                <span>
                  <strong>{conversation.title}</strong>
                  <small>{conversation.nodeCount} 个节点</small>
                </span>
              </button>
            );
          })
        )}
      </nav>

      <div className="sidebar-footer">
        <span className="engine-indicator" aria-hidden="true" />
        <span>
          <strong>本地内核</strong>
          <small>SQLite · schema v1</small>
        </span>
        <button className="icon-button" type="button" onClick={onOpenSettings} aria-label="打开模型设置">
          <Settings2 aria-hidden="true" />
        </button>
      </div>
    </aside>
  );
}
