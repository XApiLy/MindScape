import React from 'react'
import { CanvasNode } from '../types'
import { Layers, ChevronRight, Sparkles, Compass, Eye, Grid } from 'lucide-react'

interface DepthProgressBarProps {
  nodes: CanvasNode[]
  currentNodeId: string
  parentStack: CanvasNode[]
  onSelectNode: (nodeId: string) => void
  viewMode: 'focused_stack' | 'macro_canvas'
  onToggleViewMode: () => void
}

export const DepthProgressBar: React.FC<DepthProgressBarProps> = ({
  nodes,
  currentNodeId,
  parentStack,
  onSelectNode,
  viewMode,
  onToggleViewMode,
}) => {
  // Construct the full learning depth trajectory for the progress track
  const currentNode = nodes.find((n) => n.id === currentNodeId)
  const fullTrajectory = [...parentStack, ...(currentNode ? [currentNode] : [])]

  return (
    <div className="fixed bottom-22 left-1/2 -translate-x-1/2 z-30 w-full max-w-2xl px-4">
      <div className="glass-panel rounded-xl px-3 py-1.5 border border-white/10 shadow-2xl flex items-center justify-between text-xs gap-3">
        {/* Left: Mode Status Indicator */}
        <button
          onClick={onToggleViewMode}
          className={`flex items-center gap-1.5 px-2.5 py-1 rounded-lg border text-[11px] font-mono font-medium transition ${
            viewMode === 'focused_stack'
              ? 'bg-amber-500/20 text-amber-300 border-amber-500/40 shadow-sm'
              : 'bg-cyan-500/20 text-cyan-300 border-cyan-500/40 shadow-sm'
          }`}
          title="点击在“沉浸堆叠专注”与“宏观网格图谱”之间切换 (⌘+Tab)"
        >
          {viewMode === 'focused_stack' ? (
            <>
              <Layers className="w-3.5 h-3.5 text-amber-400" />
              <span>🎯 沉浸堆叠模式</span>
            </>
          ) : (
            <>
              <Grid className="w-3.5 h-3.5 text-cyan-400" />
              <span>🌐 宏观图谱模式</span>
            </>
          )}
        </button>

        {/* Center: Micro-Macro Stack Trajectory Progress Track */}
        <div className="flex-1 flex items-center gap-1.5 overflow-x-auto py-0.5 no-scrollbar">
          {fullTrajectory.map((node, index) => {
            const isActive = node.id === currentNodeId
            return (
              <React.Fragment key={node.id}>
                {index > 0 && <ChevronRight className="w-3 h-3 text-gray-600 shrink-0" />}
                <button
                  onClick={() => onSelectNode(node.id)}
                  className={`flex items-center gap-1 px-2 py-0.5 rounded-md text-[11px] font-mono shrink-0 transition ${
                    isActive
                      ? 'bg-cyan-500/30 text-cyan-200 border border-cyan-400/50 font-bold shadow-sm shadow-cyan-500/20'
                      : 'text-gray-400 hover:text-gray-200 hover:bg-white/5 border border-transparent'
                  }`}
                  title={`深度层级 ${index + 1}: ${node.title}`}
                >
                  <span>#{node.id}</span>
                  <span className="max-w-[70px] truncate hidden sm:inline">{node.title}</span>
                </button>
              </React.Fragment>
            )
          })}
        </div>

        {/* Right: Depth Counter */}
        <div className="text-[10px] font-mono text-gray-400 shrink-0 bg-black/40 px-2 py-0.5 rounded border border-white/5">
          深度 <span className="text-cyan-400 font-bold">{fullTrajectory.length}</span>/
          {nodes.length}
        </div>
      </div>
    </div>
  )
}
