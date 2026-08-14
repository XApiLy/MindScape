export type ModelType = 'DeepSeek-V4' | 'Claude-3.5-Sonnet' | 'GPT-4o' | 'Qwen-2.5-Max'

export type HighlightType = 'amber' | 'emerald' | 'coral' // amber: keypoint, emerald: code/sop, coral: todo

export interface HighlightItem {
  id: string
  nodeId: string
  type: HighlightType
  text: string
  note?: string
  timestamp: string
}

export type NodeType = 'question' | 'ai_response' | 'sub_card' | 'divergent_card' | 'branch_card'

export interface CanvasNode {
  id: string
  type: NodeType
  title: string
  content: string
  model?: ModelType
  parentId?: string
  x: number
  y: number
  width: number
  height?: number
  timestamp: string
  tags?: string[]
  isCollapsed?: boolean
  isMainline?: boolean // 是否属于本会话的主线/主干
  codeSnippets?: { language: string; code: string }[]
}

export interface Edge {
  id: string
  source: string
  target: string
  label?: string
  type?: 'sub' | 'divergent' | 'branch'
}

// 一个会话 = 一次完整的对话，拥有专属的卡片流(专注堆叠)与宏观图谱
export interface Conversation {
  id: string
  name: string
  updatedAt: string
  nodes: CanvasNode[]
  edges: Edge[]
  highlights: HighlightItem[]
  mainlineRootId: string // 主线起点节点
  currentNodeId: string // 当前专注的卡片
}

// 一个项目 = 一个分组，内含多个会话
export interface Project {
  id: string
  name: string
  description: string
  updatedAt: string
  conversations: Conversation[]
}
