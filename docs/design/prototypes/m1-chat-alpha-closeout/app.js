const els = {
  form: document.querySelector("#composer-form"),
  prompt: document.querySelector("#prompt"),
  send: document.querySelector("#send-button"),
  composerState: document.querySelector("#composer-state"),
  empty: document.querySelector("#empty-state"),
  node: document.querySelector("#node-card"),
  nodeState: document.querySelector("#node-state"),
  question: document.querySelector("#node-question"),
  answer: document.querySelector("#node-answer"),
  meta: document.querySelector("#node-meta"),
  partial: document.querySelector("#partial-note"),
  retry: document.querySelector("#retry-button"),
  focus: document.querySelector("#focus-button"),
  notice: document.querySelector("#status-notice"),
  noticeTitle: document.querySelector("#notice-title"),
  noticeCopy: document.querySelector("#notice-copy"),
  noticeAction: document.querySelector("#notice-action"),
  connectionLabel: document.querySelector("#connection-label"),
  connectionDot: document.querySelector(".connection-dot"),
  readerBackdrop: document.querySelector("#reader-backdrop"),
  reader: document.querySelector(".reader"),
  readerTitle: document.querySelector("#reader-title"),
  readerAnswer: document.querySelector("#reader-answer"),
  readerState: document.querySelector("#reader-state"),
};

const sampleQuestion = "如何把一个模糊的产品想法，组织成可验证的研究计划？";
const fullAnswer = [
  "先不要直接列功能，而是把想法拆成三层：要改变谁的什么行为、为什么现在没有发生、什么证据会让我们改变判断。",
  "\n\n接着只保留一个最高风险假设，为它设计最小证据：5 次访谈、1 个可点击原型，或一次小范围真实任务。",
  "\n\n最后在开始前写下通过、失败和中止标准。这会让研究从“寻找认同”变成“降低不确定性”。",
].join("");

let phase = "blank";
let timer = null;
let cursor = 0;
let previousFocus = null;

function clearTimer() {
  if (timer) window.clearInterval(timer);
  timer = null;
}

function setNotice(kind, title, copy, action = "") {
  els.notice.hidden = !title;
  els.notice.className = `status-notice${kind === "error" ? " is-error" : ""}`;
  els.noticeTitle.textContent = title;
  els.noticeCopy.textContent = copy;
  els.noticeAction.textContent = action;
  els.noticeAction.hidden = !action;
}

function setConnection(connected, label = "DeepSeek 已连接") {
  els.connectionLabel.textContent = label;
  els.connectionDot.classList.toggle("is-offline", !connected);
}

function setNodeState(label, tone = "") {
  els.nodeState.textContent = label;
  els.nodeState.className = `state-chip${tone ? ` is-${tone}` : ""}`;
}

function showNode(question) {
  els.empty.hidden = true;
  els.node.hidden = false;
  els.question.textContent = question;
}

function reset() {
  clearTimer();
  phase = "blank";
  cursor = 0;
  els.empty.hidden = false;
  els.node.hidden = true;
  els.answer.textContent = "";
  els.answer.className = "answer";
  els.partial.hidden = true;
  els.retry.hidden = true;
  els.focus.disabled = true;
  els.prompt.disabled = false;
  els.prompt.value = "";
  els.send.disabled = false;
  els.send.className = "send-button";
  els.send.setAttribute("aria-label", "发送");
  els.composerState.textContent = "准备就绪";
  els.meta.textContent = "deepseek-chat";
  setConnection(true);
  setNotice("", "", "");
  closeReader(false);
  els.prompt.focus();
}

function beginRun(question) {
  clearTimer();
  phase = "sending";
  cursor = 0;
  showNode(question);
  els.answer.textContent = "";
  els.answer.className = "answer is-streaming";
  els.partial.hidden = true;
  els.retry.hidden = true;
  els.focus.disabled = true;
  els.prompt.disabled = true;
  els.send.className = "send-button is-stop";
  els.send.setAttribute("aria-label", "停止生成");
  els.composerState.textContent = "正在发送…";
  setNodeState("准备生成");
  setNotice("", "", "");

  window.setTimeout(() => {
    if (phase !== "sending") return;
    phase = "streaming";
    setNodeState("正在生成");
    els.node.classList.add("is-streaming");
    els.composerState.textContent = "正在生成 · 可随时停止";
    timer = window.setInterval(streamTick, 22);
  }, 520);
}

function streamTick() {
  cursor += 1;
  els.answer.textContent = fullAnswer.slice(0, cursor);
  if (cursor >= fullAnswer.length) completeRun();
}

function completeRun() {
  clearTimer();
  phase = "completed";
  els.answer.textContent = fullAnswer;
  els.answer.className = "answer";
  els.node.classList.remove("is-streaming");
  setNodeState("已完成", "success");
  els.meta.textContent = "deepseek-chat · 842 tokens";
  els.focus.disabled = false;
  els.prompt.disabled = false;
  els.send.className = "send-button";
  els.send.setAttribute("aria-label", "发送");
  els.composerState.textContent = "已完成 · 可继续提问";
  els.prompt.value = "";
}

