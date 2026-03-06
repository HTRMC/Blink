import { useState } from "react";
import { useFileSystem } from "../hooks/useFileSystem";

interface Props {
  onFileSelect: (name: string, content: string) => void;
}

export default function Sidebar({ onFileSelect }: Props) {
  const { directoryHandle, openDirectory, entries } = useFileSystem();
  const [collapsed, setCollapsed] = useState(false);

  if (collapsed) {
    return (
      <div
        style={{
          width: 40,
          background: "#141414",
          borderRight: "1px solid #313244",
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
          {"\u25B6"}
        </button>
      </div>
    );
  }

  return (
    <div
      style={{
        width: 240,
        background: "#141414",
        borderRight: "1px solid #313244",
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
          borderBottom: "1px solid #313244",
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
          }}
        >
          {"\u25C0"}
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
        <div style={{ flex: 1, overflow: "auto", padding: "4px 0" }}>
          {entries.map((entry) => (
            <div
              key={entry.name}
              onClick={() => {
                if (entry.kind === "file") {
                  onFileSelect(entry.name, "");
                }
              }}
              style={{
                padding: "4px 16px",
                fontSize: 13,
                cursor: entry.kind === "file" ? "pointer" : "default",
                color: entry.kind === "file" ? "#cdd6f4" : "#89b4fa",
                userSelect: "none",
              }}
              onMouseEnter={(e) =>
                (e.currentTarget.style.background = "#313244")
              }
              onMouseLeave={(e) =>
                (e.currentTarget.style.background = "transparent")
              }
            >
              {entry.kind === "directory" ? "\u{1F4C1} " : "\u{1F4C4} "}
              {entry.name}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
