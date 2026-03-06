import { useState } from "react";
import { useFileSystem } from "../hooks/useFileSystem";
import SidebarCanvas from "./SidebarCanvas";

function ChevronRight({ size = 10 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 10 10" fill="none">
      <path d="M3 1.5 L7 5 L3 8.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export default function Sidebar() {
  const { directoryHandle, openDirectory } = useFileSystem();
  const [collapsed, setCollapsed] = useState(false);
  const [hovered, setHovered] = useState(false);

  if (collapsed) {
    return (
      <div
        style={{
          width: 40,
          background: "#141414",
          borderRight: "1px solid #232323",
          display: "flex",
          alignItems: "flex-start",
          justifyContent: "center",
          paddingTop: 8,
        }}
      >
        <button
          onClick={() => setCollapsed(false)}
          style={{
            background: "none",
            border: "none",
            color: "#cdd6f4",
            cursor: "pointer",
            fontSize: 18,
          }}
          title="Expand sidebar"
        >
          <ChevronRight size={14} />
        </button>
      </div>
    );
  }

  return (
    <div
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      style={{
        width: 240,
        background: "#141414",
        borderRight: "1px solid #232323",
        display: "flex",
        flexDirection: "column",
        overflow: "hidden",
      }}
    >
      <div
        style={{
          padding: "8px 12px",
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          borderBottom: "1px solid #2a2a2a",
          fontSize: 11,
          textTransform: "uppercase",
          letterSpacing: 1,
          color: "#a6adc8",
        }}
      >
        <span>Explorer</span>
        <button
          onClick={() => setCollapsed(true)}
          style={{
            background: "none",
            border: "none",
            color: "#a6adc8",
            cursor: "pointer",
            fontSize: 14,
            display: "flex",
            alignItems: "center",
          }}
        >
          <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
            <path d="M7 1.5 L3 5 L7 8.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        </button>
      </div>

      {!directoryHandle ? (
        <div style={{ padding: 16 }}>
          <button
            onClick={openDirectory}
            style={{
              width: "100%",
              padding: "8px 12px",
              background: "#89b4fa",
              color: "#1e1e2e",
              border: "none",
              borderRadius: 4,
              cursor: "pointer",
              fontWeight: 600,
              fontSize: 13,
            }}
          >
            Open Folder
          </button>
          <p style={{ marginTop: 8, fontSize: 11, color: "#6c7086" }}>
            Files stay on your machine. Nothing is uploaded.
          </p>
        </div>
      ) : (
        <div style={{ flex: 1, overflow: "hidden" }}>
          <SidebarCanvas showGuides={hovered} />
        </div>
      )}
    </div>
  );
}
