import { ReactElement, useState } from "react";
import React from "react"
import { useAppStore } from "../store/useAppStore";
import { FileEntry } from "./fileEntry";
import { File } from "../types/file.ts"
import '../App.css'

export const SideBuffer = () => {
  const [selectedElement, setSelectedElement] = useState<number>(-1)
  const { files } = useAppStore();

  return (
    <div className="file-tree">
      {files.map((file:File, index:number) => (
        <FileEntry
          key={index}
          id={index}
          selectedElement={selectedElement}
          setSelectedElement={setSelectedElement}
          file={file}
        />
      ))}
    </div>
  )
}
