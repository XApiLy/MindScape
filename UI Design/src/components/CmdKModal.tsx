import React, { useState, useEffect } from 'react'
import { Search, X, Bot, FileText, ArrowRight } from 'lucide-react'
import { CanvasNode, HighlightItem } from '../types'

interface CmdKModalProps {
  isOpen: boolean
  onClose: () => void
  nodes: CanvasNode[]
  highlights: HighlightItem[]
  onSelectNode: (nodeId: string) => void
}

export const CmdKModal: React.FC<CmdKModalProps> = ({
  isOpen,
  onClose,
  nodes,
  highlights,
  onSelectNode,
}) => {
  const [query, setQuery] = useState('')

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault()
        if (isOpen) onClose()
        else setQuery('')
      }
      if (e.key === 'Escape' && isOpen) {
        onClose()
      }
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [isOpen, onClose])

  if (!isOpen) return null

  const filteredNodes = nodes.filter(
    (n) =>
      n.title.toLowerCase().includes(query.toLowerCase()) ||
      n.content.toLowerCase().includes(query.toLowerCase()) ||
      n.id.toLowerCase().includes(query.toLowerCase())
  )

  const filteredHighlights = highlights.filter((h) =>
    h.text.toLowerCase().includes(query.toLowerCase())
  )

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center pt-20 bg-black/70 backdrop-blur-md animate-in fade-in duration-150">
      <div className="w-full max-w-xl bg-[#161618] rounded-xl border border-white/12 shadow-xl shadow-black/40 overflow-hidden">
        {/* Search Bar */}
        <div className="p-4 border-b border-white/8 flex items-center gap-3 bg-[#0f0f11]">
          <Search className="w-5 h-5 text-gray-400" />
          <input
            type="text"
            autoFocus
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="搜索节点 ID、关键词或高光金句... (Esc 退出)"
            className="w-full bg-transparent text-gray-100 placeholder-gray-500 outline-none text-sm font-sans"
          />
          <button
            onClick={onClose}
            className="p-1 rounded text-gray-400 hover:text-gray-100 hover:bg-white/[0.08]"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Results List */}
        <div className="max-h-[380px] overflow-y-auto p-2 space-y-4">
          {/* Nodes Section */}
          <div>
            <div className="px-2 py-1 text-[10px] font-mono text-gray-500 uppercase tracking-wider">
              画布节点 ({filteredNodes.length})
            </div>
            {filteredNodes.length === 0 ? (
              <div className="px-3 py-2 text-xs text-gray-500 italic">未找到匹配的卡片</div>
            ) : (
              <div className="space-y-1">
                {filteredNodes.map((n) => (
                  <button
                    key={n.id}
                    onClick={() => {
                      onSelectNode(n.id)
                      onClose()
                    }}
                    className="w-full text-left p-2.5 rounded-md bg-white/[0.02] hover:bg-white/[0.08] border border-white/8 hover:border-white/12 transition flex items-center justify-between group"
                  >
                    <div>
                      <div className="flex items-center gap-2">
                        <span className="font-mono text-xs text-[#cba86a] font-bold">#{n.id}</span>
                        <span className="text-xs text-gray-200 font-medium">{n.title}</span>
                      </div>
                      <p className="text-[11px] text-gray-400 line-clamp-1 mt-0.5">{n.content}</p>
                    </div>
                    <ArrowRight className="w-4 h-4 text-gray-400 opacity-0 group-hover:opacity-100 group-hover:translate-x-1 transition" />
                  </button>
                ))}
              </div>
            )}
          </div>

          {/* Highlights Section */}
          {filteredHighlights.length > 0 && (
            <div>
              <div className="px-2 py-1 text-[10px] font-mono text-gray-500 uppercase tracking-wider">
                重点金句 ({filteredHighlights.length})
              </div>
              <div className="space-y-1">
                {filteredHighlights.map((h) => (
                  <button
                    key={h.id}
                    onClick={() => {
                      onSelectNode(h.nodeId)
                      onClose()
                    }}
                    className="w-full text-left p-2.5 rounded-md bg-white/[0.02] hover:bg-white/[0.08] border border-white/8 hover:border-white/12 transition flex items-center justify-between group"
                  >
                    <div className="pr-2">
                      <div className="text-xs text-gray-200 font-normal line-clamp-2">
                        "{h.text}"
                      </div>
                      <span className="text-[10px] font-mono text-gray-500 mt-1 block">
                        来自 #{h.nodeId}
                      </span>
                    </div>
                    <ArrowRight className="w-4 h-4 text-gray-400 opacity-0 group-hover:opacity-100 transition" />
                  </button>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
