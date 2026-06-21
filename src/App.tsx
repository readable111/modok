import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from '@tauri-apps/plugin-dialog'
import { SideBuffer } from "./components/sideBuffer";
import { useAppStore } from "./store/useAppStore";
import { File } from "./types/file";
import "./App.css";

function App() {
  const { fileBuffer, fetchBuffer } = useAppStore()

  const [files, setFiles] = useState<[{}]>([{}])

  async function pickFolder() {
    const path = await open({
      directory: true,
      multiple: false,
    })

    if (path) {
      const result = await invoke<File[]>('open_directory', { dirPath: path });
      setFiles(result)
      console.log(result)
    }
  }

  async function readFile(e: React.MouseEvent<HTMLDivElement>, file: File) {
    await fetchBuffer(file.path)
    e.currentTarget.classList.add("active")
  }

  return (
    <main>
      <div className="content">
        <div className="side-container">
          <div>
            <button onClick={pickFolder}>Open Folder</button>
          </div>
          <SideBuffer files={files}/>
        </div>
        <div className="file-contents">
          <pre>{fileBuffer}</pre>
        </div>
      </div>
    </main>
  );
}

export default App;
