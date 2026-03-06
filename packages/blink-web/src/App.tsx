import { useState, useCallback, useRef } from "react";
import EditorCanvas from "./components/EditorCanvas";
import Sidebar from "./components/Sidebar";
import TabBar from "./components/TabBar";
import StatusBar from "./components/StatusBar";
import { FileSystemProvider, useFileSystem } from "./hooks/useFileSystem";

function ResizeHandle({
  onDrag,
}: {
  onDrag: (deltaX: number) => void;
}) {
  const [hovered, setHovered] = useState(false);
  const dragging = useRef(false);

  const onMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      dragging.current = true;
      let lastX = e.clientX;

      const onMove = (ev: MouseEvent) => {
        const dx = ev.clientX - lastX;
        lastX = ev.clientX;
        onDrag(dx);
      };

      const onUp = () => {
        dragging.current = false;
        window.removeEventListener("mousemove", onMove);
        window.removeEventListener("mouseup", onUp);
      };

      window.addEventListener("mousemove", onMove);
      window.addEventListener("mouseup", onUp);
    },
    [onDrag]
  );

  return (
    <div
      onMouseDown={onMouseDown}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => {
        if (!dragging.current) setHovered(false);
      }}
      style={{
        width: 3,
        cursor: "ew-resize",
        background: hovered ? "rgba(253, 253, 253, 0.133)" : "transparent",
        transition: "background 0.15s ease",
        flexShrink: 0,
      }}
    />
  );
}

function AppInner() {
  const { openFiles, activeFile, closeFile, setActiveFile } = useFileSystem();
  const [sidebarWidth, setSidebarWidth] = useState(240);

  const handleDrag = useCallback((dx: number) => {
    setSidebarWidth((w) => Math.max(213, w + dx));
  }, []);

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100vh" }}>
      <TabBar
        tabs={openFiles}
        activeFile={activeFile}
        onSelect={setActiveFile}
        onClose={closeFile}
      />
      <div style={{ display: "flex", flex: 1, overflow: "hidden" }}>
        <Sidebar width={sidebarWidth} />
        <ResizeHandle onDrag={handleDrag} />
        <EditorCanvas activeFile={activeFile} />
      </div>
      <StatusBar activeFile={activeFile} />
    </div>
  );
}

export default function App() {
  return (
    <FileSystemProvider>
      <AppInner />
    </FileSystemProvider>
  );
}
