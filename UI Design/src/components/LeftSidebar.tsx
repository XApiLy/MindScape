import React, { useState } from 'react'
import {
  ChevronLeft,
  ChevronDown,
  Layers,
  Grid,
  Plus,
  Folder,
  FolderOpen,
  MessageSquare,
} from 'lucide-react'
import { Project } from '../types'

interface LeftSidebarProps {
  isOpen: boolean
  onToggle: () => void
  projects: Project[]
  currentProjectId: string
  currentConversationId: string
  onSelectConversation: (projectId: string, conversationId: string) => void
  onNewChat: () => void
  viewMode: 'focused_stack' | 'macro_canvas'
  onToggleViewMode: () => void
}

export const LeftSidebar: React.FC<LeftSidebarProps> = ({
  isOpen,
  onToggle,
  projects,
  currentProjectId,
  currentConversationId,
  onSelectConversation,
  onNewChat,
  viewMode,
  onToggleViewMode,
}) => {
  const [expanded, setExpanded] = useState<string[]>([currentProjectId])

  const toggleExpand = (id: string) =>
    setExpanded((prev) => (prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]))

  if (!isOpen) {
    return null
  }

  return (
    <aside className="w-60 h-full rounded-2xl bg-[#0e0e10] border border-white/8 raise-1 flex flex-col justify-between p-3 select-none z-30 shrink-0 relative overflow-hidden animate-in slide-in-from-left duration-200">
      {/* Top Header: Wordmark + Collapse */}
      <div className="flex flex-col min-h-0 flex-1">
        <div className="flex items-center justify-between pb-3 mb-3 border-b border-white/8">
          <div className="flex items-center gap-2.5">
            <div className="w-6 h-6 rounded-md border border-white/15 flex items-center justify-center">
              <div className="w-2 h-2 rounded-[2px] bg-[#cba86a]" />
            </div>
            <span className="font-semibold text-gray-100 text-sm tracking-tight">LiquidFlow</span>
          </div>

          <button
            onClick={onToggle}
            className="p-1 rounded-md text-gray-500 hover:text-gray-200 hover:bg-white/5 transition"
            title="折叠侧边栏"
          >
            <ChevronLeft className="w-4 h-4" />
          </button>
        </div>

        {/* New Conversation Primary CTA */}
        <button
          onClick={onNewChat}
          className="w-full py-2 px-3 rounded-lg bg-white/[0.06] hover:bg-white/[0.1] text-gray-100 border border-white/10 text-xs font-medium flex items-center justify-center gap-2 transition mb-4"
        >
          <Plus className="w-4 h-4 text-gray-400" />
          <span>新建学习会话</span>
        </button>

        {/* View Switcher */}
        <div className="space-y-0.5 mb-4">
          <button
            onClick={() => viewMode !== 'focused_stack' && onToggleViewMode()}
            className={`w-full text-left px-2.5 py-1.5 rounded-md text-xs flex items-center gap-2.5 transition ${
              viewMode === 'focused_stack'
                ? 'bg-white/[0.07] text-gray-100 font-medium'
                : 'text-gray-500 hover:bg-white/[0.04] hover:text-gray-300'
            }`}
          >
            <Layers className={`w-4 h-4 ${viewMode === 'focused_stack' ? 'text-[#cba86a]' : 'text-gray-600'}`} />
            <span>沉浸堆叠学习</span>
          </button>

          <button
            onClick={() => viewMode !== 'macro_canvas' && onToggleViewMode()}
            className={`w-full text-left px-2.5 py-1.5 rounded-md text-xs flex items-center gap-2.5 transition ${
              viewMode === 'macro_canvas'
                ? 'bg-white/[0.07] text-gray-100 font-medium'
                : 'text-gray-500 hover:bg-white/[0.04] hover:text-gray-300'
            }`}
          >
            <Grid className={`w-4 h-4 ${viewMode === 'macro_canvas' ? 'text-[#cba86a]' : 'text-gray-600'}`} />
            <span>本会话宏观图谱</span>
          </button>
        </div>

        {/* Projects → Conversations tree */}
        <div className="px-2 py-1 text-[10px] font-mono text-gray-600 uppercase tracking-[0.12em]">
          学习项目 / 会话
        </div>

        <div className="space-y-0.5 mt-1 overflow-y-auto flex-1 min-h-0 pr-0.5 scrollbar-thin">
          {projects.map((p) => {
            const isOpenProj = expanded.includes(p.id)
            return (
              <div key={p.id}>
                {/* Project Row */}
                <button
                  onClick={() => toggleExpand(p.id)}
                  className={`w-full text-left px-2 py-1.5 rounded-md text-xs flex items-center gap-1.5 transition ${
                    p.id === currentProjectId
                      ? 'text-gray-100'
                      : 'text-gray-400 hover:bg-white/[0.04] hover:text-gray-200'
                  }`}
                >
                  <ChevronDown
                    className={`w-3 h-3 text-gray-600 shrink-0 transition-transform ${
                      isOpenProj ? '' : '-rotate-90'
                    }`}
                  />
                  {isOpenProj ? (
                    <FolderOpen className="w-3.5 h-3.5 text-gray-400 shrink-0" />
                  ) : (
                    <Folder className="w-3.5 h-3.5 text-gray-600 shrink-0" />
                  )}
                  <span className="truncate flex-1 font-medium">{p.name}</span>
                  <span className="text-[9px] font-mono text-gray-600 shrink-0">
                    {p.conversations.length}
                  </span>
                </button>

                {/* Conversation Children */}
                {isOpenProj && (
                  <div className="ml-3.5 pl-2 border-l border-white/8 space-y-0.5 mt-0.5 mb-1">
                    {p.conversations.map((c) => {
                      const active = c.id === currentConversationId
                      return (
                        <button
                          key={c.id}
                          onClick={() => onSelectConversation(p.id, c.id)}
                          className={`w-full text-left px-2 py-1.5 rounded-md text-xs flex items-center gap-2 transition relative ${
                            active
                              ? 'bg-white/[0.07] text-gray-100 font-medium'
                              : 'text-gray-500 hover:bg-white/[0.04] hover:text-gray-300'
                          }`}
                        >
                          {active && (
                            <span className="absolute -left-[9px] top-1/2 -translate-y-1/2 w-[2px] h-4 rounded-full bg-[#cba86a]" />
                          )}
                          <MessageSquare
                            className={`w-3 h-3 shrink-0 ${
                              active ? 'text-gray-300' : 'text-gray-600'
                            }`}
                          />
                          <span className="truncate flex-1">{c.name}</span>
                          <span className="text-[9px] font-mono text-gray-600 shrink-0">
                            {c.updatedAt}
                          </span>
                        </button>
                      )
                    })}
                  </div>
                )}
              </div>
            )
          })}
        </div>
      </div>

      {/* Footer: Engine status */}
      <div className="pt-3 mt-2 border-t border-white/8 flex items-center justify-between text-xs shrink-0">
        <div className="flex items-center gap-2">
          <span className="w-1.5 h-1.5 rounded-full bg-[#7fae8e]" />
          <div className="text-left">
            <div className="text-[11px] font-medium text-gray-300">Gateway v2.4</div>
            <div className="text-[9px] text-gray-600 font-mono">Engine online</div>
          </div>
        </div>

        <span className="px-1.5 py-0.5 text-[9px] font-mono bg-white/5 text-gray-500 rounded border border-white/8">
          Pro
        </span>
      </div>
    </aside>
  )
}
