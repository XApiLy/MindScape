import { createHash } from "node:crypto";
import { execFile, spawn } from "node:child_process";
import {
  cp,
  mkdir,
  open,
  readFile,
  readdir,
  stat,
  writeFile,
} from "node:fs/promises";
import { createServer } from "node:http";
import { extname, join, relative, resolve, sep } from "node:path";
import { promisify } from "node:util";
import {
  createVersionId,
  createAnchorId,
  parseGitReviewSignal,
  publicVersion,
  validateAnchorInput,
  validateReviewInput,
  validateVersionInput,
} from "./lib.mjs";

const execFileAsync = promisify(execFile);
const TOOL_ROOT = resolve(import.meta.dirname);
const REPOSITORY_ROOT = resolve(TOOL_ROOT, "..", "..");
const DESKTOP_ROOT = join(REPOSITORY_ROOT, "desktop");
const DATA_ROOT = join(TOOL_ROOT, "runtime-data");
const VERSIONS_ROOT = join(DATA_ROOT, "versions");
const AUTOMATION_FILE = join(DATA_ROOT, "automation.json");
const PUBLIC_ROOT = join(TOOL_ROOT, "public");
const PORT = Number(process.env.REVIEW_LAB_PORT || 4178);
const HOST = process.env.REVIEW_LAB_HOST || "127.0.0.1";
const clients = new Set();
const queue = [];
let queueRunning = false;

const mimeTypes = {
  ".css": "text/css; charset=utf-8",
  ".html": "text/html; charset=utf-8",
  ".ico": "image/x-icon",
  ".js": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".png": "image/png",
  ".svg": "image/svg+xml",
  ".webp": "image/webp",
  ".woff2": "font/woff2",
};

await mkdir(VERSIONS_ROOT, { recursive: true });

async function readAutomationState() {
  try {
    return JSON.parse(await readFile(AUTOMATION_FILE, "utf8"));
  } catch {
    return { lastSeenCommit: null, lastScanAt: null, lastSignal: null };
  }
}

let automationState = await readAutomationState();

async function writeAutomationState() {
  await writeFile(AUTOMATION_FILE, `${JSON.stringify(automationState, null, 2)}\n`, "utf8");
  broadcast("automation", automationState);
}

function sendJson(response, statusCode, value) {
  response.writeHead(statusCode, {
    "Cache-Control": "no-store",
    "Content-Type": "application/json; charset=utf-8",
  });
  response.end(JSON.stringify(value));
}

function sendError(response, statusCode, error) {
  sendJson(response, statusCode, { error: error instanceof Error ? error.message : String(error) });
}

async function readJsonBody(request) {
  const chunks = [];
  let size = 0;
  for await (const chunk of request) {
    size += chunk.length;
    if (size > 1024 * 1024) throw new Error("请求内容过大");
    chunks.push(chunk);
  }
  if (chunks.length === 0) return {};
  return JSON.parse(Buffer.concat(chunks).toString("utf8"));
}

function broadcast(type, payload) {
  const message = `event: ${type}\ndata: ${JSON.stringify(payload)}\n\n`;
  for (const client of clients) client.write(message);
}

async function readVersion(id) {
  if (!/^[\p{L}\p{N}._-]+$/u.test(id)) return null;
  try {
    return JSON.parse(await readFile(join(VERSIONS_ROOT, id, "manifest.json"), "utf8"));
  } catch {
    return null;
  }
}

async function writeVersion(version) {
  const versionRoot = join(VERSIONS_ROOT, version.id);
  await mkdir(versionRoot, { recursive: true });
  await writeFile(join(versionRoot, "manifest.json"), `${JSON.stringify(version, null, 2)}\n`, "utf8");
  broadcast("version", publicVersion(version));
}

async function listVersions() {
  const entries = await readdir(VERSIONS_ROOT, { withFileTypes: true });
  const versions = await Promise.all(
    entries.filter((entry) => entry.isDirectory()).map((entry) => readVersion(entry.name)),
  );
  return versions
    .filter(Boolean)
    .sort((left, right) => right.createdAt.localeCompare(left.createdAt))
    .map(publicVersion);
}

