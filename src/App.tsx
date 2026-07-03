import { useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { SideBuffer } from "./components/sideBuffer";
import { useAppStore } from "./store/useAppStore";
import "./App.css";

function App() {
  const { fileBuffer, cursor, moveCursor, openFolder } = useAppStore()
  const textAreaRef = useRef(null)

  const handleMouseUp = (event:MouseEvent) => {
    const cursorStart:number = textAreaRef.current.selectionStart;
    moveCursor(cursorStart);
    console.log(cursor);
  }

  const handleChange = async () => {
      await invoke("write_changes", )
  }

  return (
    <main>
      <div className="content">
        <div className="side-container">
          <div>
            <button onClick={openFolder}>Open Folder</button>
          </div>
          <SideBuffer/>
        </div>
        <div className="file-contents">
          <textarea
            ref={textAreaRef}
            onMouseUp={(e) => {handleMouseUp(e)}}
            value={fileBuffer}
            className="code-area"></textarea>
          <div>Line: {cursor.line}, Column: {cursor.col}</div>
        </div>
      </div>
    </main>
  );
}

export default App;
