const initialQuery = new URLSearchParams(window.location.search);
const state = {
  versions: [],
  selectedId: initialQuery.get("version"),
  compareId: null,
  comparing: false,
  system: null,
  pointing: false,
  pendingAnchor: null,
  activeAnchorId: initialQuery.get("anchor"),
};

const $ = (selector) => document.querySelector(selector);
const elements = {
  versionList: $("#version-list"),
  versionCount: $("#version-count"),
  systemCard: $("#system-card"),
  emptyState: $("#empty-state"),
  previewLayout: $("#preview-layout"),
  primaryPreview: $("#primary-preview"),
  primaryBuildState: $("#primary-build-state"),
  primaryLabel: $("#primary-label"),
  compareColumn: $("#compare-column"),
  comparePreview: $("#compare-preview"),
  compareBuildState: $("#compare-build-state"),
  compareSelect: $("#compare-select"),
  inspectorEmpty: $("#inspector-empty"),
  inspectorContent: $("#inspector-content"),
  toggleCompare: $("#toggle-compare"),
  launchVersion: $("#launch-version"),
  showLog: $("#show-log"),
  pointDetail: $("#point-detail"),
  annotationLayer: $("#annotation-layer"),
  anchorList: $("#anchor-list"),
  anchorDialog: $("#anchor-dialog"),
  anchorForm: $("#anchor-form"),
  anchorMessage: $("#anchor-message"),
  anchorSubmit: $("#anchor-submit"),
  elementContext: $("#element-context"),
  createDialog: $("#create-dialog"),
  createForm: $("#create-form"),
  createMessage: $("#create-message"),
  createSubmit: $("#create-submit"),
  reviewForm: $("#review-form"),
  reviewList: $("#review-list"),
  logDialog: $("#log-dialog"),
  buildLog: $("#build-log"),
  toast: $("#toast"),
};

const statusLabels = {
  queued: "等待构建",
  building: "正在构建",
  ready: "可评审",
  warning: "构建有风险",
  failed: "构建失败",
};

const verdictLabels = {
  approved: "确认方向",
  changes_requested: "需要修改",
  rejected: "明确否决",
  deferred: "后置处理",
};

const triggerLabels = {
  gui: "Review Lab 界面确认",
  cli: "CMD / PowerShell 确认",
  "git-trailer": "Git 提交标记自动识别",
  api: "HTTP 接口",
};

function escapeHtml(value) {
  return String(value ?? "").replace(/[&<>'"]/g, (character) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", "'": "&#39;", '"': "&quot;",
  })[character]);
}

function formatTime(value, includeDate = true) {
  if (!value) return "—";
  return new Intl.DateTimeFormat("zh-CN", {
    month: includeDate ? "2-digit" : undefined,
    day: includeDate ? "2-digit" : undefined,
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).format(new Date(value));
}

async function api(path, options = {}) {
  const response = await fetch(path, {
    ...options,
    headers: { "Content-Type": "application/json", ...(options.headers || {}) },
  });
  const body = await response.json();
  if (!response.ok) throw new Error(body.error || `请求失败 (${response.status})`);
  return body;
}

let toastTimer;
function toast(message) {
  elements.toast.textContent = message;
  elements.toast.classList.add("visible");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => elements.toast.classList.remove("visible"), 2600);
}

function selectedVersion() {
  return state.versions.find((version) => version.id === state.selectedId) || null;
}

function compareVersion() {
  return state.versions.find((version) => version.id === state.compareId) || null;
}

function anchorReference(version, anchor) {
  const url = `${window.location.origin}/?version=${encodeURIComponent(version.id)}&anchor=${encodeURIComponent(anchor.id)}`;
  return `[${anchor.id}] ${version.title} → ${anchor.title}\n${url}`;
}

async function copyText(value) {
  try {
    await navigator.clipboard.writeText(value);
  } catch {
    const input = document.createElement("textarea");
    input.value = value;
    input.style.position = "fixed";
    input.style.opacity = "0";
    document.body.append(input);
    input.select();
    document.execCommand("copy");
    input.remove();
  }
}

function updateDeepLink(versionId, anchorId = null) {
  const query = new URLSearchParams();
  if (versionId) query.set("version", versionId);
  if (anchorId) query.set("anchor", anchorId);
  history.replaceState(null, "", `${location.pathname}${query.size ? `?${query}` : ""}`);
}

