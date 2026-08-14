这是一份为你定制的 **初版（MVP）UI/UX 界面设计规范与布局白皮书（DESIGN SPECIFICATION）**。

设计哲学遵循 **“深色液态玻璃（Dark Glassmorphism）”** 风格，整体以**沉浸式黑色/深碳灰**为底，搭配**霓虹高光与玻璃虚化**，兼具科幻极客感与极致的心流专注感。

---

### 📐 一、 整体界面布局全景图（Layout Architecture）

界面分为 **4 大核心区域**，整体结构呈 **悬浮三栏 + 中央画布** 布局：

```text
┌────────────────────────────────────────────────────────────────────────────────────────┐
│  顶栏 (Top Navigation): 项目名 | 无限缩放比例 | 命令面板(Cmd+K) | 模式/AI选择 | 设置       │
├──────────────────────────────────────────────┬─────────────────────────────────────────┤
│                                              │                                         │
│                                              │  右侧抽屉 (Right Drawer)                │
│                                              │  【核心杀手锏：重点文档与收敛中心】      │
│               中央主画布 (Canvas Area)        │                                         │
│                                              │  ┌───────────────────────────────────┐  │
│        [ 用户提问卡片 ]                      │  │ 🟡 划重点条目 1                   │  │
│               │                              │  │    └─ 来源：卡片 #A (一键跳转)    │  │
│               ▼                              │  ├───────────────────────────────────┤  │
│        [ AI 回答卡片 ]                       │  │ 🟢 划重点条目 2 (代码/SOP)        │  │
│      ┌────────┴────────┐                     │  │    └─ 来源：卡片 #B              │  │
│      ▼                 ▼                     │  └───────────────────────────────────┘  │
│ [ ↗子卡片 ]      [ →发散卡片 ]               │                                         │
│                                              │  [ ⚡ 一键 AI 总结生成报告 (Synthesis)]│
│                                              │                                         │
├──────────────────────────────────────────────┴─────────────────────────────────────────┤
│ 底部悬浮控制台 (Bottom Command Bar):  [ 🌐 模型选择 ] [ 💬 输入你的提示词... ] [ 🚀 发送 ]  │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

---

### 🎨 二、 视觉与主题规范（Theme Tokens）

* **背景底色：** 深碳极夜黑 `#0A0C10` （配合微弱的 32px 点阵网格点阵阵点作为画布背景）。
* **卡片材质（Liquid Glass）：** 
  * 背景：`rgba(18, 22, 31, 0.7)`
  * 边框：`1px solid rgba(255, 255, 255, 0.08)`
  * 磨砂效果：`backdrop-blur(16px)`
* **高光与状态色：**
  * 🟡 **划重点/金句：** 琥珀金 `#F59E0B`
  * 🟢 **代码/方法论：** 翡翠绿 `#10B981`
  * 🔴 **疑问/待办：** 珊瑚红 `#EF4444`
  * 🔵 **AI 响应/连接线：** 电光青 `#00F0FF`

---

### 🧩 三、 4 大核心组件详细设计

#### 1. 顶栏控制区（Top Navigation Bar）
* **视觉样式：** 顶栏采用半透明玻璃悬浮长条，高度 48px，吸附在屏幕上方。
* **功能布局（左中右）：**
  * **左侧：** App Logo + 当前项目名称（点击可下拉切换项目）+ 侧边栏展开按钮；
  * **中间：** 画布控制组——显示当前画布缩放百分比（如 `100%`），提供`定位到中央`按钮，以及 `Cmd + K` 搜索全局卡片输入框；
  * **右侧：** 快捷键捕获状态指示灯（绿色表示捕获后台就绪）、深浅色主题切换、设置按钮。

---

#### 2. 中央无限画布与卡片节点（Canvas & Card Nodes）
这是用户 90% 时间交互的地方，卡片分为 **“提问卡片”** 与 **“AI 回答卡片”**：

##### 📄 卡片结构设计（Card Layout）：
* **卡片头部（Card Header）：**
  * 左侧：节点 ID（如 `#Node-04`）与类型 Badge（如 `DeepSeek-V4`）；
  * 右侧：收起/展开按钮、删除按钮。
* **卡片正文（Card Body）：**
  * 支持高质量 Markdown 排版、代码高亮（带一键 Copy 按钮）、公式渲染。
  * **核心交互（划重点）：** 用鼠标拖拽选中正文中的任意文本，**瞬间弹出悬浮小工具栏**：
    * 🟡 `划重点` | 🟢 `标记代码` | 🔴 `存为待办`
