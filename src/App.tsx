import { useState } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/core";
import { open } from '@tauri-apps/plugin-dialog'
import { SideBuffer } from "./components/sideBuffer";
import type { File } from "./types/file.ts"
import "./App.css";

function App() {
  const [files, setFiles] = useState<File[]>([])
  const [openFile, setOpenFile] = useState<File>()
  const [fileContents, setFileContents]= useState<String>()

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

  async function readFile(e:React.MouseEvent<HTMLDivElement>, file:File) {
    const contents = await invoke<String>("open_and_read_buffer", {file_path: file.path});
    setFileContents(contents)
    e.currentTarget.classList.add("active")
    setOpenFile(file)
  }

  return (
    <main>
      <SideBuffer files={files} readFile={readFile}/>
      <div>
        <button onClick={pickFolder}>Open Folder</button>
      </div>
      <div>{fileContents}</div>
    </main>
  );
}

export default App;