function elementSelector(element) {
  if (!element || element.nodeType !== Node.ELEMENT_NODE) return "";
  if (element.id) return `#${CSS.escape(element.id)}`;
  const testId = element.getAttribute("data-testid");
  if (testId) return `[data-testid="${testId.replace(/"/g, "\\\"")}"]`;
  const parts = [];
  let current = element;
  while (current && current.tagName && parts.length < 4) {
    let part = current.tagName.toLowerCase();
    const classes = [...current.classList].slice(0, 2);
    if (classes.length) part += `.${classes.map((item) => CSS.escape(item)).join(".")}`;
    parts.unshift(part);
    current = current.parentElement;
  }
  return parts.join(" > ");
}

function inspectPreviewElement(x, y) {
  try {
    const frame = elements.primaryPreview;
    const document = frame.contentDocument;
    const target = document?.elementFromPoint(x * frame.clientWidth, y * frame.clientHeight);
    if (!target) return { tag: "", role: "", label: "", text: "", selector: "" };
    return {
      tag: target.tagName?.toLowerCase() || "",
      role: target.getAttribute?.("role") || "",
      label: target.getAttribute?.("aria-label") || target.getAttribute?.("title") || "",
      text: (target.textContent || "").replace(/\s+/g, " ").trim().slice(0, 240),
      selector: elementSelector(target),
    };
  } catch {
    return { tag: "", role: "", label: "", text: "", selector: "" };
  }
}

function renderSystem() {
  const system = state.system;
  if (!system) return;
  const dirty = system.git.dirty;
  elements.systemCard.innerHTML = `
    <div class="system-line"><span class="pulse ${dirty ? "dirty" : ""}"></span><span>${dirty ? `工作区有 ${system.git.dirtyFileCount} 项变更` : "工程工作区干净"}</span></div>
    <div class="system-meta">${escapeHtml(system.git.branch)} · ${escapeHtml(system.git.shortCommit)}${system.queueLength ? ` · ${system.queueLength} 个构建中` : ""}</div>
    <div class="system-meta">Git 自动识别已连接 · ${system.automation.intervalSeconds}s</div>
  `;
}

function renderVersionList() {
  elements.versionCount.textContent = state.versions.length;
  if (!state.versions.length) {
    elements.versionList.innerHTML = `<div class="review-empty">还没有可评审版本</div>`;
    return;
  }
  elements.versionList.innerHTML = state.versions.map((version) => `
    <button class="version-item ${version.id === state.selectedId ? "selected" : ""}" data-version-id="${escapeHtml(version.id)}" type="button">
      <span class="timeline-glyph"><i class="status-dot ${escapeHtml(version.status)}"></i></span>
      <span class="version-copy">
        <strong>${escapeHtml(version.title)}</strong>
        <small><span>${escapeHtml(version.author)}</span><span>${formatTime(version.createdAt)}</span></small>
      </span>
    </button>
  `).join("");
  elements.versionList.querySelectorAll("[data-version-id]").forEach((button) => {
    button.addEventListener("click", () => {
      state.selectedId = button.dataset.versionId;
      state.activeAnchorId = null;
      state.pointing = false;
      if (state.compareId === state.selectedId) state.compareId = chooseDefaultCompareId();
      updateDeepLink(state.selectedId);
      render();
    });
  });
}

function buildStateMarkup(version) {
  if (!version) return `<strong>没有选择对比版本</strong>`;
  if (version.status === "building" || version.status === "queued") {
    return `<div><div class="loader"></div><strong>${statusLabels[version.status]}</strong>完成后会自动刷新，不需要重复操作。</div>`;
  }
  if (version.status === "failed") {
    return `<div><strong>构建未完成</strong>${escapeHtml(version.error || "请查看构建记录定位问题。")}</div>`;
  }
  if (version.status === "warning") {
    return `<div><strong>源码在构建期间发生变化</strong>预览已保存，但不能把它当作严格可复现版本。建议停止修改后重新标记。</div>`;
  }
  return `<div><strong>预览尚未生成</strong>请查看构建记录。</div>`;
}

function setPreview(iframe, overlay, version) {
  const canPreview = version?.previewUrl && ["ready", "warning"].includes(version.status);
  iframe.hidden = !canPreview;
  overlay.hidden = Boolean(canPreview);
  if (canPreview) {
    const nextUrl = version.previewUrl;
    if (iframe.dataset.versionId !== version.id) {
      iframe.src = nextUrl;
      iframe.dataset.versionId = version.id;
    }
  } else {
    iframe.removeAttribute("src");
    iframe.dataset.versionId = "";
    overlay.innerHTML = buildStateMarkup(version);
  }
}

function chooseDefaultCompareId() {
  return state.versions.find((version) => version.id !== state.selectedId)?.id || null;
}