async function findAnchor(anchorId) {
  const versions = await listVersions();
  for (const summary of versions) {
    const version = await readVersion(summary.id);
    const anchor = version?.anchors?.find((candidate) => candidate.id.toLowerCase() === anchorId.toLowerCase());
    if (anchor) return { anchor, version: publicVersion(version) };
  }
  return null;
}

async function gitInfo() {
  try {
    const [{ stdout: branch }, { stdout: commit }, { stdout: statusText }] = await Promise.all([
      execFileAsync("git", ["branch", "--show-current"], { cwd: REPOSITORY_ROOT }),
      execFileAsync("git", ["rev-parse", "HEAD"], { cwd: REPOSITORY_ROOT }),
      execFileAsync("git", ["status", "--short"], { cwd: REPOSITORY_ROOT, maxBuffer: 4 * 1024 * 1024 }),
    ]);
    const dirtyFiles = statusText.split(/\r?\n/).filter(Boolean);
    return {
      branch: branch.trim() || "detached",
      commit: commit.trim(),
      shortCommit: commit.trim().slice(0, 8),
      dirty: dirtyFiles.length > 0,
      dirtyFileCount: dirtyFiles.length,
    };
  } catch (error) {
    return { branch: "unknown", commit: "unknown", shortCommit: "unknown", dirty: true, dirtyFileCount: 0 };
  }
}

async function sourceFingerprint() {
  const hash = createHash("sha256");
  const roots = [
    join(DESKTOP_ROOT, "src"),
    join(DESKTOP_ROOT, "src-tauri", "src"),
  ];
  const singleFiles = [
    join(DESKTOP_ROOT, "package.json"),
    join(DESKTOP_ROOT, "vite.config.ts"),
    join(DESKTOP_ROOT, "src-tauri", "Cargo.toml"),
    join(DESKTOP_ROOT, "src-tauri", "Cargo.lock"),
    join(DESKTOP_ROOT, "src-tauri", "tauri.conf.json"),
  ];

  async function addTree(root) {
    const entries = await readdir(root, { withFileTypes: true });
    entries.sort((a, b) => a.name.localeCompare(b.name));
    for (const entry of entries) {
      const path = join(root, entry.name);
      if (entry.isDirectory()) await addTree(path);
      else if (entry.isFile()) {
        hash.update(relative(REPOSITORY_ROOT, path));
        hash.update(await readFile(path));
      }
    }
  }

  for (const root of roots) await addTree(root);
  for (const path of singleFiles) {
    hash.update(relative(REPOSITORY_ROOT, path));
    hash.update(await readFile(path));
  }
  return hash.digest("hex");
}

async function runLogged(command, args, options, logPath) {
  const logHandle = await open(logPath, "a");
  await logHandle.appendFile(`\n> ${command} ${args.join(" ")}\n\n`);
  return new Promise((resolvePromise, rejectPromise) => {
    const executable = process.platform === "win32"
      ? (process.env.ComSpec || "C:\\Windows\\System32\\cmd.exe")
      : command;
    const executableArgs = process.platform === "win32"
      ? ["/d", "/s", "/c", command, ...args]
      : args;
    const child = spawn(executable, executableArgs, {
      ...options,
      env: { ...process.env, CI: "true", NO_COLOR: "1" },
      windowsHide: true,
    });
    child.stdout.on("data", (chunk) => void logHandle.appendFile(chunk));
    child.stderr.on("data", (chunk) => void logHandle.appendFile(chunk));
    child.on("error", async (error) => {
      await logHandle.close();
      rejectPromise(error);
    });
    child.on("close", async (code) => {
      await logHandle.appendFile(`\n[exit ${code}]\n`);
      await logHandle.close();
      if (code === 0) resolvePromise();
      else rejectPromise(new Error(`构建命令失败，退出码 ${code}`));
    });
  });
}

