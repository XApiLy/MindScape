import { startTransition, useMemo, useState } from "react";
import { groupCopy, references, type Reference } from "./references";

type GroupId = Reference["group"];
type DisplayMode = "focus" | "compare";

const GROUPS: GroupId[] = ["surface", "weather", "cloud"];

function ArrowIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 16 16">
      <path d="M5 11 11 5M6 5h5v5" />
    </svg>
  );
}

function ReloadIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 16 16">
      <path d="M13 7a5 5 0 1 0-.7 3.6M13 3v4H9" />
    </svg>
  );
}

function ReferenceFrame({ reference, url, compact = false }: { reference: Reference; url: string; compact?: boolean }) {
  const [reloadKey, setReloadKey] = useState(0);

  return (
    <article className={compact ? "reference-frame compact" : "reference-frame"}>
      <div className="frame-bar">
        <div className="frame-identity">
          <span className={`runtime-dot ${reference.runtime}`} aria-hidden="true" />
          <span className="frame-owner">{reference.owner}</span>
          <strong>{reference.name}</strong>
        </div>
        <div className="frame-actions">
          <button type="button" onClick={() => setReloadKey((key) => key + 1)} aria-label={`重新载入 ${reference.name}`}>
            <ReloadIcon />
          </button>
          <a href={url} target="_blank" rel="noreferrer" aria-label={`单独打开 ${reference.name}`}>
            <ArrowIcon />
          </a>
        </div>
      </div>
      <div className="frame-viewport">
        <iframe key={`${url}-${reloadKey}`} src={url} title={`${reference.owner} ${reference.name} 原始效果`} />
      </div>
      <div className="frame-caption">
        <span>{reference.role}</span>
        <code>{reference.commit}</code>
      </div>
    </article>
  );
}

export default function App() {
  const [group, setGroup] = useState<GroupId>("surface");
  const [selectedId, setSelectedId] = useState("glace");
  const [mode, setMode] = useState<DisplayMode>("compare");
  const [viewIndex, setViewIndex] = useState<Record<string, number>>({});

  const groupReferences = useMemo(() => references.filter((reference) => reference.group === group), [group]);
  const selected = groupReferences.find((reference) => reference.id === selectedId) ?? groupReferences[0];
  const selectedView = selected.views[viewIndex[selected.id] ?? 0] ?? selected.views[0];
  const copy = groupCopy[group];

  function selectGroup(nextGroup: GroupId) {
    const firstReference = references.find((reference) => reference.group === nextGroup);
    startTransition(() => {
      setGroup(nextGroup);
      setSelectedId(firstReference?.id ?? "");
      setMode(nextGroup === "surface" ? "compare" : "focus");
    });
  }

  function selectReference(reference: Reference) {
    startTransition(() => {
      setSelectedId(reference.id);
      setMode("focus");
    });
  }

  return (
    <div className="lab-shell">
      <header className="lab-header">
        <div className="brand-lockup">
          <span className="brand-mark" aria-hidden="true">M</span>
          <div>
            <p>MIND­SCAPE / SOURCE REFERENCE</p>
            <h1>材质原仓对照台</h1>
          </div>
        </div>
        <div className="header-rule">
          <span>NO FUSION</span>
          <p>每个画面来自独立实现，不共享着色器和材质参数。</p>
        </div>
      </header>

      <nav className="group-tabs" aria-label="对照类型">
        {GROUPS.map((groupId) => (
          <button
            type="button"
            key={groupId}
            className={groupId === group ? "active" : ""}
            onClick={() => selectGroup(groupId)}
          >
            <small>{groupCopy[groupId].eyebrow}</small>
            <span>{groupCopy[groupId].title}</span>
          </button>
        ))}
      </nav>

      <main className="lab-main">
        <aside className="reference-index">
          <div className="section-intro">
            <p>{copy.eyebrow}</p>
            <h2>{copy.title}</h2>
            <span>{copy.summary}</span>
          </div>

          <div className="reference-list">
            {groupReferences.map((reference) => (
              <button
                type="button"
                className={reference.id === selected.id ? "reference-item active" : "reference-item"}
                key={reference.id}
                onClick={() => selectReference(reference)}
              >
                <span className="reference-number">{String(groupReferences.indexOf(reference) + 1).padStart(2, "0")}</span>
                <span>
                  <strong>{reference.name}</strong>
                  <small>{reference.owner}</small>
                </span>
                <span className={`runtime-tag ${reference.runtime}`}>{reference.runtime === "local" ? "LOCAL" : "WEB"}</span>
              </button>
            ))}
          </div>

          <div className="source-facts">
            <p>当前观察对象</p>
            <dl>
              <div><dt>作用</dt><dd>{selected.role}</dd></div>
              <div><dt>技术</dt><dd>{selected.technique}</dd></div>
              <div><dt>边界</dt><dd>{selected.boundary}</dd></div>
              <div><dt>许可</dt><dd>{selected.license}</dd></div>
            </dl>
            <a href={selected.repository} target="_blank" rel="noreferrer">
              查看原仓库 <ArrowIcon />
            </a>
          </div>
        </aside>

        <section className="stage">
          <div className="stage-toolbar">
            <div className="mode-switch" aria-label="显示模式">
              <button type="button" className={mode === "focus" ? "active" : ""} onClick={() => setMode("focus")}>单项观察</button>
              <button type="button" className={mode === "compare" ? "active" : ""} onClick={() => setMode("compare")}>同屏对照</button>
            </div>
            {mode === "focus" && selected.views.length > 1 ? (
              <div className="view-switch" aria-label="原仓页面">
                {selected.views.map((view, index) => (
                  <button
                    type="button"
                    className={(viewIndex[selected.id] ?? 0) === index ? "active" : ""}
                    key={view.url}
                    onClick={() => setViewIndex((current) => ({ ...current, [selected.id]: index }))}
                  >
                    {view.label}
                  </button>
                ))}
              </div>
            ) : (
              <p className="stage-note">同屏模式默认打开每个仓库的首个对照页面</p>
            )}
          </div>

          {mode === "focus" ? (
            <ReferenceFrame reference={selected} url={selectedView.url} />
          ) : (
            <div className={`comparison-grid count-${groupReferences.length}`}>
              {groupReferences.map((reference) => (
                <ReferenceFrame key={reference.id} reference={reference} url={reference.views[0].url} compact />
              ))}
            </div>
          )}
        </section>
      </main>

      <footer className="lab-footer">
        <span>判断顺序</span>
        <ol>
          <li>先看原始观感</li>
          <li>再看交互反馈</li>
          <li>最后看性能边界</li>
        </ol>
        <p>此工具不代表最终 MindScape UI。</p>
      </footer>
    </div>
  );
}
