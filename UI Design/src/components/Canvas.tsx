import React, { useState, useRef, useEffect, MouseEvent } from 'react'
import {
  Sparkles,
  Bot,
  Copy,
  Check,
  X,
  Plus,
  CornerDownRight,
  GitFork,
  Maximize2,
  Minimize2,
  Trash2,
  ArrowUpRight,
  MoveRight,
  ArrowDown,
  Tag,
  Share2,
} from 'lucide-react'
import { CanvasNode, Edge, ModelType, NodeType } from '../types'
import { safeCopyText } from '../utils/clipboard'

interface CanvasProps {
  nodes: CanvasNode[]
  edges: Edge[]
  zoom: number
  pan: { x: number; y: number }
  onPanChange: (pan: { x: number; y: number }) => void
  onUpdateNodePos: (id: string, x: number, y: number) => void
  onDeleteNode: (id: string) => void
  onBranchCard: (parentId: string, branchType: 'sub' | 'divergent' | 'branch') => void
  onTextSelect: (nodeId: string, text: string, clientPos: { x: number; y: number }) => void
  focusedNodeId: string | null
}

export const Canvas: React.FC<CanvasProps> = ({
  nodes,
  edges,
  zoom,
  pan,
  onPanChange,
  onUpdateNodePos,
  onDeleteNode,
  onBranchCard,
  onTextSelect,
  focusedNodeId,
}) => {
  const containerRef = useRef<HTMLDivElement>(null)
  const [isPanning, setIsPanning] = useState(false)
  const [startPan, setStartPan] = useState({ x: 0, y: 0 })
  const [draggingNodeId, setDraggingNodeId] = useState<string | null>(null)
  const [dragOffset, setDraggingOffset] = useState({ x: 0, y: 0 })
  const [copiedNodeId, setCopiedNodeId] = useState<string | null>(null)
  const [collapsedNodes, setCollapsedNodes] = useState<Record<string, boolean>>({})

  // Handle Pan dragging
  const handleMouseDown = (e: MouseEvent<HTMLDivElement>) => {
    // Only pan if clicking on empty canvas ground
    if ((e.target as HTMLElement).closest('.canvas-card')) return

    setIsPanning(true)
    setStartPan({ x: e.clientX - pan.x, y: e.clientY - pan.y })
  }

  const handleMouseMove = (e: MouseEvent<HTMLDivElement>) => {
    if (isPanning) {
      onPanChange({
        x: e.clientX - startPan.x,
        y: e.clientY - startPan.y,
      })
      return
    }

    if (draggingNodeId) {
      const newX = (e.clientX - pan.x - dragOffset.x) / zoom
      const newY = (e.clientY - pan.y - dragOffset.y) / zoom
      onUpdateNodePos(draggingNodeId, Math.round(newX), Math.round(newY))
    }
  }

  const handleMouseUp = () => {
    setIsPanning(false)
    setDraggingNodeId(null)
  }

  // Handle Card Header Dragging
  const handleStartDragCard = (e: MouseEvent<HTMLDivElement>, node: CanvasNode) => {
    e.stopPropagation()
    setDraggingNodeId(node.id)
    setDraggingOffset({
      x: e.clientX - (node.x * zoom + pan.x),
      y: e.clientY - (node.y * zoom + pan.y),
    })
  }

  // Text selection handler inside cards
  const handleTextMouseUp = (nodeId: string) => {
    const selection = window.getSelection()
    const selectedText = selection?.toString().trim()
    if (selectedText && selectedText.length > 1) {
      const range = selection?.getRangeAt(0)
      const rect = range?.getBoundingClientRect()
      if (rect) {
        onTextSelect(nodeId, selectedText, {
          x: rect.left + rect.width / 2,
          y: rect.top,
        })
      }
    }
  }

  const toggleCollapse = (nodeId: string) => {
    setCollapsedNodes((prev) => ({ ...prev, [nodeId]: !prev[nodeId] }))
  }

  const handleCopyNodeContent = async (node: CanvasNode) => {
    await safeCopyText(node.content)
    setCopiedNodeId(node.id)
    setTimeout(() => setCopiedNodeId(null), 1500)
  }

  // Helper to calculate smooth Bezier curve control points
  const renderBezierCurve = (edge: Edge) => {
    const sourceNode = nodes.find((n) => n.id === edge.source)
    const targetNode = nodes.find((n) => n.id === edge.target)
    if (!sourceNode || !targetNode) return null

    // Node bounds
    const sX = sourceNode.x + sourceNode.width
    const sY = sourceNode.y + 120
    const tX = targetNode.x
    const tY = targetNode.y + 120

    const dx = Math.abs(tX - sX) * 0.5
    const pathD = `M ${sX} ${sY} C ${sX + dx} ${sY}, ${tX - dx} ${tY}, ${tX} ${tY}`

    const isFocused = focusedNodeId === sourceNode.id || focusedNodeId === targetNode.id
    const isBranch = edge.type === 'branch'
    const baseStroke = isBranch ? '#b09bd0' : '#26262a'
    const emphasisStroke = isBranch ? '#b09bd0' : '#7fae8e'

    return (
      <g key={edge.id} className="transition-all duration-300">
        {/* Base track path */}
        <path
          d={pathD}
          fill="none"
          stroke={isFocused ? emphasisStroke : baseStroke}
          strokeWidth={isFocused ? '3' : '2'}
        />
        {/* Animated dashes */}
        <path
          d={pathD}
          fill="none"
          stroke={emphasisStroke}
          strokeWidth="1.5"
          className="animate-dash opacity-60"
        />
        {edge.label && (
          <text
            x={(sX + tX) / 2}
            y={(sY + tY) / 2 - 8}
            fill="#9ca3af"
            fontSize="10"
            fontFamily="monospace"
            textAnchor="middle"
            className="select-none"
          >
            {edge.label}
          </text>
        )}
      </g>
    )
  }

  return (
    <div
      ref={containerRef}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
      className="w-full h-screen bg-[#0b0b0c] relative overflow-hidden select-none cursor-grab active:cursor-grabbing"
      style={{
        backgroundImage: `radial-gradient(circle at center, rgba(255, 255, 255, 0.08) 1.2px, transparent 1.4px)`,
        backgroundSize: `${32 * zoom}px ${32 * zoom}px`,
        backgroundPosition: `${pan.x}px ${pan.y}px`,
      }}
    >
      {/* SVG Connections Layer */}
      <svg
        className="absolute inset-0 w-full h-full pointer-events-none z-10"
        style={{
          transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`,
          transformOrigin: '0 0',
        }}
      >
        {edges.map((edge) => renderBezierCurve(edge))}
      </svg>

      {/* Nodes Layer */}
      <div
        className="absolute inset-0 z-20 pointer-events-none"
        style={{
          transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`,
          transformOrigin: '0 0',
        }}
      >
        {nodes.map((node) => {
          const isCollapsed = collapsedNodes[node.id]
          const isFocused = focusedNodeId === node.id

          return (
            <div
              key={node.id}
              style={{
                left: `${node.x}px`,
                top: `${node.y}px`,
                width: `${node.width}px`,
              }}
              className={`canvas-card absolute pointer-events-auto rounded-lg bg-[#141416] p-4 border shadow-xl shadow-black/40 transition-colors duration-300 ${
                isFocused
                  ? 'border-[#cba86a]/60'
                  : 'border-white/10 hover:border-white/20'
              }`}
            >
              {/* Card Header (Draggable Handle) */}
              <div
                onMouseDown={(e) => handleStartDragCard(e, node)}
                className="flex items-center justify-between pb-3 border-b border-white/10 cursor-move"
              >
                <div className="flex items-center gap-2">
                  <span className="font-mono text-xs text-gray-300 font-bold bg-white/[0.06] px-2 py-0.5 rounded border border-white/10 flex items-center gap-1">
                    <span className="w-1.5 h-1.5 rounded-full bg-[#cba86a]" />
                    #{node.id}
                  </span>

                  {node.model && (
                    <span className="text-[10px] font-mono text-gray-400 bg-white/[0.04] px-2 py-0.5 rounded border border-white/8">
                      {node.model}
                    </span>
                  )}

                  {node.type === 'question' && (
                    <span className="text-[10px] font-mono text-[#b09bd0] bg-white/[0.04] px-2 py-0.5 rounded border border-white/8">
                      用户提问
                    </span>
                  )}
                </div>

                <div className="flex items-center gap-1">
                  <button
                    onClick={() => handleCopyNodeContent(node)}
                    className="p-1 rounded text-gray-400 hover:text-white hover:bg-white/10 transition"
                    title="复制卡片全文"
                  >
                    {copiedNodeId === node.id ? (
                      <Check className="w-3.5 h-3.5 text-[#7fae8e]" />
                    ) : (
                      <Copy className="w-3.5 h-3.5" />
                    )}
                  </button>

                  <button
                    onClick={() => toggleCollapse(node.id)}
                    className="p-1 rounded text-gray-400 hover:text-white hover:bg-white/10 transition"
                    title={isCollapsed ? '展开卡片' : '折叠卡片'}
                  >
                    {isCollapsed ? (
                      <Maximize2 className="w-3.5 h-3.5" />
                    ) : (
                      <Minimize2 className="w-3.5 h-3.5" />
                    )}
                  </button>

                  <button
                    onClick={() => onDeleteNode(node.id)}
                    className="p-1 rounded text-gray-400 hover:text-rose-400 hover:bg-white/10 transition"
                    title="删除卡片"
                  >
                    <X className="w-3.5 h-3.5" />
                  </button>
                </div>
              </div>

              {/* Card Title */}
              <div className="pt-2 font-semibold text-gray-100 text-sm flex items-center justify-between">
                <span>{node.title}</span>
                <span className="text-[10px] font-mono text-gray-500">{node.timestamp}</span>
              </div>

              {/* Card Body (Selection enabled) */}
              {!isCollapsed && (
                <div
                  onMouseUp={() => handleTextMouseUp(node.id)}
                  className="py-3 text-xs text-gray-200 leading-relaxed font-sans space-y-2 select-text"
                >
                  {node.content.split('\n\n').map((paragraph, idx) => {
                    if (paragraph.startsWith('```')) {
                      const codeContent = paragraph.replace(/```[a-z]*/g, '').trim()
                      return (
                        <div
                          key={idx}
                          className="my-2 p-2.5 rounded-md bg-[#0f0f11] border border-white/8 font-mono text-[11px] text-gray-300 overflow-x-auto relative group"
                        >
                          <div className="text-[9px] text-gray-500 mb-1 flex justify-between uppercase">
                            <span>CODE SNIPPET</span>
                            <span>COPY</span>
                          </div>
                          <pre>{codeContent}</pre>
                        </div>
                      )
                    }
                    return (
                      <p key={idx} className="whitespace-pre-wrap">
                        {paragraph}
                      </p>
                    )
                  })}

                  {/* Tags */}
                  {node.tags && node.tags.length > 0 && (
                    <div className="flex items-center gap-1.5 pt-2 flex-wrap">
                      {node.tags.map((tag) => (
                        <span
                          key={tag}
                          className="text-[10px] font-mono px-2 py-0.5 rounded-md bg-white/[0.04] text-gray-400 border border-white/8 flex items-center gap-1"
                        >
                          <Tag className="w-2.5 h-2.5 text-gray-500" />
                          {tag}
                        </span>
                      ))}
                    </div>
                  )}
                </div>
              )}

              {/* Card Footer: 3 Distinct Branching Action Buttons */}
              {!isCollapsed && (
                <div className="flex items-center gap-1.5 pt-3 mt-2 border-t border-white/10 text-xs font-medium">
                  <button
                    onClick={() => onBranchCard(node.id, 'sub')}
                    className="flex-1 py-1.5 px-2 rounded-md bg-white/[0.04] hover:bg-white/[0.08] text-gray-300 hover:text-gray-100 border border-white/8 transition flex items-center justify-center gap-1 text-[11px]"
                    title="针对局部细节进一步深挖，生成子卡片"
                  >
                    <ArrowUpRight className="w-3.5 h-3.5 text-gray-400" />
                    <span>深挖子卡片</span>
                  </button>

                  <button
                    onClick={() => onBranchCard(node.id, 'divergent')}
                    className="flex-1 py-1.5 px-2 rounded-md bg-white/[0.04] hover:bg-white/[0.08] text-gray-300 hover:text-gray-100 border border-white/8 transition flex items-center justify-center gap-1 text-[11px]"
                    title="同级扩展思维，平行发散节点"
                  >
                    <MoveRight className="w-3.5 h-3.5 text-[#7fae8e]" />
                    <span>平行发散</span>
                  </button>

                  <button
                    onClick={() => onBranchCard(node.id, 'branch')}
                    className="flex-1 py-1.5 px-2 rounded-md bg-white/[0.04] hover:bg-white/[0.08] text-gray-300 hover:text-gray-100 border border-white/8 transition flex items-center justify-center gap-1 text-[11px]"
                    title="继承历史上下文，换个全新角度切入"
                  >
                    <ArrowDown className="w-3.5 h-3.5 text-[#b09bd0]" />
                    <span>换角分支</span>
                  </button>
                </div>
              )}
            </div>
          )
        })}
      </div>
    </div>
  )
}
