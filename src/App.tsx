import { useState } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/core";
import { open } from '@tauri-apps/plugin-dialog'
import { SideBuffer } from "./components/sideBuffer";
import "./App.css";

function App() {
  const [files, setFiles] = useState<[{}]>([{}])

  async function pickFolder() {
    const path = await open({
      directory: true,
      multiple: false,
    })

    if (path) {
      const result = await invoke<[{}]>('open_directory', { dirPath: path });
      setFiles(result)
      console.log(result)
    }
  }

  return (
    <main>
      <SideBuffer files={files}/>
      <div>
        <button onClick={pickFolder}>Open Folder</button>
      </div>
    </main>
  );
}

export default App;
