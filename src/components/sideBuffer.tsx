import { ReactElement, useState } from "react";
import React from "react"
import { invoke } from "@tauri-apps/api/core";
import { open } from '@tauri-apps/plugin-dialog'
import '../App.css'

interface File {
  name: string,
  path: string,
  is_dir: boolean,
  extension: string
}

export const SideBuffer = (props : {files:File[]}) => {
  const [openFile, setOpenFile] = useState<File>()
  const [selectedElement,setSelectedElement] = useState<Element>()

  async function select(event:React.MouseEvent , file:File){
    const current = event.currentTarget;
    if (selectedElement) {
      selectedElement.classList.remove('selected');
    }
    // call some tauri function here to read the file for the first x lines
    current.classList.add('selected');
    setSelectedElement(current);
  }

  return (
    <div className="file-tree">
      {props.files.map(file =>(
        <div onClick={select} className="file-entry">{file.name}</div>
      ))}
    </div>
  )
}
