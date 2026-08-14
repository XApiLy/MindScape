import React, { useState, useRef } from 'react'
import {
  Loader2,
  ChevronDown,
  ChevronUp,
  Bookmark,
  Trash2,
  Maximize2,
  ArrowUpRight,
  MoveRight,
  ArrowDown,
  Tag,
  Clock,
  Send,
  Globe,
  Bot,
  Milestone,
  CornerUpLeft,
} from 'lucide-react'
import { CanvasNode, HighlightType, ModelType } from '../types'

interface FocusedStackViewProps {
  currentNode: CanvasNode
  parentNodesStack: CanvasNode[]
  childNodes: CanvasNode[]
  allNodes: CanvasNode[]
  mainlineRootId: string
  onSelectNode: (nodeId: string) => void
  onBranchCard: (parentId: string, branchType: 'sub' | 'divergent' | 'branch') => void
  onToggleMainline: (nodeId: string) => void
  onReturnToMainline: () => void
  onTextSelect: (nodeId: string, text: string, clientPos: { x: number; y: number }) => void
  onSwitchToMacro: () => void
  onDeleteNode: (nodeId: string) => void
  onQuickHighlightNode: (nodeId: string, type: HighlightType) => void
  selectedModel: ModelType
  onSelectModel: (m: ModelType) => void
  onSendMessage: (prompt: string, model: ModelType, webSearch: boolean) => void
  isGenerating: boolean
}

