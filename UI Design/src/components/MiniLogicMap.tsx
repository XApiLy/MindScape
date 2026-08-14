import React, { useMemo } from 'react'
import { GitBranch, Route } from 'lucide-react'
import { CanvasNode } from '../types'

interface MiniLogicMapProps {
  nodes: CanvasNode[]
  currentNodeId: string
  mainlineRootId: string
  onSelectNode: (nodeId: string) => void
  onExpandMacro?: () => void
}

const ROW_Y = 70
const BRANCH_Y = 24
const DOT_R = 9
const START_X = 20
const GAP = 38

const branchAccent: Record<string, string> = {
  sub_card: '#cba86a', // 深挖 - sand
  divergent_card: '#7fae8e', // 发散 - sage
  branch_card: '#b09bd0', // 换角 - mauve
}

const MAINLINE = '#7fae8e'
const TRACK = '#26262a'

const branchLabel: Record<string, string> = {
  sub_card: '深挖',
  divergent_card: '发散',
  branch_card: '换角',
}

export const MiniLogicMap: React.FC<MiniLogicMapProps> = ({
  nodes,
  currentNodeId,
  mainlineRootId,
  onSelectNode,
  onExpandMacro,
}) => {
  // 1. 计算主干序列：从主线起点出发，沿标记为主线的子节点串起主干
  const mainline = useMemo(() => {
    const chain: CanvasNode[] = []
    let curr = nodes.find((n) => n.id === mainlineRootId) || nodes.find((n) => n.isMainline)
    const guard = new Set<string>()
    while (curr && !guard.has(curr.id)) {
      chain.push(curr)
      guard.add(curr.id)
      curr = nodes.find((n) => n.parentId === curr!.id && n.isMainline)
    }
    return chain
  }, [nodes, mainlineRootId])

  const mainlineIndex = useMemo(
    () => new Map(mainline.map((n, i) => [n.id, i])),
    [mainline],
  )

  const currentNode = nodes.find((n) => n.id === currentNodeId)
  const currentIsMainline = currentNode ? mainlineIndex.has(currentNode.id) : false

  // 2. 当前若为分支：找到它挂接的主干父节点，作为轨道上的锚点
  const anchorParent = useMemo(() => {
    if (!currentNode || currentIsMainline) return null
    let p = nodes.find((n) => n.id === currentNode.parentId)
    const guard = new Set<string>()
    while (p && !guard.has(p.id)) {
      if (mainlineIndex.has(p.id)) return p
      guard.add(p.id)
      p = nodes.find((n) => n.id === p!.parentId)
    }
    return null
  }, [currentNode, currentIsMainline, nodes, mainlineIndex])

  const reachedIndex = currentIsMainline
    ? mainlineIndex.get(currentNodeId)!
    : anchorParent
      ? mainlineIndex.get(anchorParent.id)!
      : -1

  const svgWidth = Math.max(START_X + mainline.length * GAP + 24, 240)
  const cx = (i: number) => START_X + i * GAP + DOT_R

  const anchorIdx = anchorParent ? mainlineIndex.get(anchorParent.id)! : -1
  const branchCx = anchorIdx >= 0 ? cx(anchorIdx) + GAP * 0.55 : svgWidth - 60
  const branchColor = currentNode ? branchAccent[currentNode.type] ?? '#e5e7eb' : '#e5e7eb'

  return (
    <div className="rounded-lg bg-[#0f0f11] border border-white/8 px-3 pt-2 pb-1">
      {/* 状态标题行 */}
      <div className="flex items-center justify-between mb-1">
        <div className="flex items-center gap-1.5 text-[10px] font-mono tracking-wide">
          <Route className="w-3 h-3 text-gray-500" />
          {currentIsMainline ? (
            <span className="text-gray-400">当前位于主线</span>
          ) : (
            <span className="text-gray-400">
              当前位于分支「{currentNode ? branchLabel[currentNode.type] ?? '分支' : '分支'}」
            </span>
          )}
        </div>
        {onExpandMacro && (
          <button
            onClick={onExpandMacro}
            className="flex items-center gap-1 text-[10px] font-mono text-gray-600 hover:text-gray-300 transition"
            title="在宏观图谱中查看完整脉络"
          >
            <GitBranch className="w-3 h-3" />
            <span className="hidden sm:inline">展开图谱</span>
          </button>
        )}
      </div>

      {/* 轨道 SVG */}
      <div className="w-full overflow-x-auto no-scrollbar">
        <svg width={svgWidth} height={96} viewBox={`0 0 ${svgWidth} 96`} className="block">
          {/* 主干连接横线 */}
          {mainline.length > 1 && (
            <line
              x1={cx(0)}
              y1={ROW_Y}
              x2={cx(mainline.length - 1)}
              y2={ROW_Y}
              stroke={TRACK}
              strokeWidth={2}
              strokeLinecap="round"
            />
          )}
          {/* 已走过的主干段 */}
          {reachedIndex > 0 && (
            <line
              x1={cx(0)}
              y1={ROW_Y}
              x2={cx(reachedIndex)}
              y2={ROW_Y}
              stroke={MAINLINE}
              strokeWidth={2}
              strokeLinecap="round"
              opacity={0.6}
            />
          )}

          {/* 分支贝塞尔挂接线 */}
          {!currentIsMainline && anchorIdx >= 0 && (
            <path
              d={`M ${branchCx} ${BRANCH_Y + DOT_R} C ${branchCx} ${BRANCH_Y + 34}, ${cx(
                anchorIdx,
              )} ${ROW_Y - 30}, ${cx(anchorIdx)} ${ROW_Y - DOT_R}`}
              fill="none"
              stroke={branchColor}
              strokeWidth={1.75}
              strokeLinecap="round"
              opacity={0.7}
            />
          )}

          {/* 主干节点 */}
          {mainline.map((n, i) => {
            const isCurrent = currentIsMainline && n.id === currentNodeId
            const visited = i <= reachedIndex
            return (
              <g key={n.id} onClick={() => onSelectNode(n.id)} style={{ cursor: 'pointer' }}>
                <title>{n.title}</title>
                <circle
                  cx={cx(i)}
                  cy={ROW_Y}
                  r={DOT_R}
                  fill={isCurrent ? MAINLINE : '#141416'}
                  stroke={isCurrent ? MAINLINE : visited ? MAINLINE : '#3a3a40'}
                  strokeWidth={2}
                  opacity={visited || isCurrent ? 1 : 0.7}
                />
                {visited && !isCurrent && <circle cx={cx(i)} cy={ROW_Y} r={3} fill={MAINLINE} />}
                {isCurrent && <circle cx={cx(i)} cy={ROW_Y} r={3.2} fill="#0f0f11" />}
                <circle cx={cx(i)} cy={ROW_Y} r={DOT_R + 6} fill="transparent" />
              </g>
            )
          })}

          {/* 当前分支节点 */}
          {!currentIsMainline && anchorIdx >= 0 && currentNode && (
            <g onClick={() => onSelectNode(currentNode.id)} style={{ cursor: 'pointer' }}>
              <title>{currentNode.title}</title>
              <circle
                cx={branchCx}
                cy={BRANCH_Y}
                r={DOT_R}
                fill={branchColor}
                stroke={branchColor}
                strokeWidth={2}
              />
              <circle cx={branchCx} cy={BRANCH_Y} r={3.2} fill="#0f0f11" />
            </g>
          )}
        </svg>
      </div>
    </div>
  )
}
