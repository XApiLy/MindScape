import { createInterface } from "node:readline/promises";
import { stdin as input, stdout as output } from "node:process";

function parseArgs(argv) {
  const result = { command: "mark", options: {}, positionals: [] };
  const args = [...argv];
  if (args[0] && !args[0].startsWith("-")) result.command = args.shift();
  while (args.length) {
    const token = args.shift();
    if (token === "--full") result.options.buildMode = "full";
    else if (token === "--preview") result.options.buildMode = "preview";
    else if (token === "--json") result.options.json = true;
    else if (token.startsWith("--")) {
      const key = token.slice(2);
      const value = args.shift();
      if (!value || value.startsWith("--")) throw new Error(`参数 ${token} 缺少值`);
      result.options[key] = value;
    } else result.positionals.push(token);
  }
  return result;
}

function baseUrl(options) {
  return options.url || process.env.REVIEW_LAB_URL || "http://127.0.0.1:4178";
}

async function request(url, path, options = {}) {
  let response;
  try {
    response = await fetch(`${url}${path}`, {
      ...options,
      headers: {
        "Content-Type": "application/json",
        "X-Review-Lab-Client": "cli",
        ...(options.headers || {}),
      },
    });
  } catch {
    throw new Error(`无法连接 Review Lab (${url})，请先在 tools/review-lab 运行 npm run start`);
  }
  const body = await response.json();
  if (!response.ok) throw new Error(body.error || `请求失败 (${response.status})`);
  return body;
}

async function fillInteractive(options) {
  if (options.title && options.author && options.summary) {
    return { buildMode: "preview", focus: "", ...options };
  }
  if (!input.isTTY) throw new Error("非交互调用必须提供 --title、--author 和 --summary");

  const terminal = createInterface({ input, output });
  try {
    const title = options.title || await terminal.question("版本名称：");
    const author = options.author || await terminal.question("提交员工（例如 员工03）：");
    const summary = options.summary || await terminal.question("本次推进内容：");
    const focus = options.focus ?? await terminal.question("请负责人重点确认（可留空）：");
    let buildMode = options.buildMode;
    if (!buildMode) {
      const full = await terminal.question("是否构建完整 Windows 版本？(y/N)：");
      buildMode = /^y(es)?$/i.test(full.trim()) ? "full" : "preview";
    }
    return { title, author, summary, focus, buildMode };
  } finally {
    terminal.close();
  }
}

async function mark(options) {
  const payload = await fillInteractive(options);
  delete payload.url;
  const body = await request(baseUrl(options), "/api/versions", {
    method: "POST",
    body: JSON.stringify(payload),
  });
  console.log("\n已确认并进入构建队列");
  console.log(`版本：${body.version.title}`);
  console.log(`ID：${body.version.id}`);
  console.log(`等级：${body.version.buildMode === "full" ? "完整 Windows 构建" : "轻量交互预览"}`);
}

async function status(options) {
  const url = baseUrl(options);
  const [system, versionsBody] = await Promise.all([
    request(url, "/api/system"),
    request(url, "/api/versions"),
  ]);
  console.log(`Review Lab：${url}`);
  console.log(`Git：${system.git.branch} @ ${system.git.shortCommit}${system.git.dirty ? ` (${system.git.dirtyFileCount} 项变更)` : ""}`);
  console.log(`构建队列：${system.queueLength}`);
  console.log(`Git 自动识别：已启用，每 ${system.automation.intervalSeconds} 秒扫描`);
  console.log(`历史版本：${versionsBody.versions.length}`);
  for (const version of versionsBody.versions.slice(0, 8)) {
    console.log(`- [${version.status}] ${version.title} · ${version.author} · ${version.trigger?.type || "legacy"}`);
  }
}

async function scan(options) {
  const body = await request(baseUrl(options), "/api/automation/scan", { method: "POST", body: "{}" });
  if (body.detected) console.log(`检测到 Git 评审信号，已创建：${body.version.title}`);
  else console.log("当前 HEAD 没有新的 Git 评审信号。");
}

async function context(options, positionals) {
  const anchorId = options.id || positionals[0];
  if (!anchorId) throw new Error("请提供视觉引用 ID，例如 npm run context -- UI-260818094512-A1B2");
  const url = baseUrl(options);
  const body = await request(url, `/api/anchors/${encodeURIComponent(anchorId)}`);
  if (options.json) {
    console.log(JSON.stringify(body, null, 2));
    return;
  }
  const { anchor, version } = body;
  console.log(`视觉引用：${anchor.id}`);
  console.log(`版本：${version.title} (${version.id})`);
  console.log(`位置：横向 ${(anchor.x * 100).toFixed(1)}% · 纵向 ${(anchor.y * 100).toFixed(1)}%`);
  if (anchor.viewLabel) console.log(`界面状态：${anchor.viewLabel}`);
  if (anchor.elementHint?.label || anchor.elementHint?.text) {
    console.log(`对应元素：${anchor.elementHint.label || anchor.elementHint.text}`);
  }
  console.log(`指向人：${anchor.author}`);
  console.log(`细节：${anchor.title}`);
  console.log(`讨论内容：${anchor.note}`);
  console.log(`打开：${url}/?version=${encodeURIComponent(version.id)}&anchor=${encodeURIComponent(anchor.id)}`);
}

function printUsage() {
  console.log(`MindScape Review Lab CLI

交互式确认：
  npm run mark

员工或代理非交互确认：
  npm run mark -- --title "停止状态视觉" --author "员工04" --summary "完成三态实现" --focus "停止按钮反馈" --preview
  npm run mark -- --title "M1 Alpha" --author "员工06" --summary "关键版本" --full

其他命令：
  npm run status
  npm run scan
  npm run context -- UI-260818094512-A1B2

环境变量 REVIEW_LAB_URL 可连接其他 Review Lab 地址。`);
}

try {
  const { command, options, positionals } = parseArgs(process.argv.slice(2));
  if (command === "mark") await mark(options);
  else if (command === "status" || command === "list") await status(options);
  else if (command === "scan") await scan(options);
  else if (command === "context" || command === "show") await context(options, positionals);
  else if (command === "help" || command === "--help") printUsage();
  else throw new Error(`未知命令：${command}`);
} catch (error) {
  console.error(`Review Lab：${error.message}`);
  process.exitCode = 1;
}
