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
  content: string;
}

interface FileEntry {
  name: string;
  kind: "file" | "directory";
}

interface FileSystemContextValue {
  directoryHandle: FileSystemDirectoryHandle | null;
  entries: FileEntry[];
  openFiles: OpenFile[];
  activeFile: OpenFile | null;
  openDirectory: () => Promise<void>;
  openFile: (name: string, content?: string) => void;
  closeFile: (name: string) => void;
  setActiveFile: (name: string) => void;
  readFile: (name: string) => Promise<string>;
  writeFile: (name: string, content: string) => Promise<void>;
}

const FileSystemContext = createContext<FileSystemContextValue>(null!);

export function useFileSystem() {
  return useContext(FileSystemContext);
}

function supportsFileSystemAccess(): boolean {
  return "showDirectoryPicker" in window;
}

export function FileSystemProvider({ children }: { children: ReactNode }) {
  const [directoryHandle, setDirectoryHandle] =
    useState<FileSystemDirectoryHandle | null>(null);
  const [entries, setEntries] = useState<FileEntry[]>([]);
  const [openFiles, setOpenFiles] = useState<OpenFile[]>([]);
  const [activeFile, setActiveFileState] = useState<OpenFile | null>(null);

  const openDirectory = useCallback(async () => {
    if (supportsFileSystemAccess()) {
      try {
        const handle = await (window as any).showDirectoryPicker({ mode: "readwrite" });
        setDirectoryHandle(handle);

        const fileEntries: FileEntry[] = [];
        for await (const entry of handle.values()) {
          fileEntries.push({
            name: entry.name,
            kind: entry.kind,
          });
        }
        fileEntries.sort((a, b) => {
          if (a.kind !== b.kind) return a.kind === "directory" ? -1 : 1;
          return a.name.localeCompare(b.name);
        });
        setEntries(fileEntries);
      } catch (err) {
        console.error("Failed to open directory:", err);
      }
    } else {
      // IndexedDB fallback - load stored file list
      const stored = await get<FileEntry[]>("blink-files");
      if (stored) {
        setEntries(stored);
      }
    }
  }, []);

  const readFile = useCallback(
    async (name: string): Promise<string> => {
      if (directoryHandle) {
        const fileHandle = await directoryHandle.getFileHandle(name);
        const file = await fileHandle.getFile();
        return file.text();
      }
      // IndexedDB fallback
      const content = await get<string>(`blink-file-${name}`);
      return content ?? "";
    },
    [directoryHandle]
  );

  const writeFile = useCallback(
    async (name: string, content: string) => {
      if (directoryHandle) {
        const fileHandle = await directoryHandle.getFileHandle(name, {
          create: true,
        });
        const writable = await fileHandle.createWritable();
        await writable.write(content);
        await writable.close();
      } else {
        // IndexedDB fallback
        await set(`blink-file-${name}`, content);
      }
    },
    [directoryHandle]
  );

  const openFile = useCallback(
    async (name: string, _content?: string) => {
      // Check if already open
      const existing = openFiles.find((f) => f.name === name);
      if (existing) {
        setActiveFileState(existing);
        return;
      }

      const content = await readFile(name);
      const file: OpenFile = { name, content };
      setOpenFiles((prev) => [...prev, file]);
      setActiveFileState(file);
    },
    [openFiles, readFile]
  );

  const closeFile = useCallback(
    (name: string) => {
      setOpenFiles((prev) => {
        const next = prev.filter((f) => f.name !== name);
        if (activeFile?.name === name) {
          setActiveFileState(next.length > 0 ? next[next.length - 1] : null);
        }
        return next;
      });
    },
    [activeFile]
  );

  const setActiveFile = useCallback(
    (name: string) => {
      const file = openFiles.find((f) => f.name === name);
      if (file) setActiveFileState(file);
    },
    [openFiles]
  );

  return (
    <FileSystemContext.Provider
      value={{
        directoryHandle,
        entries,
        openFiles,
        activeFile,
        openDirectory,
        openFile,
        closeFile,
        setActiveFile,
        readFile,
        writeFile,
      }}
    >
      {children}
    </FileSystemContext.Provider>
  );
}
