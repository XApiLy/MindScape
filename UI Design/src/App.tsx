import React, { useState, useEffect } from 'react'
import { LeftSidebar } from './components/LeftSidebar'
import { TopNav } from './components/TopNav'
import { Canvas } from './components/Canvas'
import { FocusedStackView } from './components/FocusedStackView'
import { TrajectoryDock } from './components/TrajectoryDock'
import { HighlightToolbar } from './components/HighlightToolbar'
import { HighlightsDrawer } from './components/HighlightsDrawer'
import { BottomCommandBar } from './components/BottomCommandBar'
import { CmdKModal } from './components/CmdKModal'
import { SynthesisModal } from './components/SynthesisModal'
import { SettingsModal } from './components/SettingsModal'

import { INITIAL_PROJECTS, makeSeedConversation } from './initialData'
import {
  CanvasNode,
  Conversation,
  Edge,
  HighlightItem,
  HighlightType,
  ModelType,
  Project,
} from './types'

export default function App() {
  const [projects, setProjects] = useState<Project[]>(INITIAL_PROJECTS)
  const [currentProjectId, setCurrentProjectId] = useState<string>(INITIAL_PROJECTS[0].id)
  const [currentConversationId, setCurrentConversationId] = useState<string>(
    INITIAL_PROJECTS[0].conversations[0].id,
  )

  // Layout & Navigation State
  const [isLeftSidebarOpen, setIsLeftSidebarOpen] = useState(true)
  const [viewMode, setViewMode] = useState<'focused_stack' | 'macro_canvas'>('focused_stack')

  // Canvas View state
  const [zoom, setZoom] = useState(0.9)
  const [pan, setPan] = useState({ x: 100, y: 60 })

  // Drawer & Modals state
  const [isDrawerOpen, setIsDrawerOpen] = useState(false)
  const [isCmdKOpen, setIsCmdKOpen] = useState(false)
  const [isSynthesisOpen, setIsSynthesisOpen] = useState(false)
  const [isSettingsOpen, setIsSettingsOpen] = useState(false)

  // Selection Popup
  const [selectionPopup, setSelectionPopup] = useState<{
    nodeId: string
    text: string
    position: { x: number; y: number }
  } | null>(null)

  // AI Model Selection & Generation
  const [selectedModel, setSelectedModel] = useState<ModelType>('DeepSeek-V4')
  const [isGenerating, setIsGenerating] = useState(false)

  // ── Derive the active project & conversation (each conversation owns its own
  //    card stack + macro graph) ─────────────────────────────────────────────
  const currentProject = projects.find((p) => p.id === currentProjectId) || projects[0]
  const currentConversation =
    currentProject.conversations.find((c) => c.id === currentConversationId) ||
    currentProject.conversations[0]

  const nodes = currentConversation.nodes
  const edges = currentConversation.edges
  const highlights = currentConversation.highlights
  const currentNodeId = currentConversation.currentNodeId
  const mainlineRootId = currentConversation.mainlineRootId

  // Scoped mutation helper — every node/edge/highlight change lives inside the
  // active conversation only.
  const updateCurrentConversation = (updater: (c: Conversation) => Conversation) => {
    setProjects((prev) =>
      prev.map((p) =>
        p.id !== currentProjectId
          ? p
          : {
              ...p,
              conversations: p.conversations.map((c) =>
                c.id !== currentConversationId ? c : updater(c),
              ),
            },
      ),
    )
  }

  const setNodes = (fn: (prev: CanvasNode[]) => CanvasNode[]) =>
    updateCurrentConversation((c) => ({ ...c, nodes: fn(c.nodes) }))
  const setEdges = (fn: (prev: Edge[]) => Edge[]) =>
    updateCurrentConversation((c) => ({ ...c, edges: fn(c.edges) }))
  const setHighlights = (fn: (prev: HighlightItem[]) => HighlightItem[]) =>
    updateCurrentConversation((c) => ({ ...c, highlights: fn(c.highlights) }))
  const setCurrentNodeId = (id: string) =>
    updateCurrentConversation((c) => ({ ...c, currentNodeId: id }))

  // Compute Parent Stack Trajectory
  const getParentStack = (nodeId: string): CanvasNode[] => {
    const stack: CanvasNode[] = []
    let curr = nodes.find((n) => n.id === nodeId)

    while (curr && curr.parentId) {
      const parent = nodes.find((n) => n.id === curr?.parentId)
      if (parent) {
        stack.unshift(parent)
        curr = parent
      } else {
        break
      }
    }
    return stack
  }

  const parentStack = getParentStack(currentNodeId)
  const currentNode = nodes.find((n) => n.id === currentNodeId) || nodes[0]
  const childNodes = nodes.filter((n) => n.parentId === currentNodeId)

  // Keyboard shortcut listener: Cmd/Ctrl + Tab to toggle view mode
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'Tab') {
        e.preventDefault()
        setViewMode((prev) => (prev === 'focused_stack' ? 'macro_canvas' : 'focused_stack'))
      }
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [])

  // Zoom handlers
  const handleZoomIn = () => setZoom((prev) => Math.min(2.0, +(prev + 0.1).toFixed(2)))
  const handleZoomOut = () => setZoom((prev) => Math.max(0.4, +(prev - 0.1).toFixed(2)))
  const handleResetZoom = () => {
    setZoom(0.9)
    setPan({ x: 120, y: 80 })
  }

  const handleUpdateNodePos = (id: string, x: number, y: number) => {
    setNodes((prev) => prev.map((n) => (n.id === id ? { ...n, x, y } : n)))
  }

  const handleDeleteNode = (id: string) => {
    if (nodes.length <= 1) return
    const fallback = nodes.find((n) => n.id !== id)?.id || nodes[0].id
    updateCurrentConversation((c) => ({
      ...c,
      nodes: c.nodes.filter((n) => n.id !== id),
      edges: c.edges.filter((e) => e.source !== id && e.target !== id),
      highlights: c.highlights.filter((h) => h.nodeId !== id),
      currentNodeId: c.currentNodeId === id ? fallback : c.currentNodeId,
    }))
  }

  const handleSelectNode = (nodeId: string) => {
    setCurrentNodeId(nodeId)
    const targetNode = nodes.find((n) => n.id === nodeId)
    if (targetNode) {
      const windowWidth = window.innerWidth
      const windowHeight = window.innerHeight
      const targetPanX = windowWidth / 2 - (targetNode.x + targetNode.width / 2) * zoom
      const targetPanY = windowHeight / 2 - (targetNode.y + 120) * zoom
      setPan({ x: Math.round(targetPanX), y: Math.round(targetPanY) })
    }
  }

  // 标记 / 取消当前卡片为主线
  const handleToggleMainline = (nodeId: string) => {
    setNodes((prev) => prev.map((n) => (n.id === nodeId ? { ...n, isMainline: !n.isMainline } : n)))
  }

  // 回到主线末端
  const handleReturnToMainline = () => {
    const mainlineNodes = nodes.filter((n) => n.isMainline)
    if (mainlineNodes.length === 0) return
    // 取主干最深处（没有主线子节点的那个）
    const tail =
      mainlineNodes.find((n) => !nodes.some((m) => m.parentId === n.id && m.isMainline)) ||
      mainlineNodes[mainlineNodes.length - 1]
    handleSelectNode(tail.id)
  }

  const handleQuickHighlightNode = (nodeId: string, type: HighlightType) => {
    const node = nodes.find((n) => n.id === nodeId)
    if (!node) return
    const newHighlight: HighlightItem = {
      id: `hl-${Date.now()}`,
      nodeId,
      type,
      text: node.content.slice(0, 120) + '...',
      note: node.title,
      timestamp: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
    }
    setHighlights((prev) => [newHighlight, ...prev])
  }

  const handleAddHighlight = (type: HighlightType, text: string) => {
    if (!selectionPopup) return
    const newHighlight: HighlightItem = {
      id: `hl-${Date.now()}`,
      nodeId: selectionPopup.nodeId,
      type,
      text,
      timestamp: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
    }
    setHighlights((prev) => [newHighlight, ...prev])
    setSelectionPopup(null)
    window.getSelection()?.removeAllRanges()
  }

  const handleRemoveHighlight = (id: string) => {
    setHighlights((prev) => prev.filter((h) => h.id !== id))
  }

  // 切换会话（切换整套卡片流 + 图谱）
  const handleSelectConversation = (projectId: string, conversationId: string) => {
    setCurrentProjectId(projectId)
    setCurrentConversationId(conversationId)
    setViewMode('focused_stack')
    handleResetZoom()
  }

  // 新建会话（一个全新的对话，归属当前项目）
  const handleNewChat = () => {
    const newId = `conv-${Date.now()}`
    const seed = makeSeedConversation(
      newId,
      '全新学习会话',
      '刚刚',
      '全新研读学习开端',
      '欢迎开启全新的 AI 沉浸研读对话。在下方输入你的第一个学习主题或代码探断需求，即可开始沿主线推进...',
    )
    setProjects((prev) =>
      prev.map((p) =>
        p.id !== currentProjectId
          ? p
          : { ...p, conversations: [seed, ...p.conversations], updatedAt: '刚刚' },
      ),
    )
    setCurrentConversationId(newId)
    setViewMode('focused_stack')
    handleResetZoom()
  }

  // Branch Card
  const handleBranchCard = (parentId: string, branchType: 'sub' | 'divergent' | 'branch') => {
    const parentNode = nodes.find((n) => n.id === parentId)
    if (!parentNode) return

    const newNodeId = `${currentConversationId}-N-${Date.now()}`

    let xOffset = 520
    let yOffset = 0
    let titlePrefix = ''
    let defaultContent = ''

    if (branchType === 'sub') {
      titlePrefix = '↗ 深挖细节: '
      yOffset = -120
      defaultContent = `针对 [${parentNode.title}] 进一步堆叠推演：\n\n1. **核心算法/机制**: 低秩隐向量投影与 KV Cache 压缩。\n2. **SOP 配置**: 设置硬件层面的并发流水线。`
    } else if (branchType === 'divergent') {
      titlePrefix = '→ 平行发散: '
      yOffset = 220
      defaultContent = `横向延伸 [${parentNode.title}] 的可能关联方案：\n\n- **方案 A**: 端侧剪枝与量化部署\n- **方案 B**: 跨流式多模态推理网关`
    } else {
      titlePrefix = '↓ 换角分支: '
      xOffset = 0
      yOffset = 380
      defaultContent = `换个全新的工程视角审视：\n\n从架构数据合规与多租户隔离的角度来看，当前方案需要哪些优化支持？`
    }

    const newNode: CanvasNode = {
      id: newNodeId,
      type: branchType === 'sub' ? 'sub_card' : branchType === 'divergent' ? 'divergent_card' : 'branch_card',
      title: `${titlePrefix}${parentNode.title.slice(0, 18)}...`,
      model: selectedModel,
      parentId,
      isMainline: false, // 分支/发散/换角默认不属于主线
      x: parentNode.x + xOffset,
      y: parentNode.y + yOffset,
      width: 480,
      timestamp: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
      content: defaultContent,
      tags: [branchType === 'sub' ? '深挖' : branchType === 'divergent' ? '发散' : '换角'],
    }

    const newEdge: Edge = {
      id: `edge-${parentId}-${newNodeId}`,
      source: parentId,
      target: newNodeId,
      type: branchType,
      label: branchType === 'sub' ? '↗ 深挖' : branchType === 'divergent' ? '→ 发散' : '↓ 换角',
    }

    updateCurrentConversation((c) => ({
      ...c,
      nodes: [...c.nodes, newNode],
      edges: [...c.edges, newEdge],
      currentNodeId: newNodeId,
    }))
  }

  // Send message inside Card — 追问会沿当前卡片所在线路继续推进
  const handleSendMessage = (promptText: string, model: ModelType, webSearch: boolean) => {
    setIsGenerating(true)

    const parentNode = currentNode || nodes[0]
    const respNodeId = `${currentConversationId}-N-${Date.now()}`

    setTimeout(() => {
      const aiNode: CanvasNode = {
        id: respNodeId,
        type: 'ai_response',
        title: `AI 深度研读: ${promptText.slice(0, 16)}...`,
        model,
        parentId: parentNode.id,
        // 主线卡片上的追问继续主线；分支上的追问延续该分支
        isMainline: !!parentNode.isMainline,
        x: parentNode.x + 520,
        y: parentNode.y,
        width: 520,
        timestamp: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
        content: `基于 ${model}${
          webSearch ? '（联网实时搜索已激活）' : ''
        } 对当前卡片追问的研读解答：\n\n### 关于 "${promptText}"\n\n1. 核心原理与解答：\n我们结合前上下文进行系统化推演与验证。\n\n\`\`\`typescript\n// 核心实现片段\nexport function executeWorkflow(context: FlowContext) {\n  return context.resolve().deploy();\n}\n\`\`\`\n\n2. 建议动作：\n可随时点击卡片右上角「划重点」或下方「深挖」保持专注探索。`,
        tags: [model, '卡片追问'],
      }

      const aiEdge: Edge = {
        id: `edge-${parentNode.id}-${respNodeId}`,
        source: parentNode.id,
        target: respNodeId,
        type: 'sub',
      }

      updateCurrentConversation((c) => ({
        ...c,
        nodes: [...c.nodes, aiNode],
        edges: [...c.edges, aiEdge],
        currentNodeId: respNodeId,
      }))
      setIsGenerating(false)
    }, 1100)
  }

  return (
    <div className="w-screen h-screen overflow-hidden app-ground text-gray-100 flex gap-3 p-3 relative font-sans">
      {/* Left Navigation Sidebar */}
      <LeftSidebar
        isOpen={isLeftSidebarOpen}
        onToggle={() => setIsLeftSidebarOpen(!isLeftSidebarOpen)}
        projects={projects}
        currentProjectId={currentProjectId}
        currentConversationId={currentConversationId}
        onSelectConversation={handleSelectConversation}
        onNewChat={handleNewChat}
        viewMode={viewMode}
        onToggleViewMode={() =>
          setViewMode((prev) => (prev === 'focused_stack' ? 'macro_canvas' : 'focused_stack'))
        }
      />

      {/* Main Workspace Column */}
      <div className="flex-1 h-full min-w-0 flex flex-col relative overflow-hidden gap-3">
        {/* Top Header Bar */}
        <div className="z-20 shrink-0">
          <TopNav
            currentProject={currentProject}
            projects={projects}
            onSelectProject={(proj) =>
              handleSelectConversation(proj.id, proj.conversations[0].id)
            }
            zoom={zoom}
            onZoomIn={handleZoomIn}
            onZoomOut={handleZoomOut}
            onResetZoom={handleResetZoom}
            onOpenCmdK={() => setIsCmdKOpen(true)}
            onToggleDrawer={() => setIsDrawerOpen(!isDrawerOpen)}
            isDrawerOpen={isDrawerOpen}
            highlightCount={highlights.length}
            onOpenSettings={() => setIsSettingsOpen(true)}
            viewMode={viewMode}
            onToggleViewMode={() =>
              setViewMode((prev) => (prev === 'focused_stack' ? 'macro_canvas' : 'focused_stack'))
            }
            isLeftSidebarOpen={isLeftSidebarOpen}
            onToggleLeftSidebar={() => setIsLeftSidebarOpen(!isLeftSidebarOpen)}
          />
        </div>

        {/* Framed Workspace Stage — nested, textured surface for enclosure & depth */}
        <main className="flex-1 min-h-0 w-full relative rounded-2xl stage-surface border border-white/[0.07] overflow-hidden raise-1">
          {viewMode === 'focused_stack' ? (
            <>
              {/* Focused Card Stack View with In-Card Input */}
              <FocusedStackView
                currentNode={currentNode}
                parentNodesStack={parentStack}
                childNodes={childNodes}
                allNodes={nodes}
                mainlineRootId={mainlineRootId}
                onSelectNode={handleSelectNode}
                onBranchCard={handleBranchCard}
                onToggleMainline={handleToggleMainline}
                onReturnToMainline={handleReturnToMainline}
                onTextSelect={(nodeId, text, clientPos) => {
                  setSelectionPopup({ nodeId, text, position: clientPos })
                }}
                onSwitchToMacro={() => setViewMode('macro_canvas')}
                onDeleteNode={handleDeleteNode}
                onQuickHighlightNode={handleQuickHighlightNode}
                selectedModel={selectedModel}
                onSelectModel={setSelectedModel}
                onSendMessage={handleSendMessage}
                isGenerating={isGenerating}
              />

              {/* 会话逻辑轨迹：右下角紧凑可折叠浮层 */}
              <TrajectoryDock
                nodes={nodes}
                currentNodeId={currentNodeId}
                mainlineRootId={mainlineRootId}
                isOnBranch={!currentNode?.isMainline}
                onSelectNode={handleSelectNode}
                onExpandMacro={() => setViewMode('macro_canvas')}
                onReturnToMainline={handleReturnToMainline}
              />
            </>
          ) : (
            /* Macro Canvas View — 只呈现当前会话的脉络 */
            <>
              <Canvas
                nodes={nodes}
                edges={edges}
                zoom={zoom}
                pan={pan}
                onPanChange={setPan}
                onUpdateNodePos={handleUpdateNodePos}
                onDeleteNode={handleDeleteNode}
                onBranchCard={handleBranchCard}
                onTextSelect={(nodeId, text, clientPos) => {
                  setSelectionPopup({ nodeId, text, position: clientPos })
                }}
                focusedNodeId={currentNodeId}
              />
              {/* Bottom Command Bar in Macro Mode */}
              <BottomCommandBar
                selectedModel={selectedModel}
                onSelectModel={setSelectedModel}
                onSendMessage={handleSendMessage}
                isGenerating={isGenerating}
              />
            </>
          )}
        </main>
      </div>

      {/* Floating Selection Toolbar Popup */}
      {selectionPopup && (
        <HighlightToolbar
          position={selectionPopup.position}
          selectedText={selectionPopup.text}
          onHighlight={handleAddHighlight}
          onClose={() => setSelectionPopup(null)}
        />
      )}

      {/* Right Drawer: Highlights & Synthesis */}
      <HighlightsDrawer
        isOpen={isDrawerOpen}
        onClose={() => setIsDrawerOpen(false)}
        highlights={highlights}
        onRemoveHighlight={handleRemoveHighlight}
        onJumpToNode={(id) => handleSelectNode(id)}
        onGenerateSynthesis={() => setIsSynthesisOpen(true)}
      />

      {/* Modals */}
      <CmdKModal
        isOpen={isCmdKOpen}
        onClose={() => setIsCmdKOpen(false)}
        nodes={nodes}
        highlights={highlights}
        onSelectNode={handleSelectNode}
      />

      <SynthesisModal
        isOpen={isSynthesisOpen}
        onClose={() => setIsSynthesisOpen(false)}
        currentProject={currentProject}
        highlights={highlights}
      />

      <SettingsModal isOpen={isSettingsOpen} onClose={() => setIsSettingsOpen(false)} />
    </div>
  )
}
