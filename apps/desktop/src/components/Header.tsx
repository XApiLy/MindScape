import { useState } from "react";
import {
  Boxes,
  ChevronDown,
  Grid3X3,
  Minus,
  Network,
  Plus,
  Search,
  Settings2,
  SlidersHorizontal,
} from "lucide-react";
import { useReactFlow } from "@xyflow/react";
import { useWorkspaceStore } from "../store/workspaceStore";

type HeaderProps = {
  onProviders: () => void;
};

export function Header({ onProviders }: HeaderProps) {
  const [zoom, setZoom] = useState(88);
  const { zoomIn, zoomOut, fitView } = useReactFlow();
  const canvasMode = useWorkspaceStore((state) => state.canvasMode);
  const setCanvasMode = useWorkspaceStore((state) => state.setCanvasMode);

  const changeZoom = async (direction: "in" | "out") => {
    if (direction === "in") {
      await zoomIn({ duration: 160 });
      setZoom((value) => Math.min(200, value + 10));
    } else {
      await zoomOut({ duration: 160 });
      setZoom((value) => Math.max(20, value - 10));
    }
  };

  return (
    <header className="topbar">
      <div className="topbar-title">
        <button className="icon-button" title="切换侧栏" aria-label="切换侧栏">
          <Boxes size={17} />
        </button>
        <span className="topbar-mark" />
        <strong>MindScape</strong>
        <span className="divider" />
        <span className="workspace-title">AI 会话与画布架构</span>
        <ChevronDown size={14} />
      </div>

      <div className="view-switch" aria-label="画布视图">
        <button
          className={canvasMode === "immersive" ? "active" : ""}
          type="button"
          onClick={() => setCanvasMode("immersive")}
        >
          <Network size={15} />
          沉浸式探索
        </button>
        <button
          className={canvasMode === "grid" ? "active" : ""}
          type="button"
          onClick={() => setCanvasMode("grid")}
        >
          <Grid3X3 size={15} />
          宏观网格
        </button>
      </div>

      <div className="topbar-tools">
        <div className="zoom-control">
          <button title="缩小" onClick={() => changeZoom("out")} aria-label="缩小">
            <Minus size={14} />
          </button>
          <button className="zoom-value" onClick={() => void fitView({ padding: 0.16, duration: 240 })}>
            {zoom}%
          </button>
          <button title="放大" onClick={() => changeZoom("in")} aria-label="放大">
            <Plus size={14} />
          </button>
        </div>
        <button className="search-button" type="button">
          <Search size={15} />
          <span>搜索</span>
          <kbd>⌘K</kbd>
        </button>
        <button className="icon-button" title="显示设置" aria-label="显示设置">
          <SlidersHorizontal size={17} />
        </button>
        <button className="provider-button" type="button" onClick={onProviders}>
          <Settings2 size={16} />
          模型接口
        </button>
      </div>
    </header>
  );
}
