# MindScape Review Lab

MindScape 的共享视觉上下文工具。讨论发生在创始人与员工06的日常对话中，Review Lab 负责让一句“我说的是这里”能够准确落到某个版本、某个坐标和某个界面元素上。

它不是代码评审平台，也不是团队聊天室。版本编译和历史归档只是为UI、交互与动效讨论提供稳定底图。

## 第一版能力

- 在历史交互预览中点击任意界面细节，生成不会随下一次构建漂移的视觉引用 ID。
- 引用同时保存版本、归一化坐标、界面状态说明、讨论内容和可识别的DOM元素提示。
- 复制引用到创始人与员工06的对话；其他员工可通过深链接、CLI或HTTP读取同一上下文。
- 员工明确点击“标记可评审版本”后才开始构建。
- 每次构建记录员工、说明、重点确认项、Git 分支、提交和未提交文件数量。
- 归档 `desktop` 的静态交互预览，历史版本不被后续构建覆盖。
- 关键节点可额外构建并归档完整 Windows Tauri 程序。
- 在一个界面中切换历史版本或并排对比两个交互预览。
- 将负责人给出的确认、修改、否决和后置意见永久绑定到对应版本。
- 检测构建期间源码是否变化；变化时将版本标成风险状态，避免误认为可复现快照。

## 核心讨论方式

1. 在 Review Lab 打开要讨论的当前版或历史版。
2. 把界面操作到希望讨论的状态，例如“正在流式输出”或“停止失败”。
3. 点击“指向界面细节”，再点击具体按钮、节点、文本或空白区域。
4. 填写当时的界面状态、这个细节是什么、希望讨论或确认什么。
5. 工具生成并自动复制类似 `UI-260818095751-2CB5` 的视觉引用。
6. 在与员工06的对话中粘贴引用；员工06通过引用看到相同版本、相同位置和相同说明。
7. 需要转交员工03、04或其他员工时，他们可以打开深链接，或在命令行查询：

```powershell
cd D:\Project\MindScape\tools\review-lab
npm run context -- UI-260818095751-2CB5
```

需要机器读取完整结构时增加 `--json`：

```powershell
npm run context -- UI-260818095751-2CB5 --json
```

所有版本数据位于 `runtime-data/versions/`。该目录已被仓库根 `.gitignore` 的 `runtime-data/` 规则忽略，不进入产品源码和 Git 历史。

## 启动

在本目录执行：

```powershell
npm run start
```

然后打开 <http://127.0.0.1:4178>。

开发工具自身需要热重载时使用：

```powershell
npm run dev
```

可以用环境变量 `REVIEW_LAB_PORT` 修改端口。服务默认只监听 `127.0.0.1`，不会暴露到局域网。

## 员工如何确认“可以编译”

所有入口最终进入同一个串行构建队列，不会并发覆盖 `desktop/dist`。

### 1. Review Lab 界面

点击“标记可评审版本”，适合负责人或员工在可视界面填写说明。

### 2. CMD / PowerShell 交互确认

在 `tools/review-lab/` 中执行：

```powershell
npm run mark
```

命令行会依次询问版本名、员工、本次推进、重点确认项和是否需要完整 Windows 构建。

员工代理或脚本可以直接非交互调用：

```powershell
npm run mark -- --title "停止状态视觉" --author "员工04" --summary "完成停止中、失败回退和 cancelled 三态" --focus "停止按钮反馈与部分内容保留" --preview
```

关键版本使用 `--full`。查看连接与构建状态使用：

```powershell
npm run status
```

### 3. Git 提交标记自动识别

Review Lab 每 10 秒检查一次新的 `HEAD`。普通提交不会触发构建；提交正文只有带以下 Trailer 才会被识别：

```text
Review-Lab: preview
Review-Title: Chat 停止交互第一版
Review-Author: 员工04
Review-Summary: 完成停止中、取消失败回退和 cancelled 呈现
Review-Focus: 请确认停止按钮位置和三态视觉层级
```

`Review-Lab` 只允许 `preview` 或 `full`。同一个提交只消费一次，服务启动时也不会补编译既有旧提交。

手动要求立即扫描当前提交：

```powershell
npm run scan
```

### 4. HTTP 连接

其他员工工具、Agent 或 CI 可以向 `POST /api/versions` 发送同一份 JSON：

```json
{
  "title": "画布视口恢复",
  "author": "员工03",
  "summary": "完成多会话视口持久化",
  "focus": "切换会话与重启后的视口是否稳定",
  "buildMode": "preview"
}
```

本机 API 默认地址为 `http://127.0.0.1:4178`。当前不对局域网开放；以后连接 GitHub、内部任务系统或远程 CI 时，需要先增加身份认证，不能直接暴露现有本机接口。

## 两种构建等级

### 轻量交互预览

运行 `desktop` 的前端生产构建，将生成的 `dist/` 完整复制到版本归档。MindScape 已有浏览器预览运行模式，因此 Chat、画布和 Mock 流式交互可以直接在 Review Lab 内操作。

### 完整 Windows 构建

先生成轻量预览，再运行 Tauri `build --no-bundle` 生成可直接启动的 Windows release `.exe`，并将发布目录中的顶层 `.exe` 复制到版本归档。Review Lab 不制作 MSI/NSIS 安装包；“启动完整版本”会启动归档副本，而不是当前源码构建。

完整构建耗时和占用空间明显高于轻量预览，只用于 Alpha 验收、里程碑和需要验证原生能力的关键节点。

## 数据与安全边界

- 服务没有任意命令执行接口，构建命令固定为 MindScape 的 `npm run build` 与 `npm run tauri -- build`。
- 服务只监听本机回环地址。
- 版本 ID 会经过格式检查，静态文件服务会检查解析后的绝对路径，防止访问归档目录之外的文件。
- API Key 不会被 Review Lab 收集或复制；静态预览使用 Mock Provider。
- 当前版本记录不可修改，只能追加评审意见。后续如需删除归档，应增加带二次确认和回收站的显式流程。

## 当前限制与下一步

- 第一版的评审意见绑定到版本，还不能直接点击界面坐标生成批注。
- 完整 Tauri 程序以独立窗口启动，不能安全嵌入浏览器 iframe。
- 当前归档位于本机；后续可增加团队共享存储，但不能把 GitHub Actions 临时 Artifact 当作永久版本库。
- 后续可接入 WebDriverIO，自动录制固定状态矩阵并生成版本间视觉差异。
