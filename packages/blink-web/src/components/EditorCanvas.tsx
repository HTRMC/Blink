import { useRef, useEffect, useState } from "react";
import type { OpenFile } from "../hooks/useFileSystem";
import { useFileSystem } from "../hooks/useFileSystem";

interface Props {
  activeFile: OpenFile | null;
}

export default function EditorCanvas({ activeFile }: Props) {
  const { pinFile } = useFileSystem();
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
        await editor.init_renderer("editor-canvas", fontData, devicePixelRatio);
        editorRef.current = editor;

        editor.set_language("rs");
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
      const ext = activeFile.name.split(".").pop() ?? "";
      editor.set_language(ext);
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

    const pinActive = () => {
      if (activeFile?.preview) pinFile(activeFile.path);
    };

    const onKeyDown = async (e: KeyboardEvent) => {
      if (e.key.startsWith("F") && e.key.length > 1) return;

      if (e.ctrlKey) {
        const k = e.key.toLowerCase();
        if (k === "c") {
          e.preventDefault();
          const text = editor.get_selection_text();
          if (text) await navigator.clipboard.writeText(text);
          return;
        }
        if (k === "x") {
          e.preventDefault();
          const text = editor.get_selection_text();
          if (text) {
            await navigator.clipboard.writeText(text);
            editor.handle_key("Delete", false, false);
            editor.render();
            pinActive();
          }
          return;
        }
        if (k === "v") {
          e.preventDefault();
          const text = await navigator.clipboard.readText();
          if (text) {
            editor.insert_text(text);
            editor.render();
            pinActive();
          }
          return;
        }
        if (["z", "y"].includes(k)) return;
      }

      e.preventDefault();
      const changed = editor.handle_key(e.key, e.ctrlKey, e.shiftKey);
      if (changed) {
        editor.render();
        // Pin on actual edits (typing, backspace, delete, enter, tab)
        const editKeys = ["Backspace", "Delete", "Enter", "Tab"];
        if (e.key.length === 1 || editKeys.includes(e.key)) {
          pinActive();
        }
      }
    };

    let dragging = false;

    const onMouseDown = (e: MouseEvent) => {
      const rect = canvas.getBoundingClientRect();
      const x = (e.clientX - rect.left) * devicePixelRatio;
      const y = (e.clientY - rect.top) * devicePixelRatio;
      const onScrollbar = editor.click(x, y, e.shiftKey);
      editor.render();
      canvas.focus();
      dragging = true;
      if (onScrollbar) {
        e.preventDefault();
      }
    };

    const onMouseMove = (e: MouseEvent) => {
      if (!dragging) return;
      const rect = canvas.getBoundingClientRect();
      const x = (e.clientX - rect.left) * devicePixelRatio;
      const y = (e.clientY - rect.top) * devicePixelRatio;
      editor.drag(x, y);
      editor.render();
    };

    const onMouseUp = () => {
      dragging = false;
      editor.mouse_up();
    };

    let animFrameId = 0;

    const scheduleFrame = () => {
      if (animFrameId) return;
      animFrameId = requestAnimationFrame(animationLoop);
    };

    const animationLoop = () => {
      animFrameId = 0;
      const stillScrolling = editor.tick();
      editor.render();
      if (stillScrolling) {
        animFrameId = requestAnimationFrame(animationLoop);
      }
    };

    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      editor.scroll(e.deltaY * devicePixelRatio);
      scheduleFrame();
    };

    const wrapper = canvas.parentElement!;

    const onMouseEnter = () => {
      editor.set_canvas_hovered(true);
      scheduleFrame();
    };

    const onMouseLeave = () => {
      editor.set_canvas_hovered(false);
      scheduleFrame();
    };

    canvas.addEventListener("keydown", onKeyDown);
    canvas.addEventListener("mousedown", onMouseDown);
    canvas.addEventListener("wheel", onWheel, { passive: false });
    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
    wrapper.addEventListener("mouseenter", onMouseEnter);
    wrapper.addEventListener("mouseleave", onMouseLeave);
    return () => {
      canvas.removeEventListener("keydown", onKeyDown);
      canvas.removeEventListener("mousedown", onMouseDown);
      canvas.removeEventListener("wheel", onWheel);
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
      wrapper.removeEventListener("mouseenter", onMouseEnter);
      wrapper.removeEventListener("mouseleave", onMouseLeave);
      if (animFrameId) cancelAnimationFrame(animFrameId);
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
