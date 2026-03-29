import { useState } from "react";
import reactLogo from "./assets/react.svg";
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
  return (
    <div className="file-tree">
      {props.files.map(file =>(
        <div className="file-entry">{file.name}</div>
      ))}
    </div>
  )
}
