import React, { useMemo, useState } from 'react'
import { Route, ChevronDown, CornerUpLeft, Maximize2 } from 'lucide-react'
import { CanvasNode } from '../types'
import { MiniLogicMap } from './MiniLogicMap'

interface TrajectoryDockProps {
  nodes: CanvasNode[]
  currentNodeId: string
  mainlineRootId: string
  isOnBranch: boolean
  onSelectNode: (nodeId: string) => void
  onExpandMacro: () => void
  onReturnToMainline: () => void
}

const branchLabel: Record<string, string> = {
  sub_card: '深挖',
  divergent_card: '发散',
  branch_card: '换角',
}

export const TrajectoryDock: React.FC<TrajectoryDockProps> = ({
  nodes,
  currentNodeId,
  mainlineRootId,
  isOnBranch,
  onSelectNode,
  onExpandMacro,
  onReturnToMainline,
}) => {
  const [open, setOpen] = useState(true)

  // Real depth of the current path (ancestors + self).
  const depth = useMemo(() => {
    let d = 1
    let curr = nodes.find((n) => n.id === currentNodeId)
    const guard = new Set<string>()
    while (curr?.parentId && !guard.has(curr.id)) {
      guard.add(curr.id)
      curr = nodes.find((n) => n.id === curr!.parentId)
      if (curr) d++
    }
    return d
  }, [nodes, currentNodeId])

  const currentNode = nodes.find((n) => n.id === currentNodeId)
  const statusText = isOnBranch
    ? `分支 · ${currentNode ? branchLabel[currentNode.type] ?? '分支' : '分支'}`
    : '主线'
  const statusColor = isOnBranch ? '#b09bd0' : '#7fae8e'

  return (
    <div className="absolute bottom-4 right-4 z-30 w-[300px] max-w-[calc(100%-2rem)]">
      <div className="rounded-xl bg-[#141416]/95 backdrop-blur-md border border-white/10 raise-1 overflow-hidden">
        {/* Header / collapse toggle */}
        <button
          onClick={() => setOpen((v) => !v)}
          className="w-full flex items-center justify-between px-3 py-2 hover:bg-white/[0.03] transition"
        >
          <span className="flex items-center gap-2 min-w-0">
            <Route className="w-3.5 h-3.5 text-gray-500 shrink-0" />
            <span className="text-xs font-medium text-gray-200 shrink-0">逻辑轨迹</span>
            <span className="flex items-center gap-1.5 min-w-0 pl-1">
              <span className="w-1.5 h-1.5 rounded-full shrink-0" style={{ background: statusColor }} />
              <span className="text-[11px] font-mono text-gray-500 truncate">{statusText}</span>
            </span>
          </span>
          <span className="flex items-center gap-2 shrink-0">
            <span className="text-[10px] font-mono text-gray-600">深度 {depth}</span>
            <ChevronDown
              className={`w-3.5 h-3.5 text-gray-500 transition-transform ${open ? '' : 'rotate-180'}`}
            />
          </span>
        </button>

        {/* Expanded body */}
        {open && (
          <div className="px-2.5 pb-2.5 pt-0.5 border-t border-white/6 animate-in fade-in slide-in-from-bottom-1 duration-150">
            <MiniLogicMap
              nodes={nodes}
              currentNodeId={currentNodeId}
              mainlineRootId={mainlineRootId}
              onSelectNode={onSelectNode}
            />

            <div className="flex items-center gap-1.5 mt-2">
              {isOnBranch && (
                <button
                  onClick={onReturnToMainline}
                  className="flex-1 py-1.5 rounded-lg bg-[#7fae8e]/12 hover:bg-[#7fae8e]/20 text-[#7fae8e] border border-[#7fae8e]/25 text-[11px] font-medium flex items-center justify-center gap-1.5 transition"
                  title="返回主干末端继续推进"
                >
                  <CornerUpLeft className="w-3.5 h-3.5" />
                  回到主线
                </button>
              )}
              <button
                onClick={onExpandMacro}
                className="flex-1 py-1.5 rounded-lg bg-white/[0.04] hover:bg-white/[0.08] text-gray-400 hover:text-gray-200 border border-white/8 text-[11px] font-medium flex items-center justify-center gap-1.5 transition"
                title="在宏观图谱查看完整脉络"
              >
                <Maximize2 className="w-3.5 h-3.5" />
                宏观图谱
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
