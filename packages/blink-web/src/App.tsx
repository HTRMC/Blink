import EditorCanvas from "./components/EditorCanvas";
import Sidebar from "./components/Sidebar";
import TabBar from "./components/TabBar";
import StatusBar from "./components/StatusBar";
import { FileSystemProvider, useFileSystem } from "./hooks/useFileSystem";

function AppInner() {
  const { openFiles, activeFile, openFile, closeFile, setActiveFile } = useFileSystem();

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100vh" }}>
      <TabBar
        tabs={openFiles}
        activeFile={activeFile}
        onSelect={setActiveFile}
        onClose={closeFile}
      />
      <div style={{ display: "flex", flex: 1, overflow: "hidden" }}>
        <Sidebar onFileSelect={openFile} />
        <EditorCanvas activeFile={activeFile} />
      </div>
      <StatusBar activeFile={activeFile} />
    </div>
  );
}

export default function App() {
  return (
    <FileSystemProvider>
      <AppInner />
    </FileSystemProvider>
  );
}
