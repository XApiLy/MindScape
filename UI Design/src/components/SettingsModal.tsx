import React, { useState } from 'react'
import { Sliders, X, Check, Key, Eye, SlidersHorizontal, Sparkles } from 'lucide-react'

interface SettingsModalProps {
  isOpen: boolean
  onClose: () => void
}

export const SettingsModal: React.FC<SettingsModalProps> = ({ isOpen, onClose }) => {
  const [apiKey, setApiKey] = useState('')
  const [glassIntensity, setGlassIntensity] = useState('high')
  const [gridDensity, setGridDensity] = useState('32px')
  const [saved, setSaved] = useState(false)

  if (!isOpen) return null

  const handleSave = () => {
    setSaved(true)
    setTimeout(() => {
      setSaved(false)
      onClose()
    }, 1000)
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-md animate-in fade-in duration-150">
      <div className="w-full max-w-md bg-[#161618] rounded-xl border border-white/12 shadow-xl shadow-black/40 overflow-hidden">
        <div className="p-4 border-b border-white/8 flex items-center justify-between bg-[#111113]">
          <div className="flex items-center gap-2">
            <SlidersHorizontal className="w-5 h-5 text-gray-400" />
            <h3 className="font-bold text-gray-100 text-sm">画布与 AI 引擎偏好设置</h3>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded text-gray-400 hover:text-gray-100 hover:bg-white/[0.08]"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="p-5 space-y-4 text-xs">
          {/* API Key */}
          <div className="space-y-1.5">
            <label className="text-gray-300 font-medium flex items-center gap-1.5">
              <Key className="w-3.5 h-3.5 text-gray-400" />
              自定义 API Key (DeepSeek / Claude)
            </label>
            <input
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              className="w-full p-2.5 rounded-md bg-[#0f0f11] border border-white/8 text-gray-200 outline-none focus:border-white/20 font-mono"
            />
          </div>

          {/* Glass Intensity */}
          <div className="space-y-1.5">
            <label className="text-gray-300 font-medium flex items-center gap-1.5">
              <Sparkles className="w-3.5 h-3.5 text-gray-400" />
              液态玻璃磨砂强度 (Liquid Glassmorphism)
            </label>
            <div className="grid grid-cols-3 gap-2">
              {['low', 'medium', 'high'].map((level) => (
                <button
                  key={level}
                  onClick={() => setGlassIntensity(level)}
                  className={`py-2 rounded-md capitalize font-mono transition ${
                    glassIntensity === level
                      ? 'bg-white/[0.07] text-gray-100 border border-white/12 font-bold'
                      : 'bg-[#0f0f11] text-gray-400 border border-white/8 hover:text-gray-100'
                  }`}
                >
                  {level === 'low' ? '微弱 (10px)' : level === 'medium' ? '标准 (16px)' : '极致 (24px)'}
                </button>
              ))}
            </div>
          </div>

          {/* Canvas Dot Grid */}
          <div className="space-y-1.5">
            <label className="text-gray-300 font-medium flex items-center gap-1.5">
              <Eye className="w-3.5 h-3.5 text-gray-400" />
              画布 32px 点阵网格密度
            </label>
            <div className="grid grid-cols-3 gap-2">
              {['24px', '32px', '48px'].map((d) => (
                <button
                  key={d}
                  onClick={() => setGridDensity(d)}
                  className={`py-2 rounded-md font-mono transition ${
                    gridDensity === d
                      ? 'bg-white/[0.07] text-gray-100 border border-white/12 font-bold'
                      : 'bg-[#0f0f11] text-gray-400 border border-white/8 hover:text-gray-100'
                  }`}
                >
                  {d}
                </button>
              ))}
            </div>
          </div>
        </div>

        <div className="p-4 bg-[#111113] border-t border-white/8 flex justify-end">
          <button
            onClick={handleSave}
            className="px-4 py-2 rounded-md bg-[#cba86a] hover:bg-[#d8b877] text-black font-bold text-xs transition flex items-center gap-1.5"
          >
            {saved ? <Check className="w-4 h-4 text-black" /> : null}
            <span>{saved ? '已保存设置' : '保存设置'}</span>
          </button>
        </div>
      </div>
    </div>
  )
}