async function collectFullArtifacts(versionRoot) {
  const releaseRoot = join(DESKTOP_ROOT, "src-tauri", "target", "release");
  const targetRoot = join(versionRoot, "native");
  await mkdir(targetRoot, { recursive: true });
  const copied = [];
  try {
    const entries = await readdir(releaseRoot, { withFileTypes: true });
    for (const entry of entries) {
      if (entry.isFile() && entry.name.toLowerCase().endsWith(".exe")) {
        await cp(join(releaseRoot, entry.name), join(targetRoot, entry.name), { force: true });
        copied.push(entry.name);
      }
    }
  } catch {
    // The build error is reported separately; this only collects optional outputs.
  }
  return copied;
}

async function buildVersion(id) {
  const version = await readVersion(id);
  if (!version) return;
  const versionRoot = join(VERSIONS_ROOT, version.id);
  const logPath = join(versionRoot, "build.log");
  version.status = "building";
  version.startedAt = new Date().toISOString();
  await writeVersion(version);

  try {
    const fingerprintBefore = await sourceFingerprint();
    const npmCommand = process.platform === "win32" ? "npm.cmd" : "npm";
    await runLogged(npmCommand, ["run", "build"], { cwd: DESKTOP_ROOT }, logPath);
    const previewRoot = join(versionRoot, "preview");
    await cp(join(DESKTOP_ROOT, "dist"), previewRoot, { recursive: true, force: true });
    version.previewUrl = `/artifacts/${encodeURIComponent(version.id)}/preview/index.html`;

    if (version.buildMode === "full") {
      await runLogged(npmCommand, ["run", "tauri", "--", "build", "--no-bundle"], { cwd: DESKTOP_ROOT }, logPath);
      version.nativeArtifacts = await collectFullArtifacts(versionRoot);
    }

    const fingerprintAfter = await sourceFingerprint();
    version.sourceFingerprint = fingerprintBefore;
    version.sourceChangedDuringBuild = fingerprintBefore !== fingerprintAfter;
    version.status = fingerprintBefore === fingerprintAfter ? "ready" : "warning";
    version.completedAt = new Date().toISOString();
  } catch (error) {
    version.status = "failed";
    version.error = error instanceof Error ? error.message : String(error);
    version.completedAt = new Date().toISOString();
  }
  await writeVersion(version);
}

async function processQueue() {
  if (queueRunning) return;
  queueRunning = true;
  while (queue.length > 0) {
    const id = queue.shift();
    await buildVersion(id);
  }
  queueRunning = false;
}

function enqueue(id) {
  queue.push(id);
  void processQueue();
}

async function createQueuedVersion(rawInput, trigger = { type: "api" }) {
  const input = validateVersionInput(rawInput);
  const git = await gitInfo();
  const version = {
    id: createVersionId(input.title),
    ...input,
    createdAt: new Date().toISOString(),
    status: "queued",
    git,
    trigger,
    reviews: [],
    anchors: [],
  };
  await writeVersion(version);
  enqueue(version.id);
  return version;
}

async function scanGitReviewSignal({ initialize = false } = {}) {
  const { stdout } = await execFileAsync(
    "git",
    ["log", "-1", "--pretty=format:%H%x00%an%x00%s%x00%B"],
    { cwd: REPOSITORY_ROOT, maxBuffer: 1024 * 1024 },
  );
  const [commit, gitAuthor, subject, body] = stdout.split("\u0000");
  automationState.lastScanAt = new Date().toISOString();

  if (!automationState.lastSeenCommit || initialize) {
    automationState.lastSeenCommit = commit;
    await writeAutomationState();
    return { detected: false, initialized: true, commit };
  }
  if (automationState.lastSeenCommit === commit) {
    await writeAutomationState();
    return { detected: false, unchanged: true, commit };
  }

  automationState.lastSeenCommit = commit;
  const signal = parseGitReviewSignal({ commit, gitAuthor, subject, body });
  if (!signal) {
    automationState.lastSignal = { type: "git", commit, detected: false, at: automationState.lastScanAt };
    await writeAutomationState();
    return { detected: false, commit };
  }

  const version = await createQueuedVersion(signal.input, signal.trigger);
  automationState.lastSignal = { type: "git", commit, detected: true, versionId: version.id, at: automationState.lastScanAt };
  await writeAutomationState();
  return { detected: true, commit, version: publicVersion(version) };
}