export const FocusedStackView: React.FC<FocusedStackViewProps> = ({
  currentNode,
  parentNodesStack,
  childNodes,
  allNodes,
  mainlineRootId,
  onSelectNode,
  onBranchCard,
  onToggleMainline,
  onReturnToMainline,
  onTextSelect,
  onSwitchToMacro,
  onDeleteNode,
  onQuickHighlightNode,
  selectedModel,
  onSelectModel,
  onSendMessage,
  isGenerating,
}) => {
  const [showReasoning, setShowReasoning] = useState(true)
  const [inCardPrompt, setInCardPrompt] = useState('')
  const [webSearch, setWebSearch] = useState(true)
  const [showModelDropdown, setShowModelDropdown] = useState(false)
  const textareaRef = useRef<HTMLTextAreaElement>(null)

  const handleSendInCard = () => {
    if (!inCardPrompt.trim() || isGenerating) return
    onSendMessage(inCardPrompt.trim(), selectedModel, webSearch)
    setInCardPrompt('')
  }

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSendInCard()
    }
  }

  const handleTextMouseUp = () => {
    const selection = window.getSelection()
    const selectedText = selection?.toString().trim()
    if (selectedText && selectedText.length > 1) {
      const range = selection?.getRangeAt(0)
      const rect = range?.getBoundingClientRect()
      if (rect) {
        onTextSelect(currentNode.id, selectedText, {
          x: rect.left + rect.width / 2,
          y: rect.top,
        })
      }
    }
  }

  const models: { id: ModelType; name: string }[] = [
    { id: 'DeepSeek-V4', name: 'DeepSeek-V4' },
    { id: 'Claude-3.5-Sonnet', name: 'Claude-3.5 Sonnet' },
    { id: 'GPT-4o', name: 'GPT-4o' },
    { id: 'Qwen-2.5-Max', name: 'Qwen-2.5 Max' },
  ]

  return (
    <div className="w-full h-full flex flex-col items-center justify-center relative px-6 pt-10 pb-6 overflow-hidden select-none">
      {/* Container for the Fanned Card Deck */}
      <div className="relative w-full max-w-3xl min-h-[600px] flex items-center justify-center">
        {/* Fanned Background Deck — the REAL ancestor cards stacked behind the
            active one. Each layer is an actual node (immediate parent → root):
            it shows that card's title and clicks back to it. The fan grows and
            shrinks with the true depth of the path — nothing is faked. */}
        {(() => {
          const ancestors = [...parentNodesStack].reverse() // immediate parent first
          // Render deepest first so shallower cards paint on top of the pile.
          return ancestors
            .map((ancestor, i) => {
              const depth = i + 1 // 1 = the card directly behind the active one
              const rot = depth * 3.4 // fan angle (clockwise)
              const scale = 1 - depth * 0.012
              const ty = -depth * 8 // each real card peeks a little higher
              return { ancestor, depth, rot, scale, ty }
            })
            .reverse()
            .map(({ ancestor, depth, rot, scale, ty }) => (
              <div
                key={ancestor.id}
                onClick={() => onSelectNode(ancestor.id)}
                title={`返回上层卡片：${ancestor.title}`}
                style={{
                  transform: `translateY(${ty}px) rotate(${rot}deg) scale(${scale})`,
                  transformOrigin: '50% 62%',
                  zIndex: 10 - depth,
                  opacity: Math.max(0.28, 0.7 - depth * 0.08),
                }}
                className={`absolute w-full h-[600px] rounded-2xl border border-white/[0.07] shadow-[0_24px_60px_rgba(0,0,0,0.55)] cursor-pointer hover:opacity-90 transition-all duration-300 overflow-hidden ${
                  depth % 2 === 0 ? 'bg-[#111113]' : 'bg-[#151517]'
                }`}
              >
                {/* Peek of the real card behind — its title strip */}
                <div className="flex items-center gap-2 px-6 pt-5">
                  <span className="text-[11px] font-mono text-gray-600 shrink-0">
                    #{ancestor.id}
                  </span>
                  <span className="text-xs text-gray-500 truncate">{ancestor.title}</span>
                </div>
              </div>
            ))
        })()}

        {/* Foreground Active Focused Card */}
        <div className="relative w-full min-h-[600px] rounded-2xl bg-[#141416] border border-white/12 card-elev p-6 sm:p-8 flex flex-col justify-between z-20 animate-in fade-in duration-200">
          {/* Card Header */}
          <div className="flex items-center justify-between pb-4 border-b border-white/8">
            {/* Title & Meta */}
            <div className="flex items-center gap-3 min-w-0">
              <h2 className="text-base sm:text-lg font-semibold text-gray-100 tracking-tight truncate">
                {currentNode.title}
              </h2>
              <span className="text-[11px] font-mono text-gray-500 bg-white/[0.05] px-2 py-0.5 rounded border border-white/8 shrink-0">
                #{currentNode.id}
              </span>
              {currentNode.model && (
                <span className="text-[11px] font-mono text-gray-500 hidden sm:inline shrink-0">
                  {currentNode.model}
                </span>
              )}
            </div>

            {/* Actions */}
            <div className="flex items-center gap-1.5 shrink-0">
              <button
                onClick={() => onToggleMainline(currentNode.id)}
                className={`h-7 px-2.5 rounded-md border flex items-center gap-1.5 transition text-[11px] font-medium ${
                  currentNode.isMainline
                    ? 'text-[#7fae8e] border-[#7fae8e]/35 bg-[#7fae8e]/10'
                    : 'bg-white/[0.04] text-gray-400 border-white/8 hover:text-gray-200 hover:bg-white/[0.08]'
                }`}
                title={currentNode.isMainline ? '当前为主线卡片，点击取消' : '标记为主线卡片'}
              >
                <Milestone className="w-3.5 h-3.5" />
                <span className="hidden sm:inline">{currentNode.isMainline ? '主线' : '设为主线'}</span>
              </button>

              <button
                onClick={() => onQuickHighlightNode(currentNode.id, 'amber')}
                className="w-7 h-7 rounded-md bg-white/[0.04] hover:bg-white/[0.08] text-gray-400 hover:text-[#cba86a] border border-white/8 flex items-center justify-center transition"
                title="划重点归集"
              >
                <Bookmark className="w-4 h-4" />
              </button>

              <button
                onClick={() => onDeleteNode(currentNode.id)}
                className="w-7 h-7 rounded-md bg-white/[0.04] hover:bg-white/[0.08] text-gray-400 hover:text-rose-400 border border-white/8 flex items-center justify-center transition"
                title="清理卡片"
              >
                <Trash2 className="w-4 h-4" />
              </button>

              <button
                onClick={onSwitchToMacro}
                className="ml-1 h-7 px-2.5 rounded-md bg-white/[0.04] hover:bg-white/[0.08] text-gray-400 hover:text-gray-200 border border-white/8 text-[11px] transition flex items-center gap-1.5"
                title="切换到宏观网格图谱"
              >
                <Maximize2 className="w-3.5 h-3.5" />
                <span className="hidden sm:inline">宏观网格</span>
              </button>
            </div>
          </div>

          {/* Primary Card Content */}
          <div className="flex-1 py-5 overflow-y-auto space-y-4 max-h-[360px] pr-2">
            {/* User Question Pill */}
            <div className="flex flex-col items-end gap-1 mb-2">
              <span className="text-[10px] font-mono text-gray-600 flex items-center gap-1">
                <Clock className="w-3 h-3" />
                {currentNode.timestamp}
              </span>
              <div className="px-3.5 py-2 rounded-lg bg-white/[0.05] border border-white/8 text-xs text-gray-300 max-w-md leading-relaxed">
                {currentNode.type === 'question'
                  ? currentNode.content
                  : `提问研读: ${currentNode.title}`}
              </div>
            </div>

            {/* Foldable AI Reasoning Block */}
            {currentNode.type !== 'question' && (
              <div className="rounded-lg bg-black/25 border border-white/6 overflow-hidden">
                <button
                  onClick={() => setShowReasoning(!showReasoning)}
                  className="w-full px-3 py-2 text-[11px] font-mono text-gray-500 hover:text-gray-300 flex items-center justify-between transition"
                >
                  <span className="flex items-center gap-2">
                    <span className="w-1.5 h-1.5 rounded-full bg-[#7fae8e]" />
                    <span>思考完成 · 1s</span>
                  </span>
                  {showReasoning ? (
                    <ChevronUp className="w-3.5 h-3.5" />
                  ) : (
                    <ChevronDown className="w-3.5 h-3.5" />
                  )}
                </button>
                {showReasoning && (
                  <div className="px-3 py-2 text-[11px] text-gray-500 font-mono border-t border-white/6 leading-relaxed">
                    已分析该卡片的上下文链接与知识框架，提炼深度思考与代码 / SOP…
                  </div>
                )}
              </div>
            )}

            {/* Text Content */}
            <div
              onMouseUp={handleTextMouseUp}
              className="text-sm sm:text-[15px] text-gray-200 leading-relaxed space-y-3 select-text"
            >
              {currentNode.content.split('\n\n').map((paragraph, idx) => {
                if (paragraph.startsWith('```')) {
                  const codeContent = paragraph.replace(/```[a-z]*/g, '').trim()
                  return (
                    <div
                      key={idx}
                      className="my-3 rounded-lg bg-[#0d0d0f] border border-white/8 overflow-hidden"
                    >
                      <div className="text-[10px] text-gray-600 px-3.5 pt-2.5 flex justify-between uppercase font-mono tracking-wider">
                        <span>code</span>
                        <span className="hover:text-gray-400 cursor-pointer transition">copy</span>
                      </div>
                      <pre className="px-3.5 pb-3 pt-1 font-mono text-xs text-gray-300 overflow-x-auto">
                        {codeContent}
                      </pre>
                    </div>
                  )
                }
                return (
                  <p key={idx} className="whitespace-pre-wrap">
                    {paragraph}
                  </p>
                )
              })}
            </div>

            {/* Tags */}
            {currentNode.tags && currentNode.tags.length > 0 && (
              <div className="flex items-center gap-2 pt-1 flex-wrap">
                {currentNode.tags.map((t) => (
                  <span
                    key={t}
                    className="text-[11px] font-mono px-2 py-0.5 rounded bg-white/[0.04] text-gray-400 border border-white/8 flex items-center gap-1.5"
                  >
                    <Tag className="w-3 h-3 text-gray-600" />
                    {t}
                  </span>
                ))}
              </div>
            )}
          </div>

          {/* Bottom Card Footer: Branch Pills + In-Card Prompt Input Area */}
          <div className="pt-4 border-t border-white/8 space-y-3">
            {/* Branch Pills */}
            <div className="flex items-center justify-between gap-2 text-xs">
              {!currentNode.isMainline ? (
                <button
                  onClick={onReturnToMainline}
                  className="px-2.5 py-1 rounded-md text-[#7fae8e] border border-[#7fae8e]/30 bg-[#7fae8e]/10 hover:bg-[#7fae8e]/15 text-[11px] font-medium flex items-center gap-1.5 transition shrink-0"
                  title="返回主干末端继续推进"
                >
                  <CornerUpLeft className="w-3.5 h-3.5" />
                  <span className="hidden sm:inline">回到主线</span>
                </button>
              ) : (
                <span className="text-[11px] text-gray-600 hidden sm:inline">衍生新视角</span>
              )}
              <div className="flex items-center gap-1.5 flex-1 justify-end">
                <button
                  onClick={() => onBranchCard(currentNode.id, 'sub')}
                  className="px-2.5 py-1 rounded-md bg-white/[0.04] hover:bg-white/[0.08] text-gray-400 hover:text-gray-100 border border-white/8 text-[11px] font-medium transition flex items-center gap-1.5"
                  title="针对局部细节进一步深挖，堆叠新卡片"
                >
                  <ArrowUpRight className="w-3.5 h-3.5" />
                  <span>深挖</span>
                </button>

                <button
                  onClick={() => onBranchCard(currentNode.id, 'divergent')}
                  className="px-2.5 py-1 rounded-md bg-white/[0.04] hover:bg-white/[0.08] text-gray-400 hover:text-gray-100 border border-white/8 text-[11px] font-medium transition flex items-center gap-1.5"
                  title="同级扩展思维，平行发散节点"
                >
                  <MoveRight className="w-3.5 h-3.5" />
                  <span>发散</span>
                </button>

                <button
                  onClick={() => onBranchCard(currentNode.id, 'branch')}
                  className="px-2.5 py-1 rounded-md bg-white/[0.04] hover:bg-white/[0.08] text-gray-400 hover:text-gray-100 border border-white/8 text-[11px] font-medium transition flex items-center gap-1.5"
                  title="换个全新角度切入"
                >
                  <ArrowDown className="w-3.5 h-3.5" />
                  <span>换角</span>
                </button>
              </div>
            </div>

            {/* In-Card Prompt Input Area */}
            <div className="rounded-xl bg-[#0f0f11] border border-white/10 p-2 flex items-end gap-2 focus-within:border-white/20 transition">
              {/* Model Dropdown inside card */}
              <div className="relative self-center">
                <button
                  onClick={() => setShowModelDropdown(!showModelDropdown)}
                  className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg bg-white/[0.04] hover:bg-white/[0.08] border border-white/8 text-xs font-mono text-gray-300 transition"
                >
                  <Bot className="w-3.5 h-3.5 text-gray-500" />
                  <span className="hidden sm:inline">{selectedModel}</span>
                  <ChevronDown className="w-3 h-3 text-gray-500" />
                </button>

                {showModelDropdown && (
                  <div className="absolute left-0 bottom-full mb-2 w-48 bg-[#161618] rounded-lg p-1 border border-white/12 shadow-xl shadow-black/40 z-50 animate-in fade-in slide-in-from-bottom-2">
                    {models.map((m) => (
                      <button
                        key={m.id}
                        onClick={() => {
                          onSelectModel(m.id)
                          setShowModelDropdown(false)
                        }}
                        className={`w-full text-left px-2 py-1.5 rounded-md text-xs font-mono transition ${
                          m.id === selectedModel
                            ? 'bg-white/[0.08] text-gray-100'
                            : 'text-gray-400 hover:bg-white/[0.04] hover:text-gray-200'
                        }`}
                      >
                        {m.name}
                      </button>
                    ))}
                  </div>
                )}
              </div>

              {/* In-Card Prompt Textarea */}
              <div className="flex-1">
                <textarea
                  ref={textareaRef}
                  rows={1}
                  value={inCardPrompt}
                  onChange={(e) => setInCardPrompt(e.target.value)}
                  onKeyDown={handleKeyDown}
                  placeholder="基于当前卡片追问或探讨…（Enter 发送）"
                  className="w-full bg-transparent text-xs sm:text-sm text-gray-100 placeholder-gray-600 resize-none outline-none py-1.5 px-1"
                />
              </div>

              {/* Web Search & Send */}
              <div className="flex items-center gap-1.5 self-center">
                <button
                  onClick={() => setWebSearch(!webSearch)}
                  className={`p-1.5 rounded-lg border text-xs transition ${
                    webSearch
                      ? 'bg-white/[0.08] text-gray-200 border-white/15'
                      : 'bg-transparent text-gray-600 border-white/8 hover:text-gray-400'
                  }`}
                  title={webSearch ? '联网检索已开启' : '联网检索已关闭'}
                >
                  <Globe className="w-3.5 h-3.5" />
                </button>

                <button
                  onClick={handleSendInCard}
                  disabled={!inCardPrompt.trim() || isGenerating}
                  className="p-2 rounded-lg bg-[#cba86a] hover:bg-[#d8b877] text-black disabled:opacity-25 disabled:hover:bg-[#cba86a] transition flex items-center justify-center"
                  title="发送追问并在堆叠层推入 AI 新回答"
                >
                  {isGenerating ? (
                    <Loader2 className="w-3.5 h-3.5 animate-spin" />
                  ) : (
                    <Send className="w-3.5 h-3.5" />
                  )}
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
