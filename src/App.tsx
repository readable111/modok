import { useState } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/core";
import { open } from '@tauri-apps/plugin-dialog'
import "./App.css";

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");
  const [files, setFiles] = useState<string[]>([])

  async function greet() {
    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
    setGreetMsg(await invoke("greet", { name }));
  }

  async function pickFolder() {
    const path = await open({
      directory: true,
      multiple: false,
    })

    if (path) {
      const result = await invoke<string[]>('open_directory', { dirPath: path });
      setFiles(result)
    }
  }

  return (
    <main>
      <div>
        <button onClick={pickFolder}>Open Folder</button>
        {files.map(name =>(
          <p>{name}</p>
        ))}
      </div>
    </main>
  );
}

export default App;
