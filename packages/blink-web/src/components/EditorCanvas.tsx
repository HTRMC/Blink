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
        if (!navigator.gpu) {
          setStatus(
            "WebGPU not supported in this browser. Please use Chrome 113+ or Edge 113+."
          );
          return;
        }

        setStatus("Loading Blink core...");
        const blink = await import("../../wasm/blink_core");
        await blink.default();

        if (cancelled) return;

        setStatus("Loading font...");
        const fontResp = await fetch("/fonts/JetBrainsMono-Regular.ttf");
        if (!fontResp.ok) {
          setStatus(
            "Font not found. Place a monospace .ttf at public/fonts/JetBrainsMono-Regular.ttf"
          );
          return;
        }
        const fontData = new Uint8Array(await fontResp.arrayBuffer());

        if (cancelled) return;

        const editor = new blink.Editor();
        await editor.init_renderer("editor-canvas", fontData);
        editorRef.current = editor;

        editor.set_content(
          [
            "// Welcome to Blink",
            "// A GPU-accelerated code editor",
            "",
            "fn main() {",
            '    println!("Hello, Blink!");',
            "",
            '    let message = "GPU-rendered text!";',
            "    for i in 0..10 {",
            '        println!("{}: {}", i, message);',
            "    }",
            "}",
            "",
          ].join("\n")
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

  useEffect(() => {
    const editor = editorRef.current;
    if (editor && activeFile) {
      editor.set_content(activeFile.content);
      editor.render();
    }
  }, [activeFile]);

  // Keyboard input
  useEffect(() => {
    const canvas = canvasRef.current;
    const editor = editorRef.current;
    if (!canvas || !editor) return;

    // Make canvas focusable
    canvas.tabIndex = 0;
    canvas.style.outline = "none";
    canvas.focus();

    const onKeyDown = (e: KeyboardEvent) => {
      // Let browser handle Ctrl+C/V/X/A/Z, F5, F12, etc.
      if (e.ctrlKey && ["c", "v", "x", "a", "z", "y"].includes(e.key.toLowerCase())) return;
      if (e.key.startsWith("F") && e.key.length > 1) return;

      e.preventDefault();
      const changed = editor.handle_key(e.key, e.ctrlKey, e.shiftKey);
      if (changed) {
        editor.render();
      }
    };

    const onClick = (e: MouseEvent) => {
      const rect = canvas.getBoundingClientRect();
      const x = (e.clientX - rect.left) * devicePixelRatio;
      const y = (e.clientY - rect.top) * devicePixelRatio;
      editor.click(x, y);
      editor.render();
      canvas.focus();
    };

    canvas.addEventListener("keydown", onKeyDown);
    canvas.addEventListener("mousedown", onClick);
    return () => {
      canvas.removeEventListener("keydown", onKeyDown);
      canvas.removeEventListener("mousedown", onClick);
    };
  }, [status]);

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