function focusAnchor(version, anchorId) {
  state.activeAnchorId = anchorId;
  state.pointing = false;
  updateDeepLink(version.id, anchorId);
  renderAnchorMarkers(version);
  renderAnchorList(version);
  const marker = elements.annotationLayer.querySelector(`[data-anchor-id="${CSS.escape(anchorId)}"]`);
  marker?.animate(
    [{ transform: "translate(-50%, -50%) scale(1)" }, { transform: "translate(-50%, -50%) scale(1.45)" }, { transform: "translate(-50%, -50%) scale(1.18)" }],
    { duration: 420, easing: "ease-out" },
  );
}

function renderAnchorMarkers(version) {
  const anchors = version?.anchors || [];
  elements.annotationLayer.classList.toggle("picking", state.pointing);
  elements.annotationLayer.innerHTML = anchors.map((anchor, index) => `
    <button
      class="anchor-marker ${anchor.id === state.activeAnchorId ? "active" : ""}"
      data-anchor-id="${escapeHtml(anchor.id)}"
      style="left:${anchor.x * 100}%;top:${anchor.y * 100}%"
      title="${escapeHtml(`${anchor.id} · ${anchor.title}`)}"
      type="button"
    >${index + 1}</button>
  `).join("");
  elements.annotationLayer.querySelectorAll("[data-anchor-id]").forEach((marker) => {
    marker.addEventListener("click", (event) => {
      event.stopPropagation();
      focusAnchor(version, marker.dataset.anchorId);
    });
  });
}

function renderAnchorList(version) {
  const anchors = version?.anchors || [];
  $("#anchor-count").textContent = anchors.length;
  if (!anchors.length) {
    elements.anchorList.innerHTML = `<div class="review-empty">还没有界面细节引用。<br />点击“指向界面细节”开始。</div>`;
    return;
  }
  elements.anchorList.innerHTML = anchors.map((anchor, index) => `
    <div class="anchor-card ${anchor.id === state.activeAnchorId ? "active" : ""}" data-focus-anchor="${escapeHtml(anchor.id)}">
      <span class="anchor-index">${index + 1}</span>
      <span class="anchor-copy"><strong>${escapeHtml(anchor.title)}</strong><small>${escapeHtml(anchor.id)}${anchor.viewLabel ? ` · ${escapeHtml(anchor.viewLabel)}` : ""}</small></span>
      <button class="copy-reference" data-copy-anchor="${escapeHtml(anchor.id)}" type="button" title="复制引用">⧉</button>
    </div>
  `).join("");
  elements.anchorList.querySelectorAll("[data-focus-anchor]").forEach((card) => {
    card.addEventListener("click", () => focusAnchor(version, card.dataset.focusAnchor));
  });
  elements.anchorList.querySelectorAll("[data-copy-anchor]").forEach((button) => {
    button.addEventListener("click", async (event) => {
      event.stopPropagation();
      const anchor = anchors.find((candidate) => candidate.id === button.dataset.copyAnchor);
      await copyText(anchorReference(version, anchor));
      toast(`已复制 ${anchor.id}，可直接发给员工06或其他员工`);
    });
  });
}

function renderPreview(version) {
  const hasVersion = Boolean(version);
  elements.emptyState.hidden = hasVersion;
  elements.previewLayout.hidden = !hasVersion;
  if (!hasVersion) {
    elements.pointDetail.disabled = true;
    return;
  }

  elements.primaryLabel.textContent = version.title;
  setPreview(elements.primaryPreview, elements.primaryBuildState, version);
  const canPoint = Boolean(version.previewUrl && ["ready", "warning"].includes(version.status));
  elements.pointDetail.disabled = !canPoint;
  if (!canPoint) state.pointing = false;
  elements.pointDetail.classList.toggle("active", state.pointing);
  elements.pointDetail.textContent = state.pointing ? "取消指向" : "◎ 指向界面细节";
  renderAnchorMarkers(version);

  const otherVersions = state.versions.filter((candidate) => candidate.id !== version.id);
  elements.toggleCompare.disabled = otherVersions.length === 0;
  if (!otherVersions.length) state.comparing = false;
  elements.previewLayout.classList.toggle("comparing", state.comparing);
  elements.compareColumn.hidden = !state.comparing;
  elements.toggleCompare.textContent = state.comparing ? "关闭对比" : "◫ 对比版本";

  elements.compareSelect.innerHTML = otherVersions.map((candidate) => `
    <option value="${escapeHtml(candidate.id)}" ${candidate.id === state.compareId ? "selected" : ""}>${escapeHtml(candidate.title)} · ${formatTime(candidate.createdAt)}</option>
  `).join("");
  if (state.comparing) {
    if (!state.compareId || state.compareId === version.id) state.compareId = chooseDefaultCompareId();
    elements.compareSelect.value = state.compareId || "";
    setPreview(elements.comparePreview, elements.compareBuildState, compareVersion());
  }
}

