import { useRef, useEffect, useState } from "react";
import type { OpenFile } from "../hooks/useFileSystem";

interface Props {
  activeFile: OpenFile | null;
}

export default function EditorCanvas({ activeFile }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const editorRef = useRef<any>(null);
  const [status, setStatus] = useState<string>("Initializing...");

  useEffect(() => {
    let cancelled = false;

    async function initEditor() {
      try {
        // Check WebGPU support
        if (!navigator.gpu) {
          setStatus("WebGPU not supported in this browser. Please use Chrome 113+ or Edge 113+.");
          return;
        }

        setStatus("Loading Blink core...");
        const blink = await import("../../wasm/blink_core");

        if (cancelled) return;

        const editor = new blink.Editor();
        await editor.init_renderer("editor-canvas");
        editorRef.current = editor;

        // Set sample content
        editor.set_content(
          `// Welcome to Blink\n// A GPU-accelerated code editor\n\nfn main() {\n    println!("Hello, Blink!");\n}\n`
        );
        editor.render();
        setStatus("Ready");
      } catch (err) {
        console.error("Failed to init editor:", err);
        setStatus(`Error: ${err}`);
      }
    }

    initEditor();

    return () => {
      cancelled = true;
    };
  }, []);

  // Re-render when active file changes
  useEffect(() => {
    const editor = editorRef.current;
    if (editor && activeFile) {
      editor.set_content(activeFile.content);
      editor.render();
    }
  }, [activeFile]);

  // Handle resize
  useEffect(() => {
    const canvas = canvasRef.current;
    const editor = editorRef.current;
    if (!canvas || !editor) return;

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const { width, height } = entry.contentRect;
        canvas.width = width * devicePixelRatio;
        canvas.height = height * devicePixelRatio;
        editor.resize(canvas.width, canvas.height);
        editor.render();
      }
    });

    observer.observe(canvas);
    return () => observer.disconnect();
  }, [status]);

  return (
    <div style={{ flex: 1, position: "relative", overflow: "hidden" }}>
      <canvas
        id="editor-canvas"
        ref={canvasRef}
        style={{ width: "100%", height: "100%", display: "block" }}
      />
      {status !== "Ready" && (
        <div
          style={{
            position: "absolute",
            top: "50%",
            left: "50%",
            transform: "translate(-50%, -50%)",
            color: "#a6adc8",
            fontSize: "14px",
          }}
        >
          {status}
        </div>
      )}
    </div>
  );
}
