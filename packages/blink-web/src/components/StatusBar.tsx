import type { OpenFile } from "../hooks/useFileSystem";

interface Props {
  activeFile: OpenFile | null;
}

export default function StatusBar({ activeFile }: Props) {
  return (
    <div
      style={{
        height: 24,
        background: "#181825",
        borderTop: "1px solid #313244",
        display: "flex",
        alignItems: "center",
        padding: "0 12px",
        fontSize: 12,
        color: "#6c7086",
        justifyContent: "space-between",
      }}
    >
      <span>{activeFile ? activeFile.name : "No file open"}</span>
      <span>Blink v0.1.0</span>
    </div>
  );
}