function renderReviews(version) {
  const reviews = version.reviews || [];
  $("#review-count").textContent = reviews.length;
  if (!reviews.length) {
    elements.reviewList.innerHTML = `<div class="review-empty">等待负责人留下第一次判断。<br />结论会永久绑定到当前版本。</div>`;
    return;
  }
  elements.reviewList.innerHTML = [...reviews].reverse().map((review) => `
    <article class="review-card">
      <header><strong>${escapeHtml(review.author)}</strong><time>${formatTime(review.createdAt)}</time></header>
      <span class="verdict ${escapeHtml(review.verdict)}">${verdictLabels[review.verdict] || review.verdict}</span>
      <p>${escapeHtml(review.note)}</p>
    </article>
  `).join("");
}

function renderInspector(version) {
  elements.inspectorEmpty.hidden = Boolean(version);
  elements.inspectorContent.hidden = !version;
  elements.showLog.disabled = !version;
  if (!version) return;
  const status = $("#detail-status");
  status.textContent = statusLabels[version.status] || version.status;
  status.className = `status-pill ${version.status}`;
  $("#detail-build-kind").textContent = version.buildMode === "full" ? "完整 Windows 构建" : "轻量交互预览";
  $("#detail-title").textContent = version.title;
  $("#detail-summary").textContent = version.summary;
  $("#detail-author").textContent = version.author;
  $("#detail-commit").textContent = `${version.git?.shortCommit || "unknown"} · ${version.git?.branch || "unknown"}`;
  $("#detail-time").textContent = new Date(version.createdAt).toLocaleString("zh-CN", { hour12: false });
  $("#detail-trigger").textContent = triggerLabels[version.trigger?.type] || "早期版本记录";
  $("#detail-dirty").textContent = version.git?.dirty ? `包含 ${version.git.dirtyFileCount} 项未提交变更` : "已提交快照";
  $("#detail-focus").textContent = version.focus || "未单独指定；请按本轮推进说明整体评审。";
  elements.launchVersion.hidden = !(version.nativeArtifacts?.length);
  renderAnchorList(version);
  renderReviews(version);
}

function render() {
  const version = selectedVersion();
  renderVersionList();
  renderSystem();
  renderPreview(version);
  renderInspector(version);
  $("#version-eyebrow").textContent = version ? `${version.author} · ${statusLabels[version.status] || version.status}` : "尚未选择版本";
  $("#version-title").textContent = version?.title || "建立第一个评审版本";
}

async function refresh({ preserveSelection = true } = {}) {
  const [versionsBody, system] = await Promise.all([api("/api/versions"), api("/api/system")]);
  state.versions = versionsBody.versions;
  state.system = system;
  if (!preserveSelection || !state.versions.some((version) => version.id === state.selectedId)) {
    state.selectedId = state.versions[0]?.id || null;
  }
  if (!state.versions.some((version) => version.id === state.compareId)) state.compareId = chooseDefaultCompareId();
  render();
}

function openCreateDialog() {
  elements.createMessage.textContent = "";
  elements.createDialog.showModal();
  setTimeout(() => elements.createForm.elements.title.focus(), 30);
}

elements.createForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  if (event.submitter?.value === "cancel") {
    elements.createDialog.close();
    return;
  }
  elements.createSubmit.disabled = true;
  elements.createMessage.textContent = "正在建立版本记录…";
  try {
    const data = Object.fromEntries(new FormData(elements.createForm));
    const body = await api("/api/versions", { method: "POST", body: JSON.stringify(data) });
    state.selectedId = body.version.id;
    elements.createDialog.close();
    elements.createForm.reset();
    elements.createForm.elements.author.value = "员工06";
    toast("版本已进入构建队列");
    await refresh();
  } catch (error) {
    elements.createMessage.textContent = error.message;
  } finally {
    elements.createSubmit.disabled = false;
  }
});

elements.pointDetail.addEventListener("click", () => {
  const version = selectedVersion();
  if (!version?.previewUrl) return;
  state.pointing = !state.pointing;
  renderPreview(version);
  if (state.pointing) toast("现在点击预览中你要讨论的具体位置");
});

