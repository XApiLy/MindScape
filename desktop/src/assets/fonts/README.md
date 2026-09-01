# MindScape 内置阅读字体资源清单

> 获取日期：2026-08-29（Asia/Shanghai）  
> 状态：官方资源与许可证已下载；七款内置预设已注册 `@font-face` 并进入字体选择器；尚未完成统一 Tauri Release 验收  
> 产品规格：[M2 首批阅读字体预设目录](../../../../docs/design/visual-and-interaction/06-reading-font-preset-catalog-m2.md)

## 1. 来源与固定版本

所有文件均直接来自字体作者/维护者的官方 GitHub 仓库或官方 Release，没有经过第三方字体站、网盘或运行时 CDN。

| 预设 | 官方来源 | 固定版本 | 本地生产候选 |
| --- | --- | --- | --- |
| Caveat | [google/fonts：Caveat](https://github.com/google/fonts/tree/5571d84c0d8c70ec1af4f64072d8c5cf1e4e9643/ofl/caveat) | `5571d84c0d8c70ec1af4f64072d8c5cf1e4e9643` | `caveat/Caveat-Variable.ttf` |
| Nunito | [google/fonts：Nunito](https://github.com/google/fonts/tree/8b0a1d0f5983c89bc2b93f1b5fb55f9e252744b5/ofl/nunito) | `8b0a1d0f5983c89bc2b93f1b5fb55f9e252744b5` | Normal/Italic 可变字体 |
| Lato | [google/fonts：Lato](https://github.com/google/fonts/tree/5d3b76120a319730fda218cc7410174a462b32cb/ofl/lato) | `5d3b76120a319730fda218cc7410174a462b32cb` | Regular/Italic/Bold/BoldItalic |
| Cormorant | [google/fonts：Cormorant](https://github.com/google/fonts/tree/3dd78844021e948ceb633d1dcee3f7885561b5d9/ofl/cormorant) | `3dd78844021e948ceb633d1dcee3f7885561b5d9` | Normal/Italic 可变字体 |
| 寒蝉半圆体 | [Warren2060/ChillRound](https://github.com/Warren2060/ChillRound/releases/tag/v1.805) | Release `v1.805` | `chill-round-m/ChillRoundM.otf` |
| 小赖字体 | [lxgw/kose-font](https://github.com/lxgw/kose-font/releases/tag/v3.126) | Release `v3.126` | `xiaolai-sc/Xiaolai-Regular.ttf` |
| 得意黑 | [atelier-anchor/smiley-sans](https://github.com/atelier-anchor/smiley-sans/releases/tag/v2.0.1) | Release `v2.0.1` | `smiley-sans/SmileySans-Oblique.woff2` |

每个子目录均保存对应 `OFL.txt`。本批字体均由其本地许可证声明为 SIL Open Font License 1.1；Lato、ChillRoundM 和 Smiley Sans 的许可证包含 Reserved Font Name 条款，后续若对子集、轮廓、名称或字体文件进行修改，必须重新核对命名要求，不能把修改版继续冒充官方原名。

## 2. 字体元数据核验

| 资源 | 实测内部 family / full name | 字重/样式说明 |
| --- | --- | --- |
| Caveat | `Caveat` / `Caveat Regular` | 可变 `wght`，Normal |
| Nunito | `Nunito` 系列 | Normal/Italic 可变 `wght`；Windows legacy name 会显示起始实例 `Nunito ExtraLight`，注册时使用 MindScape 自定义 family alias |
| Lato | `Lato` | 400、400 Italic、700、700 Italic |
| Cormorant | `Cormorant` 系列 | Normal/Italic 可变 `wght`；Windows legacy name 会显示起始实例 `Cormorant Light`，注册时使用 MindScape 自定义 family alias |
| ChillRoundM | `ChillRoundM` / `ChillRoundM` | Regular |
| Xiaolai | `Xiaolai` / `Xiaolai` | Regular；截图中的 `Xiaolai SC` 是产品/站点标识，官方 `v3.126` 文件内部 family 实测为 `Xiaolai` |
| Smiley Sans | `Smiley Sans Oblique` | Regular Oblique；官方项目明确不推荐正文、代码和手机 UI，MindScape 中保持用户主动选择并接受长文实测 |

CSS 中使用项目自己的稳定 alias，例如 `MindScape Nunito`，不要依赖变量字体的 legacy family name，也不要把任意文件名直接拼入 CSS。

## 3. 文件完整性

| 文件 | 字节 | SHA-256 |
| --- | ---: | --- |
| `caveat/Caveat-Variable.ttf` | 403648 | `0BDB6B660482D31531B3945849FBA5916B3EF8695DA7024A9E6B9EE3C4157988` |
| `caveat/OFL.txt` | 4385 | `1F9D81D094273D82F3898A1EE8B598A717D050ECBF5FF7BEDE105B704880157B` |
| `nunito/Nunito-Variable.ttf` | 276932 | `BB55A5CA5C2042335B3991AF27C4D0705D0EF41CAC6164AC737FD8F2A1E85207` |
| `nunito/Nunito-Italic-Variable.ttf` | 281832 | `B520CC871868B0ACFCA1BEDA875DF7F4A44EBCE914F8A89F83977FC9C09529C8` |
| `nunito/OFL.txt` | 4385 | `580DF76C95A1EC5AB878CEB25BB3D85C6A076804E9C970C8C6972AEA775FDF65` |
| `lato/Lato-Regular.ttf` | 656568 | `D636E4683231F931EDA222D588E944D082BFD3BDBA02F928BEE461C0F185B251` |
| `lato/Lato-Italic.ttf` | 722900 | `E399C44EFE1387100531D26C7E4800C5D12251B890D6654A3098C7C679CB1786` |
| `lato/Lato-Bold.ttf` | 656544 | `8A0AACE75D33794EECE4B28187BFC1DF0BBD2888B5D8A56E01788C8D65D16BE1` |
| `lato/Lato-BoldItalic.ttf` | 698364 | `62C1B7F0D2E74B45960154C3520EFC337B553DB0961BFDC950D5618334596CC8` |
| `lato/OFL.txt` | 4407 | `74BA064D03F1F1C4A952DA936C3EB71866C34404916734DE3CAE73B34357E59E` |
| `cormorant/Cormorant-Variable.ttf` | 572892 | `8F12CB21F05B61649192EAFF13EEEB1B5619BC524FEEAE672FB916974259A076` |
| `cormorant/Cormorant-Italic-Variable.ttf` | 350096 | `2C4E1C43FA126B51A84160815B9264AF442C6C531D13BFD3C1723703CD489DD2` |
| `cormorant/OFL.txt` | 4387 | `60700D351CAC4650C51F3F9DB318D2A420F8B45052DBA2715EB5FEC41F0F6956` |
| `chill-round-m/ChillRoundM.otf` | 4764200 | `2C8DA065414E3CAB951744F15C25B17A9AB4AE93E48DEF22004BC917550D1219` |
| `chill-round-m/OFL.txt` | 4375 | `D45891F8ADFD21368C98E803603DF1F575FF3FC4A6EC713BA7CCF0E3CBA15B28` |
| `xiaolai-sc/Xiaolai-Regular.ttf` | 22220806 | `E2F68DAF0E72777A8CF58BC83DE1B98634B251E537DDBFCA24B0AE50D1802DA2` |
| `xiaolai-sc/OFL.txt` | 4432 | `0DF7E09BE4C2C850A48BD8BEB9CD64B343AAD49CD5D3F6CFB2AD2E3D28A56CA4` |
| `smiley-sans/SmileySans-Oblique.woff2` | 1150924 | `731F22973349404B15A88A99EF3B5DD4104C0965C23B7E485C1F11E84FEA99E2` |
| `smiley-sans/OFL.txt` | 4422 | `9401F4050F1B66C26B6CCDC8B0E14A3C1CC37AAC122EDA84386F25854A9BEC72` |

当前资源合计 19 个文件、32,786,499 字节（约 31.27 MiB）。未被 CSS/TypeScript 引用前，Vite 不应把它们复制进正式产物；接线后必须记录实际安装包增量，不能只报告源码目录大小。

### 2026-08-29 工程接线记录

- 12 个生产字体文件已由 Vite 全量输出至 `dist/assets/`，实测合计 `32,755,706` 字节；许可证和本清单保留在源码树中，不作为字体运行资源加载。
- 字体选择器使用固定 preset ID 和 `MindScape …` family alias，不接收 URL、任意 CSS 或本地路径；打开选择器时通过 `document.fonts.load` 区分“内置字体可用”和“使用安全回退”。
- Chat、导入 Markdown 预览和画布聚焦阅读器继续消费共享 `--reading-font-family`；行内代码与代码块保持独立等宽字体。
- 本记录只证明前端生产资源输出和浏览器链路，不等于安装包增量或统一 Tauri Release 验收。最终安装包体积、断网、DPI、缺字和重启证据仍由统一 Release 提供。

## 4. 实施分工

- **员工04主责内置预设接线**：注册受控 `@font-face`、扩展 reading preference 枚举、字体选择器/预览、中文回退、缺失状态和重启偏好；不得读取任意路径，也不得覆盖代码块/UI 字体。
- **员工02主责用户导入字体边界**：冻结受控字体目录、格式/大小/元数据校验、复制/删除/重启恢复和 typed IPC；内置资源不经过用户导入 IPC。
- **员工01只做契约评审**：确认内置 preset ID 与用户导入 font ID 不混淆，字体偏好不进入会话、RAG、ModelRun 或 Provider。
- **员工05主责回归与发布证据**：断网、缺字、加载失败、长文、流式阅读锁定、DPI、重启和安装包体积；代码冻结后只发布一个统一 Release。
- **员工03仅检查聚焦阅读与画布性能**：画布节点仍保持轻量摘要，不因字体资源加载触发全画布重排。
- **员工06仍由创始人直管**：是否调整字体卡片、配对、字号、行高和最终视觉，只接受创始人直接指令；本清单不构成对员工06派单。

## 5. 禁止事项

- 不再由任何员工重复从字体网站下载同名文件；如需升级，先更新固定版本、来源、许可证和 hash。
- 不在运行时请求 Google Fonts、ZeoSeven API 或其他远程字体/CDN。
- 不擅自子集化、转换格式、改名或修改字形；确需优化 22.2 MB 的 Xiaolai 时先做许可证/Reserved Font Name 评审和中英文缺字回归。
- 不因字体加载失败阻塞 Chat、导入或知识页；必须回退系统字体。
- 不把当前资源就位描述为字体功能完成；只有正式选择器、真实正文、重启和统一 Release 验收通过才算完成。
