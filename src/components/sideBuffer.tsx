import { ReactElement, useState } from "react";
import React from "react"
import { FileEntry } from "./fileEntry";
import { File } from "../types/file.ts"
import '../App.css'

export const SideBuffer = (props: { files: File[] }) => {
  const [selectedElement, setSelectedElement] = useState<number>(-1)

  return (
    <div className="file-tree">
      {props.files.map((file, index) => (
        <FileEntry
          key={index}
          id={index}
          selectedElement={selectedElement}
          setSelectedElement={setSelectedElement}
          file={file}
        />p
      ))}
    </div>
  )
}
