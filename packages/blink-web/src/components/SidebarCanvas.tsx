import { useRef, useEffect, useState } from "react";
import type { FileEntry } from "../hooks/useFileSystem";
import { useFileSystem } from "../hooks/useFileSystem";

interface FlatEntry {
  name: string;
  depth: number;
  is_dir: boolean;
  expanded: boolean;
  is_last: boolean[];
  entry: FileEntry;
}

function flattenTree(
  entries: FileEntry[],
  depth: number,
  parentIsLast: boolean[],
  expandedSet: Set<string>,
  childrenMap: Map<string, FileEntry[]>
): FlatEntry[] {
  const result: FlatEntry[] = [];
  entries.forEach((entry, i) => {
    const isLast = i === entries.length - 1;
    const isLastArr = [...parentIsLast, isLast];
    const expanded =
      entry.kind === "directory" && expandedSet.has(entry.path);
    result.push({
      name: entry.name,
      depth,
      is_dir: entry.kind === "directory",
      expanded,
      is_last: parentIsLast,
      entry,
    });
    if (expanded) {
      const children = childrenMap.get(entry.path) ?? entry.children ?? [];
      result.push(
        ...flattenTree(children, depth + 1, isLastArr, expandedSet, childrenMap)
      );
    }
  });
  return result;
}

interface Props {
  showGuides: boolean;
}

export default function SidebarCanvas({ showGuides }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const rendererRef = useRef<any>(null);
  const flatRef = useRef<FlatEntry[]>([]);
  const expandedRef = useRef<Set<string>>(new Set());
  const childrenMapRef = useRef<Map<string, FileEntry[]>>(new Map());
  const rootEntriesRef = useRef<FileEntry[]>([]);
  const showGuidesRef = useRef(showGuides);
  const [ready, setReady] = useState(false);
  const { rootEntries, openFile, loadChildren } = useFileSystem();

  // Keep refs in sync
  rootEntriesRef.current = rootEntries;
  showGuidesRef.current = showGuides;

  function renderTree() {
    const renderer = rendererRef.current;
    if (!renderer) return;

    renderer.set_guides_visible(showGuidesRef.current);

    const flat = flattenTree(
      rootEntriesRef.current,
      0,
      [],
      expandedRef.current,
      childrenMapRef.current
    );
    flatRef.current = flat;

    const entries = flat.map((f) => ({
      name: f.name,
      depth: f.depth,
      is_dir: f.is_dir,
      expanded: f.expanded,
      is_last: f.is_last,
    }));

    console.log("SidebarCanvas: rendering", entries.length, "entries");
    if (entries.length > 0) console.log("SidebarCanvas: first entry:", entries[0]);
    renderer.render(entries);
  }

  // Init renderer
  useEffect(() => {
    let cancelled = false;

    async function init() {
      const canvas = canvasRef.current;
      if (!canvas || !navigator.gpu) return;

      // Set canvas size before creating renderer
      const rect = canvas.getBoundingClientRect();
      const w = Math.max(1, Math.round(rect.width * devicePixelRatio));
      const h = Math.max(1, Math.round(rect.height * devicePixelRatio));
      canvas.width = w;
      canvas.height = h;

      const blink = await import("../../wasm/blink_core");
      await blink.default();

      if (cancelled) return;

      const fontResp = await fetch("/fonts/CursorGothic-Regular.ttf");
      if (!fontResp.ok) {
        console.error("SidebarCanvas: font not found");
        return;
      }
      const fontData = new Uint8Array(await fontResp.arrayBuffer());

      if (cancelled) return;

      console.log("SidebarCanvas: creating renderer, canvas size:", w, h);
      const renderer = await blink.SidebarRenderer.create(
        "sidebar-canvas",
        fontData,
        devicePixelRatio
      );
      rendererRef.current = renderer;
      console.log("SidebarCanvas: renderer created");

      if (cancelled) return;

      setReady(true);
      console.log("SidebarCanvas: rootEntries count:", rootEntriesRef.current.length);
      renderTree();
    }

    init();
    return () => {
      cancelled = true;
    };
  }, []);

  // Re-render when ready, rootEntries, or guides change
  useEffect(() => {
    if (!ready) return;
    renderTree();
  }, [ready, rootEntries, showGuides]);

  // Resize observer
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !ready) return;

    const observer = new ResizeObserver((entries) => {
      const renderer = rendererRef.current;
      if (!renderer) return;
      for (const entry of entries) {
        const { width, height } = entry.contentRect;
        const pw = Math.max(1, Math.round(width * devicePixelRatio));
        const ph = Math.max(1, Math.round(height * devicePixelRatio));
        canvas.width = pw;
        canvas.height = ph;
        renderer.resize(pw, ph);
        renderTree();
      }
    });

    observer.observe(canvas);
    return () => observer.disconnect();
  }, [ready]);

  // Mouse events
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !ready) return;

    const onMouseMove = (e: MouseEvent) => {
      const renderer = rendererRef.current;
      if (!renderer) return;
      const rect = canvas.getBoundingClientRect();
      const y = (e.clientY - rect.top) * devicePixelRatio;
      const index = renderer.hit_test(0, y);
      renderer.set_hover(index);
      renderTree();
    };

    const onMouseLeave = () => {
      const renderer = rendererRef.current;
      if (!renderer) return;
      renderer.set_hover(-1);
      renderTree();
    };

    const onClick = async (e: MouseEvent) => {
      const renderer = rendererRef.current;
      if (!renderer) return;
      const rect = canvas.getBoundingClientRect();
      const y = (e.clientY - rect.top) * devicePixelRatio;
      const index = renderer.hit_test(0, y);
      if (index < 0 || index >= flatRef.current.length) return;

      const flat = flatRef.current[index];
      if (flat.is_dir) {
        const path = flat.entry.path;
        if (expandedRef.current.has(path)) {
          expandedRef.current.delete(path);
        } else {
          if (!childrenMapRef.current.has(path)) {
            const kids = await loadChildren(flat.entry);
            childrenMapRef.current.set(path, kids);
          }
          expandedRef.current.add(path);
        }
        renderTree();
      } else {
        openFile(flat.entry);
      }
    };

    canvas.addEventListener("mousemove", onMouseMove);
    canvas.addEventListener("mouseleave", onMouseLeave);
    canvas.addEventListener("click", onClick);
    return () => {
      canvas.removeEventListener("mousemove", onMouseMove);
      canvas.removeEventListener("mouseleave", onMouseLeave);
      canvas.removeEventListener("click", onClick);
    };
  }, [ready, openFile, loadChildren]);

  return (
    <canvas
      id="sidebar-canvas"
      ref={canvasRef}
      style={{ width: "100%", height: "100%", display: "block" }}
    />
  );
}
