import React, { useMemo } from 'react'
import { Route, GitBranch, CornerUpLeft, Maximize2 } from 'lucide-react'
import { CanvasNode } from '../types'
import { MiniLogicMap } from './MiniLogicMap'

interface TrajectoryRailProps {
  nodes: CanvasNode[]
  currentNodeId: string
  mainlineRootId: string
  isOnBranch: boolean
  onSelectNode: (nodeId: string) => void
  onExpandMacro: () => void
  onReturnToMainline: () => void
}

export const TrajectoryRail: React.FC<TrajectoryRailProps> = ({
  nodes,
  currentNodeId,
  mainlineRootId,
  isOnBranch,
  onSelectNode,
  onExpandMacro,
  onReturnToMainline,
}) => {
  const stats = useMemo(() => {
    const mainlineCount = nodes.filter((n) => n.isMainline).length
    const branchCount = nodes.length - mainlineCount
    return { total: nodes.length, mainlineCount, branchCount }
  }, [nodes])

  return (
    <aside className="hidden lg:flex flex-col w-[320px] shrink-0 h-full py-3 pr-3">
      <div className="flex-1 bg-[#111113] rounded-xl border border-white/8 flex flex-col overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-white/8">
          <div className="flex items-center gap-2">
            <Route className="w-4 h-4 text-gray-500" />
            <span className="text-sm font-medium text-gray-100 tracking-tight">会话逻辑轨迹</span>
          </div>
          <span
            className="text-[10px] font-mono px-2 py-0.5 rounded border border-white/8 bg-white/[0.04] flex items-center gap-1.5"
          >
            <span
              className="w-1.5 h-1.5 rounded-full"
              style={{ background: isOnBranch ? '#b09bd0' : '#7fae8e' }}
            />
            <span className="text-gray-400">{isOnBranch ? '分支线' : '主线'}</span>
          </span>
        </div>

        {/* Mini logic map (bezier glow) */}
        <div className="px-3 pt-3">
          <MiniLogicMap
            nodes={nodes}
            currentNodeId={currentNodeId}
            mainlineRootId={mainlineRootId}
            onSelectNode={onSelectNode}
          />
        </div>

        {/* Stats */}
        <div className="grid grid-cols-3 gap-2 px-3 pt-3">
          {[
            { label: '总节点', value: stats.total, dot: '#8a8a90' },
            { label: '主线', value: stats.mainlineCount, dot: '#7fae8e' },
            { label: '分支', value: stats.branchCount, dot: '#b09bd0' },
          ].map((s) => (
            <div
              key={s.label}
              className="rounded-lg bg-white/[0.03] border border-white/8 px-2 py-2.5 text-center"
            >
              <div className="text-lg font-semibold font-mono text-gray-100">{s.value}</div>
              <div className="text-[10px] font-mono text-gray-500 mt-1 flex items-center justify-center gap-1">
                <span className="w-1 h-1 rounded-full" style={{ background: s.dot }} />
                {s.label}
              </div>
            </div>
          ))}
        </div>

        {/* Spacer pushes actions to bottom */}
        <div className="flex-1" />

        {/* Actions */}
        <div className="px-3 pb-3 space-y-2 border-t border-white/8 pt-3">
          {isOnBranch && (
            <button
              onClick={onReturnToMainline}
              className="w-full py-2 rounded-lg bg-[#7fae8e]/12 hover:bg-[#7fae8e]/20 text-[#7fae8e] border border-[#7fae8e]/30 text-xs font-medium flex items-center justify-center gap-1.5 transition"
            >
              <CornerUpLeft className="w-3.5 h-3.5" />
              回到主线继续推进
            </button>
          )}
          <button
            onClick={onExpandMacro}
            className="w-full py-2 rounded-lg bg-white/[0.04] hover:bg-white/[0.08] text-gray-300 hover:text-gray-100 border border-white/8 text-xs font-medium flex items-center justify-center gap-1.5 transition"
          >
            <Maximize2 className="w-3.5 h-3.5 text-gray-500" />
            在宏观图谱查看完整脉络
          </button>
          <div className="flex items-center justify-center gap-1.5 text-[10px] font-mono text-gray-600 pt-0.5">
            <GitBranch className="w-3 h-3" />
            <span>点击轨道节点可快速跳转</span>
          </div>
        </div>
      </div>
    </aside>
  )
}
