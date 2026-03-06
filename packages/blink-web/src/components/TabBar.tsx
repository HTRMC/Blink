import type { OpenFile } from "../hooks/useFileSystem";

interface Props {
  tabs: OpenFile[];
  activeFile: OpenFile | null;
  onSelect: (name: string) => void;
  onClose: (name: string) => void;
}

export default function TabBar({ tabs, activeFile, onSelect, onClose }: Props) {
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
          key={tab.name}
          onClick={() => onSelect(tab.name)}
          style={{
            padding: "0 16px",
            display: "flex",
            alignItems: "center",
            gap: 8,
            fontSize: 13,
            cursor: "pointer",
            background:
              activeFile?.name === tab.name ? "#181818" : "transparent",
            borderRight: "1px solid #313244",
            color:
              activeFile?.name === tab.name ? "#cdd6f4" : "#6c7086",
          }}
        >
          <span>{tab.name}</span>
          <span
            onClick={(e) => {
              e.stopPropagation();
              onClose(tab.name);
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
