import { Dispatch, SetStateAction } from "react"
import { useAppStore } from "../store/useAppStore"
import { File } from "../types/file.ts"

interface FileEntryProps {
  id: number
  selectedElement: number
  setSelectedElement: Dispatch<SetStateAction<number>>
  onClick?: () => void
  file: File
}

export const FileEntry = ({ id, selectedElement, setSelectedElement, file }: FileEntryProps) => {
  const { fetchBuffer } = useAppStore()

  const isSelected = selectedElement === id
  const style = isSelected ? "file-entry selected" : "file-entry"

  async function handleClick() {
    setSelectedElement(id)
    await fetchBuffer(file.path)
  }

  return (
    <div className={style} onClick={handleClick}>
      {file.name}
    </div>
  )
}
