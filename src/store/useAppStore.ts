import { create } from "zustand";
import { File } from "../types/file.ts"
import { ReactNode } from "react";

interface AppStore {
  fileElement: ReactNode | undefined,
  currentFile: File | undefined,
  selectFile: (e:React.MouseEvent, file:File) => Promise<void>
}

export const useAppStore = create<AppStore>((set, get) => ({
  currentFile: undefined,
  fileElement: undefined,
  selectFile: async (e, file) =>{
    const selectedFile = get().fileElement
    if(file) {
    }
    set({currentFile: file});

  },
}))

