import React, { useState } from 'react'
import {
  X,
  Bookmark,
  Code2,
  CheckSquare,
  Zap,
  ExternalLink,
  Trash2,
  Sparkles,
  Filter,
  FileText,
  Copy,
  Check,
} from 'lucide-react'
import { HighlightItem, HighlightType } from '../types'
import { safeCopyText } from '../utils/clipboard'

interface HighlightsDrawerProps {
  isOpen: boolean
  onClose: () => void
  highlights: HighlightItem[]
  onRemoveHighlight: (id: string) => void
  onJumpToNode: (nodeId: string) => void
  onGenerateSynthesis: () => void
}

export const HighlightsDrawer: React.FC<HighlightsDrawerProps> = ({
  isOpen,
  onClose,
  highlights,
  onRemoveHighlight,
  onJumpToNode,
  onGenerateSynthesis,
}) => {
  const [activeFilter, setActiveFilter] = useState<'all' | HighlightType>('all')
  const [copiedId, setCopiedId] = useState<string | null>(null)

  if (!isOpen) return null

  const filteredHighlights = highlights.filter((h) => {
    if (activeFilter === 'all') return true
    return h.type === activeFilter
  })

  const getBadgeStyle = (type: HighlightType) => {
    switch (type) {
      case 'amber':
        return {
          bg: 'bg-white/[0.04] text-gray-300 border-white/8',
          dot: 'bg-[#cba86a]',
          label: '划重点',
        }
      case 'emerald':
        return {
          bg: 'bg-white/[0.04] text-gray-300 border-white/8',
          dot: 'bg-[#7fae8e]',
          label: '代码 / SOP',
        }
      case 'coral':
        return {
          bg: 'bg-white/[0.04] text-gray-300 border-white/8',
          dot: 'bg-rose-400',
          label: '疑问 / 待办',
        }
    }
  }

  const handleCopy = async (text: string, id: string) => {
    await safeCopyText(text)
    setCopiedId(id)
    setTimeout(() => setCopiedId(null), 1500)
  }

  return (
    <aside className="fixed top-16 right-4 bottom-4 w-[360px] z-30 bg-[#141416] rounded-lg border border-white/8 shadow-xl shadow-black/40 flex flex-col overflow-hidden animate-in slide-in-from-right duration-200">
      {/* Header */}
      <div className="p-4 border-b border-white/8 flex items-center justify-between bg-[#111113]">
        <div className="flex items-center gap-2">
          <div className="p-1.5 rounded-md bg-white/[0.04] text-[#cba86a] border border-white/8">
            <Bookmark className="w-4 h-4" />
          </div>
          <div>
            <h3 className="font-semibold text-gray-100 text-sm flex items-center gap-2">
              重点文档 & 收敛中心
            </h3>
            <p className="text-[11px] text-gray-400">
              已采集 <span className="text-[#cba86a] font-mono font-bold">{highlights.length}</span> 条高光金句与核心节点
            </p>
          </div>
        </div>
        <button
          onClick={onClose}
          className="p-1.5 rounded-md text-gray-400 hover:text-gray-100 hover:bg-white/[0.08] transition"
        >
          <X className="w-4 h-4" />
        </button>
      </div>

      {/* Filter Tabs */}
      <div className="px-3 py-2 border-b border-white/8 flex items-center gap-1 bg-[#111113] text-xs">
        <button
          onClick={() => setActiveFilter('all')}
          className={`px-2.5 py-1 rounded-md text-[11px] font-medium transition ${
            activeFilter === 'all'
              ? 'bg-white/[0.07] text-gray-100 border border-white/12'
              : 'text-gray-400 hover:text-gray-200'
          }`}
        >
          全部 ({highlights.length})
        </button>
        <button
          onClick={() => setActiveFilter('amber')}
          className={`px-2 py-1 rounded-md text-[11px] font-medium transition flex items-center gap-1 ${
            activeFilter === 'amber'
              ? 'bg-white/[0.07] text-gray-100 border border-white/12'
              : 'text-gray-400 hover:text-gray-200'
          }`}
        >
          <span className="w-1.5 h-1.5 rounded-full bg-[#cba86a]" />
          重点
        </button>
        <button
          onClick={() => setActiveFilter('emerald')}
          className={`px-2 py-1 rounded-md text-[11px] font-medium transition flex items-center gap-1 ${
            activeFilter === 'emerald'
              ? 'bg-white/[0.07] text-gray-100 border border-white/12'
              : 'text-gray-400 hover:text-gray-200'
          }`}
        >
          <span className="w-1.5 h-1.5 rounded-full bg-[#7fae8e]" />
          代码
        </button>
        <button
          onClick={() => setActiveFilter('coral')}
          className={`px-2 py-1 rounded-md text-[11px] font-medium transition flex items-center gap-1 ${
            activeFilter === 'coral'
              ? 'bg-white/[0.07] text-gray-100 border border-white/12'
              : 'text-gray-400 hover:text-gray-200'
          }`}
        >
          <span className="w-1.5 h-1.5 rounded-full bg-rose-400" />
          待办
        </button>
      </div>

      {/* Highlights List */}
      <div className="flex-1 overflow-y-auto p-3 space-y-3">
        {filteredHighlights.length === 0 ? (
          <div className="h-full flex flex-col items-center justify-center text-center p-6 text-gray-500">
            <FileText className="w-10 h-10 mb-2 opacity-40 text-gray-500" />
            <p className="text-xs font-medium text-gray-400">尚无匹配的重点条目</p>
            <p className="text-[11px] text-gray-500 mt-1 max-w-[200px]">
              在中央画布卡片中拖拽选中任意文字，点击弹出浮窗中的“划重点”即可归集至此处。
            </p>
          </div>
        ) : (
          filteredHighlights.map((item) => {
            const badge = getBadgeStyle(item.type)
            return (
              <div
                key={item.id}
                className="group relative p-3 rounded-md bg-white/[0.03] hover:bg-white/[0.07] border border-white/8 hover:border-white/12 transition-all duration-200"
              >
                {/* Top Badge & Source */}
                <div className="flex items-center justify-between mb-2">
                  <span
                    className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded text-[10px] font-medium border ${badge.bg}`}
                  >
                    <span className={`w-1.5 h-1.5 rounded-full ${badge.dot}`} />
                    {badge.label}
                  </span>

                  <button
                    onClick={() => onJumpToNode(item.nodeId)}
                    className="flex items-center gap-1 text-[10px] font-mono text-gray-400 hover:text-gray-100 bg-white/[0.04] px-2 py-0.5 rounded border border-white/8 hover:bg-white/[0.08] transition"
                    title="点击平滑跳转至画布卡片节点"
                  >
                    <span>来自 #{item.nodeId}</span>
                    <ExternalLink className="w-2.5 h-2.5" />
                  </button>
                </div>

                {/* Highlight Content */}
                <p className="text-xs text-gray-200 leading-relaxed font-sans font-normal border-l-2 border-[#cba86a]/50 pl-2.5 py-0.5 my-1.5 bg-[#0f0f11] rounded-r">
                  {item.text}
                </p>

                {item.note && (
                  <p className="text-[11px] text-gray-400 italic font-sans pl-2.5">
                    {item.note}
                  </p>
                )}

                {/* Card Actions */}
                <div className="flex items-center justify-between mt-2 pt-2 border-t border-white/8 opacity-80 group-hover:opacity-100 transition">
                  <span className="text-[10px] font-mono text-gray-500">{item.timestamp}</span>
                  <div className="flex items-center gap-1">
                    <button
                      onClick={() => handleCopy(item.text, item.id)}
                      className="p-1 rounded text-gray-400 hover:text-white hover:bg-white/10 transition"
                      title="复制文本"
                    >
                      {copiedId === item.id ? (
                        <Check className="w-3 h-3 text-[#7fae8e]" />
                      ) : (
                        <Copy className="w-3 h-3" />
                      )}
                    </button>
                    <button
                      onClick={() => onRemoveHighlight(item.id)}
                      className="p-1 rounded text-gray-400 hover:text-rose-400 hover:bg-white/10 transition"
                      title="删除此重点"
                    >
                      <Trash2 className="w-3 h-3" />
                    </button>
                  </div>
                </div>
              </div>
            )
          })
        )}
      </div>

      {/* Bottom Synthesis Action CTA */}
      <div className="p-3 border-t border-white/8 bg-[#111113]">
        <button
          onClick={onGenerateSynthesis}
          disabled={highlights.length === 0}
          className="w-full py-2.5 px-4 rounded-md bg-[#cba86a] hover:bg-[#d8b877] text-black font-bold text-xs flex items-center justify-center gap-2 shadow-xl shadow-black/40 disabled:opacity-50 disabled:cursor-not-allowed transition transform active:scale-95"
        >
          <Zap className="w-4 h-4 text-black fill-black" />
          <span>一键 AI 总结 / 生成结构化报告 (Synthesis)</span>
        </button>
      </div>
    </aside>
  )
}
