import React from 'react'
import { Bookmark, Code2, CheckSquare, X } from 'lucide-react'
import { HighlightType } from '../types'

interface HighlightToolbarProps {
  position: { x: number; y: number }
  selectedText: string
  onHighlight: (type: HighlightType, text: string) => void
  onClose: () => void
}

export const HighlightToolbar: React.FC<HighlightToolbarProps> = ({
  position,
  selectedText,
  onHighlight,
  onClose,
}) => {
  if (!selectedText) return null

  return (
    <div
      style={{
        left: `${position.x}px`,
        top: `${position.y - 48}px`,
      }}
      className="fixed z-50 transform -translate-x-1/2 flex items-center gap-1 p-1 rounded-lg bg-[#161618] border border-white/12 shadow-xl shadow-black/40 animate-in fade-in zoom-in-95 duration-150"
    >
      <div className="px-2 py-0.5 text-[11px] font-mono text-gray-400 max-w-[140px] truncate border-r border-white/8 pr-2">
        "{selectedText}"
      </div>

      <button
        onClick={() => onHighlight('amber', selectedText)}
        className="flex items-center gap-1 px-2 py-1 rounded-md bg-white/[0.04] hover:bg-white/[0.08] text-gray-200 hover:text-gray-100 border border-white/8 text-xs font-medium transition"
        title="划重点"
      >
        <Bookmark className="w-3.5 h-3.5 text-[#cba86a]" />
        <span>划重点</span>
      </button>

      <button
        onClick={() => onHighlight('emerald', selectedText)}
        className="flex items-center gap-1 px-2 py-1 rounded-md bg-white/[0.04] hover:bg-white/[0.08] text-gray-200 hover:text-gray-100 border border-white/8 text-xs font-medium transition"
        title="标记代码/SOP"
      >
        <Code2 className="w-3.5 h-3.5 text-[#7fae8e]" />
        <span>标记代码</span>
      </button>

      <button
        onClick={() => onHighlight('coral', selectedText)}
        className="flex items-center gap-1 px-2 py-1 rounded-md bg-white/[0.04] hover:bg-white/[0.08] text-gray-200 hover:text-gray-100 border border-white/8 text-xs font-medium transition"
        title="存为待办"
      >
        <CheckSquare className="w-3.5 h-3.5 text-[#b09bd0]" />
        <span>存待办</span>
      </button>

      <button
        onClick={onClose}
        className="p-1 rounded-md text-gray-400 hover:text-gray-100 hover:bg-white/[0.08] ml-0.5"
      >
        <X className="w-3.5 h-3.5" />
      </button>
    </div>
  )
}
