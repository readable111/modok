import { SetStateAction, useState, Dispatch } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/core";
import { open } from '@tauri-apps/plugin-dialog'
import type { File } from "../types/file.d.ts"
import '../App.css'

interface SideBufferProps {
  files: File[],
  readFile: (e:React.MouseEvent<HTMLDivElement>, file:File) => Promise<void>
}

export const SideBuffer = (props :SideBufferProps) => {
  return (
    <div className="file-tree">
      {props.files.map(file =>(
        <div className="file-entry" onClick={(e) => props.readFile(e,file)}>{file.name}</div>
      ))}
    </div>
  )
}
