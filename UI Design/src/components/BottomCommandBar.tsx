import React, { useState, useRef, useEffect } from 'react'
import {
  Send,
  Globe,
  ChevronDown,
  Sparkles,
  Bot,
  Command,
  HelpCircle,
  Lightbulb,
  X,
} from 'lucide-react'
import { ModelType } from '../types'

interface BottomCommandBarProps {
  selectedModel: ModelType
  onSelectModel: (model: ModelType) => void
  onSendMessage: (prompt: string, model: ModelType, webSearch: boolean) => void
  isGenerating: boolean
}

export const BottomCommandBar: React.FC<BottomCommandBarProps> = ({
  selectedModel,
  onSelectModel,
  onSendMessage,
  isGenerating,
}) => {
  const [prompt, setPrompt] = useState('')
  const [webSearch, setWebSearch] = useState(true)
  const [showModelDropdown, setShowModelDropdown] = useState(false)
  const [showPromptPresets, setShowPromptPresets] = useState(false)
  const textareaRef = useRef<HTMLTextAreaElement>(null)

  const models: { id: ModelType; name: string; tag: string; badgeColor: string }[] = [
    {
      id: 'DeepSeek-V4',
      name: 'DeepSeek-V4 (MLA + MoE)',
      tag: '极致吞吐 & RAG 极低成本',
      badgeColor: 'text-gray-400 bg-white/[0.04] border-white/8',
    },
    {
      id: 'Claude-3.5-Sonnet',
      name: 'Claude-3.5 Sonnet',
      tag: '复杂逻辑推演与精细代码',
      badgeColor: 'text-gray-400 bg-white/[0.04] border-white/8',
    },
    {
      id: 'GPT-4o',
      name: 'GPT-4o (Omni Realtime)',
      tag: '全能旗舰与结构化输出',
      badgeColor: 'text-gray-400 bg-white/[0.04] border-white/8',
    },
    {
      id: 'Qwen-2.5-Max',
      name: 'Qwen-2.5 Max (开源全效)',
      tag: '中文理解与私有化精调',
      badgeColor: 'text-gray-400 bg-white/[0.04] border-white/8',
    },
  ]

  const PRESET_PROMPTS = [
    '请详细拆解系统的高并发架构，并给出代码示例与性能测试指标。',
    '如何优化当前 RAG 知识库的重排序 (Re-ranking) 与混合检索召回率？',
    '从云原生 K8s 与 GPU 显存分配视角，给出成本最低的私有化微调方案。',
  ]

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSend()
    }
  }

  const handleSend = () => {
    if (!prompt.trim() || isGenerating) return
    onSendMessage(prompt.trim(), selectedModel, webSearch)
    setPrompt('')
  }

  return (
    <div className="fixed bottom-5 left-1/2 -translate-x-1/2 z-40 w-full max-w-2xl px-4">
      {/* Preset Prompts Floating Panel */}
      {showPromptPresets && (
        <div className="mb-2 p-2 bg-[#161618] rounded-lg border border-white/8 shadow-xl shadow-black/40 space-y-1 animate-in fade-in slide-in-from-bottom-2">
          <div className="flex items-center justify-between px-2 py-1 text-[11px] font-mono text-gray-400 border-b border-white/8">
            <span className="flex items-center gap-1">
              <Lightbulb className="w-3 h-3 text-gray-400" />
              灵感预设提示词 (PRESETS)
            </span>
            <button
              onClick={() => setShowPromptPresets(false)}
              className="text-gray-500 hover:text-gray-100"
            >
              <X className="w-3.5 h-3.5" />
            </button>
          </div>
          {PRESET_PROMPTS.map((p, idx) => (
            <button
              key={idx}
              onClick={() => {
                setPrompt(p)
                setShowPromptPresets(false)
              }}
              className="w-full text-left p-2 rounded-md text-xs text-gray-300 hover:bg-white/[0.08] hover:text-gray-100 transition border border-transparent hover:border-white/12"
            >
              {p}
            </button>
          ))}
        </div>
      )}

      {/* Main Capsule Command Bar */}
      <div className="bg-[#161618] rounded-xl p-2 flex items-end gap-2 border border-white/12 shadow-xl shadow-black/40 relative">
        {/* Model Selector Dropdown Button */}
        <div className="relative self-center">
          <button
            onClick={() => setShowModelDropdown(!showModelDropdown)}
            className="flex items-center gap-1.5 px-3 py-2 rounded-lg bg-[#0f0f11] hover:bg-white/[0.08] border border-white/8 text-xs font-medium text-gray-200 hover:text-gray-100 transition shrink-0"
          >
            <Bot className="w-3.5 h-3.5 text-gray-400" />
            <span className="font-mono">{selectedModel}</span>
            <ChevronDown className="w-3 h-3 text-gray-400" />
          </button>

          {showModelDropdown && (
            <div className="absolute left-0 bottom-full mb-2 w-72 bg-[#161618] rounded-lg p-1.5 border border-white/12 shadow-xl shadow-black/40 z-50 animate-in fade-in slide-in-from-bottom-2">
              <div className="px-2 py-1 text-[10px] font-mono text-gray-400 border-b border-white/8">
                选择底层推理大模型 (MODELS)
              </div>
              <div className="py-1 space-y-1">
                {models.map((m) => (
                  <button
                    key={m.id}
                    onClick={() => {
                      onSelectModel(m.id)
                      setShowModelDropdown(false)
                    }}
                    className={`w-full text-left p-2 rounded-md text-xs flex flex-col gap-0.5 transition ${
                      m.id === selectedModel
                        ? 'bg-white/[0.07] text-gray-100 border border-white/12'
                        : 'text-gray-300 hover:bg-white/[0.04] hover:text-gray-100'
                    }`}
                  >
                    <div className="flex items-center justify-between font-medium">
                      <span>{m.name}</span>
                      <span className={`px-1.5 py-0.2 text-[9px] rounded font-mono border ${m.badgeColor}`}>
                        ACTIVE
                      </span>
                    </div>
                    <div className="text-[10px] text-gray-400">{m.tag}</div>
                  </button>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Input Textarea */}
        <div className="flex-1 relative">
          <textarea
            ref={textareaRef}
            rows={1}
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="探索一切... (Enter 发送, Shift+Enter 换行)"
            className="w-full bg-transparent text-sm text-gray-100 placeholder-gray-500 resize-none outline-none py-2 px-1 max-h-32 font-sans"
          />
        </div>

        {/* Action Controls: Web search + Presets + Send */}
        <div className="flex items-center gap-1.5 self-center shrink-0">
          {/* Preset Prompts Button */}
          <button
            onClick={() => setShowPromptPresets(!showPromptPresets)}
            className="p-2 rounded-lg text-gray-400 hover:text-gray-100 hover:bg-white/[0.08] transition"
            title="灵感预设"
          >
            <Lightbulb className="w-4 h-4" />
          </button>

          {/* Web Search Toggle */}
          <button
            onClick={() => setWebSearch(!webSearch)}
            className={`p-2 rounded-lg border text-xs transition flex items-center gap-1 ${
              webSearch
                ? 'bg-white/[0.07] text-gray-100 border-white/12'
                : 'bg-[#0f0f11] text-gray-400 border-white/8 hover:text-gray-200'
            }`}
            title={webSearch ? '联网搜索已开启 (Web Search ON)' : '联网搜索已关闭'}
          >
            <Globe className="w-4 h-4" />
            <span className="text-[10px] font-mono hidden sm:inline">联网</span>
          </button>

          {/* Send Button */}
          <button
            onClick={handleSend}
            disabled={!prompt.trim() || isGenerating}
            className="p-2.5 rounded-lg bg-[#cba86a] hover:bg-[#d8b877] text-black font-bold disabled:opacity-30 disabled:hover:bg-[#cba86a] transition active:scale-95 flex items-center justify-center"
            title="发送提示词生成节点"
          >
            {isGenerating ? (
              <Sparkles className="w-4 h-4 text-black animate-spin" />
            ) : (
              <Send className="w-4 h-4 text-black fill-black" />
            )}
          </button>
        </div>
      </div>
    </div>
  )
}