elements.annotationLayer.addEventListener("click", (event) => {
  if (!state.pointing) return;
  const rect = elements.annotationLayer.getBoundingClientRect();
  const x = Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width));
  const y = Math.max(0, Math.min(1, (event.clientY - rect.top) / rect.height));
  const elementHint = inspectPreviewElement(x, y);
  state.pendingAnchor = { x, y, elementHint };
  state.pointing = false;
  elements.anchorMessage.textContent = "";
  elements.anchorForm.reset();
  elements.anchorForm.elements.author.value = "项目负责人";
  elements.elementContext.textContent = elementHint.label || elementHint.text
    ? `${elementHint.tag || "element"} · ${elementHint.label || elementHint.text}${elementHint.selector ? ` · ${elementHint.selector}` : ""}`
    : `位置：横向 ${(x * 100).toFixed(1)}% · 纵向 ${(y * 100).toFixed(1)}%`;
  renderPreview(selectedVersion());
  elements.anchorDialog.showModal();
  setTimeout(() => elements.anchorForm.elements.title.focus(), 30);
});

elements.anchorForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  if (event.submitter?.value === "cancel") {
    elements.anchorDialog.close();
    state.pendingAnchor = null;
    return;
  }
  const version = selectedVersion();
  if (!version || !state.pendingAnchor) return;
  elements.anchorSubmit.disabled = true;
  elements.anchorMessage.textContent = "正在生成稳定引用…";
  try {
    const data = Object.fromEntries(new FormData(elements.anchorForm));
    const body = await api(`/api/versions/${encodeURIComponent(version.id)}/anchors`, {
      method: "POST",
      body: JSON.stringify({ ...data, ...state.pendingAnchor }),
    });
    state.activeAnchorId = body.anchor.id;
    state.pendingAnchor = null;
    elements.anchorDialog.close();
    await refresh();
    updateDeepLink(version.id, body.anchor.id);
    await copyText(anchorReference(selectedVersion(), body.anchor));
    toast(`已生成并复制视觉引用 ${body.anchor.id}`);
  } catch (error) {
    elements.anchorMessage.textContent = error.message;
  } finally {
    elements.anchorSubmit.disabled = false;
  }
});

elements.reviewForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  const version = selectedVersion();
  if (!version) return;
  const submit = elements.reviewForm.querySelector("button[type=submit]");
  submit.disabled = true;
  try {
    const data = Object.fromEntries(new FormData(elements.reviewForm));
    await api(`/api/versions/${encodeURIComponent(version.id)}/reviews`, { method: "POST", body: JSON.stringify(data) });
    elements.reviewForm.elements.note.value = "";
    toast("评审结论已绑定到此版本");
    await refresh();
  } catch (error) {
    toast(error.message);
  } finally {
    submit.disabled = false;
  }
});

elements.toggleCompare.addEventListener("click", () => {
  state.comparing = !state.comparing;
  if (state.comparing && !state.compareId) state.compareId = chooseDefaultCompareId();
  renderPreview(selectedVersion());
});
elements.compareSelect.addEventListener("change", () => {
  state.compareId = elements.compareSelect.value;
  setPreview(elements.comparePreview, elements.compareBuildState, compareVersion());
});
$("#reload-primary").addEventListener("click", () => {
  const version = selectedVersion();
  if (version?.previewUrl) elements.primaryPreview.src = `${version.previewUrl}?reload=${Date.now()}`;
});
$("#reload-compare").addEventListener("click", () => {
  const version = compareVersion();
  if (version?.previewUrl) elements.comparePreview.src = `${version.previewUrl}?reload=${Date.now()}`;
});
elements.launchVersion.addEventListener("click", async () => {
  const version = selectedVersion();
  if (!version) return;
  try {
    await api(`/api/versions/${encodeURIComponent(version.id)}/launch`, { method: "POST", body: "{}" });
    toast("完整版本已启动");
  } catch (error) { toast(error.message); }
});
elements.showLog.addEventListener("click", async () => {
  const version = selectedVersion();
  if (!version) return;
  elements.buildLog.textContent = "正在读取…";
  elements.logDialog.showModal();
  try {
    const body = await api(`/api/versions/${encodeURIComponent(version.id)}/log`);
    elements.buildLog.textContent = body.log;
    elements.buildLog.scrollTop = elements.buildLog.scrollHeight;
  } catch (error) { elements.buildLog.textContent = error.message; }
});
$("#close-log").addEventListener("click", () => elements.logDialog.close());
$("#open-create").addEventListener("click", openCreateDialog);
$("#empty-create").addEventListener("click", openCreateDialog);

const events = new EventSource("/api/events");
events.addEventListener("version", () => void refresh());
events.onerror = () => setTimeout(() => void refresh(), 3000);

refresh({ preserveSelection: Boolean(state.selectedId) }).catch((error) => toast(error.message));
