import { Dispatch, SetStateAction } from "react"

interface FileEntryProps {
  id: number
  selectedElement: number
  setSelectedElement: Dispatch<SetStateAction<number>>
  onClick?: () => void
  file: File
}

export const FileEntry = ({ id, selectedElement, setSelectedElement, onClick, file }: FileEntryProps) => {
  const isSelected = selectedElement === id
  const style = isSelected ? "file-entry selected" : "file-entry"

  function handleClick() {
    setSelectedElement(id)
    onClick?.()
  }

  return (
    <div className={style} onClick={handleClick}>
      {file.name}
    </div>
  )
}
