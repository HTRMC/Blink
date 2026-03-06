import type { OpenFile } from "../hooks/useFileSystem";

interface Props {
  tabs: OpenFile[];
  activeFile: OpenFile | null;
  onSelect: (path: string) => void;
  onClose: (path: string) => void;
  onPin: (path: string) => void;
}

export default function TabBar({ tabs, activeFile, onSelect, onClose, onPin }: Props) {
  if (tabs.length === 0) return null;

  return (
    <div
      style={{
        display: "flex",
        background: "#141414",
        borderBottom: "1px solid #313244",
        height: 36,
        overflow: "auto",
      }}
    >
      {tabs.map((tab) => (
        <div
          key={tab.path}
          onClick={() => onSelect(tab.path)}
          onDoubleClick={() => onPin(tab.path)}
          style={{
            padding: "0 16px",
            display: "flex",
            alignItems: "center",
            gap: 8,
            fontSize: 13,
            cursor: "pointer",
            background:
              activeFile?.path === tab.path ? "#181818" : "transparent",
            borderRight: "1px solid #313244",
            color:
              activeFile?.path === tab.path ? "#cdd6f4" : "#6c7086",
            fontStyle: tab.preview ? "italic" : "normal",
          }}
        >
          <span>{tab.name}</span>
          <span
            onClick={(e) => {
              e.stopPropagation();
              onClose(tab.path);
            }}
            style={{
              fontSize: 16,
              lineHeight: 1,
              opacity: 0.5,
            }}
            onMouseEnter={(e) => (e.currentTarget.style.opacity = "1")}
            onMouseLeave={(e) => (e.currentTarget.style.opacity = "0.5")}
          >
            x
          </span>
        </div>
      ))}
    </div>
  );
}