async function findLaunchableExe(version) {
  if (!version.nativeArtifacts?.length) return null;
  const nativeRoot = join(VERSIONS_ROOT, version.id, "native");
  const candidates = [];
  async function walk(root) {
    const entries = await readdir(root, { withFileTypes: true });
    for (const entry of entries) {
      const path = join(root, entry.name);
      if (entry.isDirectory()) await walk(path);
      else if (entry.name.toLowerCase().endsWith(".exe")) candidates.push(path);
    }
  }
  await walk(nativeRoot);
  return candidates.find((path) => !/uninstall|setup|installer/i.test(path)) ?? candidates[0] ?? null;
}

async function serveFile(response, root, requestPath, cacheControl = "no-store") {
  const decoded = decodeURIComponent(requestPath);
  const requested = resolve(root, `.${decoded.startsWith("/") ? decoded : `/${decoded}`}`);
  if (requested !== root && !requested.startsWith(`${root}${sep}`)) {
    sendError(response, 403, new Error("路径无效"));
    return;
  }
  try {
    const fileStat = await stat(requested);
    const filePath = fileStat.isDirectory() ? join(requested, "index.html") : requested;
    response.writeHead(200, {
      "Cache-Control": cacheControl,
      "Content-Type": mimeTypes[extname(filePath).toLowerCase()] || "application/octet-stream",
    });
    response.end(await readFile(filePath));
  } catch {
    sendError(response, 404, new Error("文件不存在"));
  }
}

