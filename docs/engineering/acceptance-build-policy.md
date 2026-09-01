# MindScape 验收程序构建与交付规则

> 状态：立即生效  
> 适用范围：员工01～06及所有后续参与MindScape构建、联调和验收的人员  
> 权威入口：`artifacts/acceptance/`

## 1. 目的

编译缓存、工程构建产物和交给创始人验收的程序是三种不同对象。禁止继续把`target/`里的程序、桌面根目录的临时EXE或`target-fixed`一类临时目录直接称为“新验收程序”。

本规则确保每个验收程序都有唯一版本、明确路径、来源提交、源程序编译时间、发布时间和SHA-256，避免运行错版本、旧程序占用导致重复构建目录，以及二进制污染Git仓库。

## 与 Review Lab 的边界

Review Lab 员工主线 `preview` 是可视进度快照，可以来自明确标注的 dirty 工作树，用来让创始人及时观察 UI、交互和状态；它不是 Windows 验收程序，也不证明原生 Provider、SQLite、凭据或恢复能力。可见 UI 切片的 preview 发布频率和员工责任以当前控制 PEC 为准。

本目录发布的 Acceptance Build 是唯一正式 Tauri Release。它发布后必须在 Review Lab 形成同一 Build ID、commit、SHA-256、程序路径与验收范围的 `full` 里程碑引用，但登记动作不得再次编译或产生第二份等价 EXE。自动登记接口尚未落地时，Release PEC 必须显式报告该缺口，禁止把 Acceptance Build 已发布误写成 Review Lab 已同步。

## 2. 唯一目录结构

```text
artifacts/acceptance/
├── README.md
├── LATEST.txt
└── versions/
    └── <时间>-<任务标签>-<提交>-<dirty可选>/
        ├── mindscape-desktop.exe
        ├── manifest.json
        └── SHA256SUMS.txt
```

- `versions/<build-id>/`一经发布即视为不可变，不覆盖、不改名。
- `LATEST.txt`只指向最近一次成功发布的版本。
- 默认只保留最近5个本地验收版本；运行中的旧版本无法清理时，关闭程序后再清理。
- 该目录中的生成文件全部被Git忽略，只有目录说明和本规则进入Git。

## 3. 标准发布命令

从仓库根目录执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/publish-acceptance.ps1 -Label <task-id>
```

脚本必须完成：

1. 使用Tauri CLI的release模式和标准`desktop/src-tauri/target/`编译缓存构建内嵌前端的Windows程序。
2. 将可执行程序复制到新的不可变验收版本目录。
3. 生成`manifest.json`和`SHA256SUMS.txt`。
4. 更新`LATEST.txt`。
5. 清理超出留存数量的旧验收版本。

脚本不接受现成EXE作为来源，也不提供跳过构建参数。Rust的`cargo build`成功不等于Tauri应用可独立运行，任何验收版本必须由本次脚本内的`pnpm tauri build --no-bundle`直接产生。

发布后还必须在本机没有`localhost:1420`前端开发服务的条件下，从`artifacts/acceptance/versions/<build-id>/`实际启动一次。验收负责人必须看到MindScape真实工作区，不得只检查进程存活或窗口标题；真实工作区截图或现场确认完成后，该Build ID才可通知创始人。

## 4. 强制禁止事项

- 禁止让创始人直接运行`desktop/src-tauri/target/.../mindscape-desktop.exe`进行验收。
- 禁止把`cargo build`或`cargo test`产生的调试EXE当作Tauri验收程序；出现连接`localhost`的页面即判定构建无效。
- 禁止在`desktop/`、仓库根目录或个人临时目录复制散装EXE。
- 禁止创建`target-fixed`、`target-new`、`target-final`、`target-final-2`等替代编译目录解决文件占用。
- 禁止只发送“mindscape-desktop.exe”文件名而不提供Build ID和完整路径。
- 禁止覆盖已经交付的验收版本；修复后必须发布新Build ID。
- 禁止将EXE、MSI、构建缓存、API Key、私人会话或本地数据库提交到Git。
- 禁止把Preview、Review Lab静态版本或浏览器页面冒充真实Windows程序验收。

如果验收程序正在运行而影响下一次构建，应让验收者关闭旧程序；不得通过制造新的`target-*`目录绕过。

## 5. 每次交付必须提供

员工向创始人发送验收通知时必须使用以下格式：

```text
验收任务：<TASK-ID和目的>
Build ID：<唯一ID>
程序路径：D:\Project\MindScape\artifacts\acceptance\versions\<ID>\mindscape-desktop.exe
SHA-256：<64位哈希>
来源提交：<commit；dirty时必须明确说明>
本次只验收：<操作和预期结果>
已知未通过：<没有则写无>
```

不得只写“新验收程序：mindscape-desktop.exe”。

## 6. PEC与证据要求

- PEC引用验收程序时必须写Build ID、SHA-256、验收范围和结果。
- PEC必须同时写明“无开发服务器独立启动”结果；只证明进程存在或窗口标题为MindScape不算通过。
- 截图、录屏和结构化验收说明可以进入脱敏证据目录；二进制本体不进入`docs/`或PEC目录。
- 如果工作树为dirty，`manifest.json`会明确记录。里程碑最终Go版本原则上必须来自可追溯的clean commit。
- 同一次验收的所有员工必须引用同一个Build ID，不能各自编译一个“等价版本”。
- 验收失败时保留原Build ID和失败证据，修复后发布新Build ID，不覆盖历史。

## 7. 编译缓存边界

`desktop/src-tauri/target/`是Rust/Tauri标准本地编译缓存，可以保留以提高工程构建速度，但它不是交付目录、不是版本历史，也不是验收事实。需要释放磁盘时可以在确认没有构建进程后清理，之后由标准构建命令重新生成。

前端`desktop/dist/`、TypeScript增量文件和Review Lab构建归档同样属于生成内容，不得作为正式验收程序散落或提交。

## 8. 责任

- 触发验收的任务负责人负责发布版本并填写完整通知。
- 员工01负责在M1门禁表中确认所有人引用同一Build ID。
- 员工06负责该Build ID对应的视觉引用与界面验收，不重新编译另一份程序。
- 创始人只从`artifacts/acceptance/versions/`打开程序；路径不符合本规则时可以直接拒绝验收。
