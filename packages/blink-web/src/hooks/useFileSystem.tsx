import {
  createContext,
  useContext,
  useState,
  useCallback,
  type ReactNode,
} from "react";
import { get, set } from "idb-keyval";

export interface OpenFile {
  name: string;
  path: string;
  content: string;
}

export interface FileEntry {
  name: string;
  path: string;
  kind: "file" | "directory";
  children?: FileEntry[];
  loaded?: boolean;
  handle?: FileSystemDirectoryHandle | FileSystemFileHandle;
}

interface FileSystemContextValue {
  directoryHandle: FileSystemDirectoryHandle | null;
  rootEntries: FileEntry[];
  openFiles: OpenFile[];
  activeFile: OpenFile | null;
  openDirectory: (handle?: FileSystemDirectoryHandle) => Promise<void>;
  loadChildren: (entry: FileEntry) => Promise<FileEntry[]>;
  openFile: (entry: FileEntry) => Promise<void>;
  closeFile: (path: string) => void;
  setActiveFile: (path: string) => void;
  readFileByHandle: (handle: FileSystemFileHandle) => Promise<string>;
}

const FileSystemContext = createContext<FileSystemContextValue>(null!);

export function useFileSystem() {
  return useContext(FileSystemContext);
}

function supportsFileSystemAccess(): boolean {
  return "showDirectoryPicker" in window;
}

async function readDirEntries(
  dirHandle: FileSystemDirectoryHandle,
  parentPath: string
): Promise<FileEntry[]> {
  const entries: FileEntry[] = [];
  for await (const entry of (dirHandle as any).values()) {
    entries.push({
      name: entry.name,
      path: parentPath ? `${parentPath}/${entry.name}` : entry.name,
      kind: entry.kind,
      handle: entry,
      children: entry.kind === "directory" ? [] : undefined,
      loaded: entry.kind !== "directory",
    });
  }
  entries.sort((a, b) => {
    if (a.kind !== b.kind) return a.kind === "directory" ? -1 : 1;
    return a.name.localeCompare(b.name);
  });
  return entries;
}

export function FileSystemProvider({ children }: { children: ReactNode }) {
  const [directoryHandle, setDirectoryHandle] =
    useState<FileSystemDirectoryHandle | null>(null);
  const [rootEntries, setRootEntries] = useState<FileEntry[]>([]);
  const [openFiles, setOpenFiles] = useState<OpenFile[]>([]);
  const [activeFile, setActiveFileState] = useState<OpenFile | null>(null);

  const openDirectory = useCallback(async (existingHandle?: FileSystemDirectoryHandle) => {
    if (supportsFileSystemAccess()) {
      try {
        const handle = existingHandle ?? await (window as any).showDirectoryPicker({
          mode: "readwrite",
        });
        setDirectoryHandle(handle);
        const entries = await readDirEntries(handle, "");
        setRootEntries(entries);

        // Save to recent projects
        const recent = (await get<Array<{ name: string; path: string; handle: FileSystemDirectoryHandle }>>("blink-recent-projects")) ?? [];
        const filtered = recent.filter((p) => p.name !== handle.name);
        const updated = [{ name: handle.name, handle }, ...filtered].slice(0, 10);
        await set("blink-recent-projects", updated);
      } catch (err) {
        console.error("Failed to open directory:", err);
      }
    } else {
      const stored = await get<FileEntry[]>("blink-files");
      if (stored) {
        setRootEntries(stored);
      }
    }
  }, []);

  const loadChildren = useCallback(
    async (entry: FileEntry): Promise<FileEntry[]> => {
      if (entry.kind !== "directory" || !entry.handle) return [];
      const dirHandle = entry.handle as FileSystemDirectoryHandle;
      const children = await readDirEntries(dirHandle, entry.path);
      return children;
    },
    []
  );

  const readFileByHandle = useCallback(
    async (handle: FileSystemFileHandle): Promise<string> => {
      const file = await handle.getFile();
      return file.text();
    },
    []
  );

  const openFile = useCallback(
    async (entry: FileEntry) => {
      const existing = openFiles.find((f) => f.path === entry.path);
      if (existing) {
        setActiveFileState(existing);
        return;
      }

      let content = "";
      if (entry.handle && entry.kind === "file") {
        content = await readFileByHandle(
          entry.handle as FileSystemFileHandle
        );
      } else {
        const stored = await get<string>(`blink-file-${entry.path}`);
        content = stored ?? "";
      }

      const file: OpenFile = { name: entry.name, path: entry.path, content };
      setOpenFiles((prev) => [...prev, file]);
      setActiveFileState(file);
    },
    [openFiles, readFileByHandle]
  );

  const closeFile = useCallback(
    (path: string) => {
      setOpenFiles((prev) => {
        const next = prev.filter((f) => f.path !== path);
        if (activeFile?.path === path) {
          setActiveFileState(next.length > 0 ? next[next.length - 1] : null);
        }
        return next;
      });
    },
    [activeFile]
  );

  const setActiveFile = useCallback(
    (path: string) => {
      const file = openFiles.find((f) => f.path === path);
      if (file) setActiveFileState(file);
    },
    [openFiles]
  );

  return (
    <FileSystemContext.Provider
      value={{
        directoryHandle,
        rootEntries,
        openFiles,
        activeFile,
        openDirectory,
        loadChildren,
        openFile,
        closeFile,
        setActiveFile,
        readFileByHandle,
      }}
    >
      {children}
    </FileSystemContext.Provider>
  );
}
