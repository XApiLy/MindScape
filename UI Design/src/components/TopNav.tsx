import React, { useState } from 'react'
import {
  ChevronDown,
  Search,
  Focus,
  Plus,
  Minus,
  SlidersHorizontal,
  FolderOpen,
  CheckCircle2,
  PanelRight,
  PanelLeft,
  Layers,
  Grid,
} from 'lucide-react'
import { Project } from '../types'

interface TopNavProps {
  currentProject: Project
  projects: Project[]
  onSelectProject: (proj: Project) => void
  zoom: number
  onZoomIn: () => void
  onZoomOut: () => void
  onResetZoom: () => void
  onOpenCmdK: () => void
  onToggleDrawer: () => void
  isDrawerOpen: boolean
  highlightCount: number
  onOpenSettings: () => void
  viewMode: 'focused_stack' | 'macro_canvas'
  onToggleViewMode: () => void
  isLeftSidebarOpen?: boolean
  onToggleLeftSidebar?: () => void
}

export const TopNav: React.FC<TopNavProps> = ({
  currentProject,
  projects,
  onSelectProject,
  zoom,
  onZoomIn,
  onZoomOut,
  onResetZoom,
  onOpenCmdK,
  onToggleDrawer,
  isDrawerOpen,
  highlightCount,
  onOpenSettings,
  viewMode,
  onToggleViewMode,
  isLeftSidebarOpen,
  onToggleLeftSidebar,
}) => {
  const [showProjectDropdown, setShowProjectDropdown] = useState(false)

  return (
    <header className="w-full h-12 flex items-center justify-between px-3 bg-[#111113] rounded-lg border border-white/8 text-sm">
      {/* Left: Sidebar Toggle + Brand + Project Switcher */}
      <div className="flex items-center gap-2">
        {onToggleLeftSidebar && (
          <button
            onClick={onToggleLeftSidebar}
            className={`p-1.5 rounded-md transition ${
              isLeftSidebarOpen
                ? 'bg-white/[0.07] text-gray-200'
                : 'text-gray-500 hover:text-gray-200 hover:bg-white/5'
            }`}
            title={isLeftSidebarOpen ? '折叠侧边栏' : '展开侧边栏'}
          >
            <PanelLeft className="w-4 h-4" />
          </button>
        )}

        <div className="flex items-center gap-2 pr-2 mr-1 border-r border-white/8">
          <div className="w-6 h-6 rounded-md border border-white/15 flex items-center justify-center">
            <div className="w-2 h-2 rounded-[2px] bg-[#cba86a]" />
          </div>
          <span className="font-semibold text-gray-100 tracking-tight text-[15px]">LiquidFlow</span>
        </div>

        {/* Project Switcher */}
        <div className="relative">
          <button
            onClick={() => setShowProjectDropdown(!showProjectDropdown)}
            className="flex items-center gap-2 px-2.5 py-1 rounded-md hover:bg-white/5 text-gray-300 hover:text-gray-100 transition group text-xs font-medium"
          >
            <FolderOpen className="w-3.5 h-3.5 text-gray-500" />
            <span className="max-w-[180px] truncate">{currentProject.name}</span>
            <ChevronDown className="w-3.5 h-3.5 text-gray-500" />
          </button>

          {showProjectDropdown && (
            <div className="absolute left-0 top-full mt-2 w-72 bg-[#161618] rounded-lg p-1.5 border border-white/12 shadow-xl shadow-black/40 z-50 animate-in fade-in slide-in-from-top-2">
              <div className="px-2 py-1.5 text-[10px] font-mono text-gray-600 uppercase tracking-[0.12em] border-b border-white/8">
                切换学习项目
              </div>
              <div className="py-1 space-y-0.5">
                {projects.map((p) => (
                  <button
                    key={p.id}
                    onClick={() => {
                      onSelectProject(p)
                      setShowProjectDropdown(false)
                    }}
                    className={`w-full text-left px-2.5 py-2 rounded-md text-xs flex items-center justify-between transition ${
                      p.id === currentProject.id
                        ? 'bg-white/[0.07] text-gray-100'
                        : 'text-gray-400 hover:bg-white/[0.04] hover:text-gray-200'
                    }`}
                  >
                    <div className="truncate pr-2">
                      <div className="font-medium truncate">{p.name}</div>
                      <div className="text-[10px] text-gray-500 truncate">{p.description}</div>
                    </div>
                    {p.id === currentProject.id && (
                      <CheckCircle2 className="w-4 h-4 text-[#cba86a] shrink-0" />
                    )}
                  </button>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Center: Mode Switcher Segment + Zoom / Cmd+K */}
      <div className="flex items-center gap-3">
        {/* Primary View Mode Switcher Segment */}
        <div className="flex items-center bg-black/30 rounded-md p-0.5 border border-white/8">
          <button
            onClick={() => viewMode !== 'focused_stack' && onToggleViewMode()}
            className={`flex items-center gap-1.5 px-3 py-1 rounded text-xs transition ${
              viewMode === 'focused_stack'
                ? 'bg-white/[0.08] text-gray-100 font-medium'
                : 'text-gray-500 hover:text-gray-300'
            }`}
          >
            <Layers className="w-3.5 h-3.5" />
            <span>沉浸专注堆叠</span>
          </button>

          <button
            onClick={() => viewMode !== 'macro_canvas' && onToggleViewMode()}
            className={`flex items-center gap-1.5 px-3 py-1 rounded text-xs transition ${
              viewMode === 'macro_canvas'
                ? 'bg-white/[0.08] text-gray-100 font-medium'
                : 'text-gray-500 hover:text-gray-300'
            }`}
          >
            <Grid className="w-3.5 h-3.5" />
            <span>宏观网格图谱</span>
          </button>
        </div>

        {/* Canvas Zoom controls (Visible in Canvas mode) */}
        {viewMode === 'macro_canvas' && (
          <div className="hidden md:flex items-center bg-black/30 rounded-md p-0.5 border border-white/8">
            <button
              onClick={onZoomOut}
              className="p-1 text-gray-500 hover:text-gray-200 hover:bg-white/5 rounded transition"
              title="缩小"
            >
              <Minus className="w-3.5 h-3.5" />
            </button>
            <span className="px-2 font-mono text-xs text-gray-300 min-w-[42px] text-center">
              {Math.round(zoom * 100)}%
            </span>
            <button
              onClick={onZoomIn}
              className="p-1 text-gray-500 hover:text-gray-200 hover:bg-white/5 rounded transition"
              title="放大"
            >
              <Plus className="w-3.5 h-3.5" />
            </button>
            <button
              onClick={onResetZoom}
              className="p-1 ml-0.5 text-gray-500 hover:text-gray-200 hover:bg-white/5 rounded transition"
              title="居中重置"
            >
              <Focus className="w-3.5 h-3.5" />
            </button>
          </div>
        )}

        {/* Cmd + K Search Trigger */}
        <button
          onClick={onOpenCmdK}
          className="flex items-center gap-2 px-3 py-1 rounded-md bg-black/30 hover:bg-white/5 border border-white/8 text-gray-500 hover:text-gray-300 text-xs transition"
        >
          <Search className="w-3.5 h-3.5" />
          <span className="hidden sm:inline">搜索</span>
          <kbd className="px-1.5 py-0.5 text-[10px] font-mono bg-white/[0.06] text-gray-400 rounded border border-white/8">
            ⌘K
          </kbd>
        </button>
      </div>

      {/* Right: Drawer Toggle & Settings */}
      <div className="flex items-center gap-3">
        {/* Settings button */}
        <button
          onClick={onOpenSettings}
          className="p-1.5 rounded-md text-gray-500 hover:text-gray-200 hover:bg-white/5 transition"
          title="系统设置"
        >
          <SlidersHorizontal className="w-4 h-4" />
        </button>

        {/* Highlights Drawer Toggle */}
        <button
          onClick={onToggleDrawer}
          className={`flex items-center gap-1.5 px-2.5 py-1 rounded-md border text-xs transition ${
            isDrawerOpen
              ? 'bg-white/[0.08] text-gray-100 border-white/15'
              : 'bg-black/30 text-gray-400 border-white/8 hover:bg-white/5 hover:text-gray-200'
          }`}
        >
          <PanelRight className="w-3.5 h-3.5" />
          <span className="font-medium hidden sm:inline">重点收敛</span>
          {highlightCount > 0 && (
            <span className="px-1.5 py-0.5 rounded-full bg-[#cba86a] text-black font-semibold text-[10px] leading-none">
              {highlightCount}
            </span>
          )}
        </button>
      </div>
    </header>
  )
}