function stopRun() {
  if (phase !== "sending" && phase !== "streaming") return;
  clearTimer();
  phase = "stopping";
  els.send.className = "send-button is-stop is-stopping";
  els.send.disabled = true;
  els.composerState.textContent = "正在停止…";
  setNodeState("正在停止");

  window.setTimeout(() => {
    if (phase !== "stopping") return;
    phase = "cancelled";
    els.node.classList.remove("is-streaming");
    els.answer.className = "answer";
    if (!els.answer.textContent) els.answer.textContent = "已发出请求，但还没有收到正文。";
    els.partial.hidden = false;
    els.retry.hidden = false;
    els.focus.disabled = false;
    setNodeState("已停止", "danger");
    els.meta.textContent = "deepseek-chat · 部分内容";
    els.prompt.disabled = false;
    els.send.disabled = false;
    els.send.className = "send-button";
    els.send.setAttribute("aria-label", "发送");
    els.composerState.textContent = "已停止 · 部分内容已保留";
  }, 620);
}

function showNoKey() {
  reset();
  phase = "no-key";
  setConnection(false, "DeepSeek 未配置");
  setNotice("error", "还没有可用的 DeepSeek Key", "先在系统凭据入口完成配置，Key 不会显示在对话中。", "打开设置");
  els.composerState.textContent = "需要配置 Key";
  els.send.disabled = true;
}

function showFailed() {
  reset();
  phase = "failed";
  showNode(sampleQuestion);
  els.answer.textContent = "连接在回答完成前中断，本次没有生成可保存的完整回答。";
  setNodeState("连接中断", "danger");
  els.meta.textContent = "deepseek-chat · 未自动重试";
  els.retry.hidden = false;
  els.focus.disabled = true;
  setNotice("error", "本次生成未完成", "请检查网络后手动重试；系统不会自动发起可能计费的请求。", "重试");
  els.composerState.textContent = "失败 · 等待你决定";
}

function showRecovered() {
  reset();
  phase = "recovered";
  showNode(sampleQuestion);
  els.answer.textContent = fullAnswer.slice(0, 92);
  els.partial.hidden = false;
  els.retry.hidden = false;
  els.focus.disabled = false;
  setNodeState("已恢复", "success");
  els.meta.textContent = "deepseek-chat · 恢复的部分内容";
  setNotice("", "已恢复上次未完成的运行", "异常退出前的部分文本已保留，运行已收口，不会继续后台生成。", "知道了");
  els.composerState.textContent = "恢复完成 · 可重试";
}

function openReader() {
  if (els.focus.disabled) return;
  previousFocus = document.activeElement;
  els.readerTitle.textContent = els.question.textContent;
  els.readerAnswer.textContent = els.answer.textContent;
  els.readerState.textContent = els.nodeState.textContent;
  els.readerBackdrop.hidden = false;
  els.reader.focus();
}

function closeReader(restoreFocus = true) {
  els.readerBackdrop.hidden = true;
  if (restoreFocus && previousFocus instanceof HTMLElement) previousFocus.focus();
}

els.form.addEventListener("submit", (event) => {
  event.preventDefault();
  if (phase === "sending" || phase === "streaming") {
    stopRun();
    return;
  }
  const question = els.prompt.value.trim();
  if (!question) {
    els.prompt.focus();
    return;
  }
  beginRun(question);
});

els.prompt.addEventListener("keydown", (event) => {
  if (event.key === "Enter" && !event.shiftKey) {
    event.preventDefault();
    els.form.requestSubmit();
  }
});

document.querySelector("#sample-prompt").addEventListener("click", () => {
  els.prompt.value = sampleQuestion;
  els.prompt.focus();
});

document.querySelectorAll("[data-scenario]").forEach((button) => {
  button.addEventListener("click", () => {
    const scenario = button.dataset.scenario;
    if (scenario === "no-key") showNoKey();
    else if (scenario === "failed") showFailed();
    else if (scenario === "recovered") showRecovered();
    else reset();
  });
});

els.retry.addEventListener("click", () => beginRun(els.question.textContent || sampleQuestion));
els.noticeAction.addEventListener("click", () => {
  if (phase === "failed") beginRun(els.question.textContent || sampleQuestion);
  else if (phase === "recovered") setNotice("", "", "");
});
els.focus.addEventListener("click", openReader);
document.querySelector("#close-reader").addEventListener("click", () => closeReader());
document.querySelector("#reader-done").addEventListener("click", () => closeReader());
els.readerBackdrop.addEventListener("click", (event) => {
  if (event.target === els.readerBackdrop) closeReader();
});
document.addEventListener("keydown", (event) => {
  if (event.key === "Escape" && !els.readerBackdrop.hidden) closeReader();
});

const initialScenario = new URLSearchParams(window.location.search).get("scenario");
if (initialScenario === "no-key") showNoKey();
else if (initialScenario === "failed") showFailed();
else if (initialScenario === "recovered") showRecovered();
else if (initialScenario === "streaming") {
  reset();
  beginRun(sampleQuestion);
}
else if (initialScenario === "stopped") {
  reset();
  beginRun(sampleQuestion);
  window.setTimeout(stopRun, 1300);
}
else if (initialScenario === "completed" || initialScenario === "reader") {
  reset();
  showNode(sampleQuestion);
  completeRun();
  if (initialScenario === "reader") openReader();
}
else reset();