* **卡片底部衍生按钮组（Card Footer）：**
  * 放置 3 个极具辨识度的发散按钮，悬浮高光反馈：
    * `↗ 子卡片`（针对局部细节深挖）
    * `→ 发散卡片`（同级平移扩展）
    * `↓ 分支卡片`（继承历史换个角度提问）

##### 🔗 节点连接线（Edges）：
* 采用**贝塞尔平滑曲线**连接父节点与子节点；
* 默认颜色为微弱半透明灰白，当鼠标悬停或选中卡片时，连接线变为**电光青发光流动效果**。

---

#### 3. 右侧“重点文档与收敛抽屉”（Highlights Drawer）
这是你的**核心杀手锏面板**，默认可以折叠，也可固定在右侧（宽度 360px）：

* **顶部：** 面板标题 `重点文档 (Highlights)` + 当前收集计数（如 `12 条重点`）。
* **中部列表区（流水线卡片）：**
  * 当用户在中央画布上划高光时，此抽屉内会**带降落动画**掉入一个新的重点卡片；
  * 每条重点卡片显示：高光颜色标签 + 提取的文本摘要；
  * **跳转按钮（定位）：** 点击卡片上的 `🔗 来自 #Node-04`，中央画布自动平滑平移并聚焦到对应的卡片节点。
* **底部固定操作区：**
  * **巨幅高亮按钮：`⚡ 一键 AI 总结/生成报告 (Synthesis AI)`**
  * 点击后弹出导出选择菜单：[ 导出为 Markdown ] | [ 同步至 Obsidian ] | [ 复制精简报告 ]。

---

#### 4. 底部悬浮主输入框（Bottom Command Bar）
* **视觉样式：** 胶囊状悬浮输入框，位于画布中下部，带电光青色外发光。
* **布局设计：**
  * **左侧：** 大模型切换下拉菜单（默认选 `DeepSeek-v4` / `Claude-3.5`）；
  * **中间：** 自适应宽度的输入框，提示词 `探索一切... (Enter 发送, Ctrl+Enter 换行)`；
  * **右侧：** 联网搜索开关（🌐 Icon）+ 发送图标按钮。

---

### 💻 四、 初版 Tailwind CSS 样式参考代码

为了方便你或前端直接上手，这里提供核心卡片（Glass Card）的样式 class 配置：

```html
<!-- 示例：初版液态玻璃卡片 CSS 配置 -->
<div class="relative w-[480px] rounded-2xl p-5 
            bg-[#12161f]/70 backdrop-blur-xl 
            border border-white/10 shadow-2xl shadow-black/50 
            hover:border-cyan-500/40 transition-all duration-300">
  
  <!-- 卡片头部 -->
  <div class="flex items-center justify-between pb-3 border-b border-white/5">
    <span class="text-xs font-mono text-cyan-400 bg-cyan-950/50 px-2 py-0.5 rounded border border-cyan-800/40">
      #Node-01 • AI Response
    </span>
    <button class="text-gray-400 hover:text-white text-xs">✕</button>
  </div>

  <!-- 卡片正文区 -->
  <div class="py-4 text-sm text-gray-200 leading-relaxed font-sans">
    这里是 AI 回答内容，选中文本将自动触发 <mark class="bg-amber-500/30 text-amber-200 px-1 rounded">划重点</mark> 功能...
  </div>

  <!-- 卡片底部三向衍生按钮 -->
  <div class="flex items-center gap-2 pt-3 border-t border-white/5 text-xs font-medium">
    <button class="flex-1 py-1.5 rounded-lg bg-white/5 hover:bg-cyan-500/20 hover:text-cyan-300 text-gray-300 border border-white/5 border-hover:cyan-500/30 transition">
      ↗ 深挖子卡片
    </button>
    <button class="flex-1 py-1.5 rounded-lg bg-white/5 hover:bg-emerald-500/20 hover:text-emerald-300 text-gray-300 border border-white/5 border-hover:emerald-500/30 transition">
      → 平行发散
    </button>
    <button class="flex-1 py-1.5 rounded-lg bg-white/5 hover:bg-purple-500/20 hover:text-purple-300 text-gray-300 border border-white/5 border-hover:purple-500/30 transition">
      ↓ 换角分支
    </button>
  </div>
</div>
```

---

### 🎯 初版界面开发优先级总结

1. **第一步：** 渲染带有深色网格背景的无限画布（PixiJS v8 视口）；
2. **第二步：** 渲染上述样式的 HTML 卡片节点，并用贝塞尔曲线把它们连起来；
3. **第三步：** 实现右侧“重点文档”折叠抽屉，监听卡片的选中文本事件；
4. **第四步：** 加上底部悬浮输入框，跑通第一个完整的提问、衍生与划重点流程！