const server = createServer(async (request, response) => {
  const url = new URL(request.url || "/", `http://${request.headers.host || `${HOST}:${PORT}`}`);
  try {
    if (request.method === "GET" && url.pathname === "/api/events") {
      response.writeHead(200, {
        "Cache-Control": "no-cache",
        Connection: "keep-alive",
        "Content-Type": "text/event-stream",
      });
      response.write("event: connected\ndata: {}\n\n");
      clients.add(response);
      request.on("close", () => clients.delete(response));
      return;
    }

    if (request.method === "GET" && url.pathname === "/api/system") {
      sendJson(response, 200, {
        repositoryRoot: REPOSITORY_ROOT,
        desktopRoot: DESKTOP_ROOT,
        dataRoot: DATA_ROOT,
        queueLength: queue.length + (queueRunning ? 1 : 0),
        git: await gitInfo(),
        automation: {
          enabled: true,
          intervalSeconds: 10,
          trailer: "Review-Lab: preview|full",
          ...automationState,
        },
      });
      return;
    }

    if (request.method === "GET" && url.pathname === "/api/versions") {
      sendJson(response, 200, { versions: await listVersions() });
      return;
    }

    if (request.method === "POST" && url.pathname === "/api/versions") {
      const source = request.headers["x-review-lab-client"] === "cli" ? "cli" : "gui";
      const version = await createQueuedVersion(await readJsonBody(request), { type: source });
      sendJson(response, 202, { version: publicVersion(version) });
      return;
    }

    if (request.method === "POST" && url.pathname === "/api/automation/scan") {
      sendJson(response, 200, await scanGitReviewSignal());
      return;
    }

    const reviewMatch = url.pathname.match(/^\/api\/versions\/([^/]+)\/reviews$/);
    if (request.method === "POST" && reviewMatch) {
      const version = await readVersion(decodeURIComponent(reviewMatch[1]));
      if (!version) return sendError(response, 404, new Error("版本不存在"));
      const review = {
        id: `review-${Date.now()}-${Math.random().toString(16).slice(2, 6)}`,
        ...validateReviewInput(await readJsonBody(request)),
        createdAt: new Date().toISOString(),
      };
      version.reviews = [...(version.reviews || []), review];
      await writeVersion(version);
      sendJson(response, 201, { review, version: publicVersion(version) });
      return;
    }

    const anchorMatch = url.pathname.match(/^\/api\/versions\/([^/]+)\/anchors$/);
    if (request.method === "POST" && anchorMatch) {
      const version = await readVersion(decodeURIComponent(anchorMatch[1]));
      if (!version) return sendError(response, 404, new Error("版本不存在"));
      if (!version.previewUrl) return sendError(response, 409, new Error("此版本没有可定位的界面预览"));
      const anchor = {
        id: createAnchorId(),
        ...validateAnchorInput(await readJsonBody(request)),
        createdAt: new Date().toISOString(),
      };
      version.anchors = [...(version.anchors || []), anchor];
      await writeVersion(version);
      sendJson(response, 201, { anchor, version: publicVersion(version) });
      return;
    }

    const anchorLookupMatch = url.pathname.match(/^\/api\/anchors\/([^/]+)$/);
    if (request.method === "GET" && anchorLookupMatch) {
      const found = await findAnchor(decodeURIComponent(anchorLookupMatch[1]));
      if (!found) return sendError(response, 404, new Error("视觉引用不存在"));
      sendJson(response, 200, found);
      return;
    }

    const launchMatch = url.pathname.match(/^\/api\/versions\/([^/]+)\/launch$/);
    if (request.method === "POST" && launchMatch) {
      const version = await readVersion(decodeURIComponent(launchMatch[1]));
      if (!version) return sendError(response, 404, new Error("版本不存在"));
      const executable = await findLaunchableExe(version);
      if (!executable) return sendError(response, 409, new Error("此版本没有可启动的完整构建"));
      const child = spawn(executable, [], { cwd: join(VERSIONS_ROOT, version.id, "native"), detached: true, stdio: "ignore", windowsHide: false });
      child.unref();
      sendJson(response, 202, { launched: true });
      return;
    }

    const logMatch = url.pathname.match(/^\/api\/versions\/([^/]+)\/log$/);
    if (request.method === "GET" && logMatch) {
      const version = await readVersion(decodeURIComponent(logMatch[1]));
      if (!version) return sendError(response, 404, new Error("版本不存在"));
      let log = "";
      try {
        log = await readFile(join(VERSIONS_ROOT, version.id, "build.log"), "utf8");
      } catch {
        log = "构建尚未开始。";
      }
      sendJson(response, 200, { log });
      return;
    }

    const artifactMatch = url.pathname.match(/^\/artifacts\/([^/]+)(\/.*)?$/);
    if (request.method === "GET" && artifactMatch) {
      const id = decodeURIComponent(artifactMatch[1]);
      const version = await readVersion(id);
      if (!version) return sendError(response, 404, new Error("版本不存在"));
      const subpath = artifactMatch[2] || "/";
      await serveFile(response, join(VERSIONS_ROOT, id), subpath, "public, max-age=31536000, immutable");
      return;
    }

    if (request.method === "GET") {
      const path = url.pathname === "/" ? "/index.html" : url.pathname;
      await serveFile(response, PUBLIC_ROOT, path);
      return;
    }

    sendError(response, 404, new Error("接口不存在"));
  } catch (error) {
    sendError(response, 400, error);
  }
});

server.listen(PORT, HOST, () => {
  console.log(`MindScape Review Lab: http://${HOST}:${PORT}`);
  console.log(`Version archive: ${VERSIONS_ROOT}`);
  void scanGitReviewSignal({ initialize: !automationState.lastSeenCommit }).catch((error) => {
    console.error(`Git review signal initialization failed: ${error.message}`);
  });
});

setInterval(() => {
  void scanGitReviewSignal().catch((error) => {
    automationState.lastScanAt = new Date().toISOString();
    automationState.lastSignal = { type: "git", detected: false, error: error.message, at: automationState.lastScanAt };
    void writeAutomationState();
  });
}, 10_000).unref();
