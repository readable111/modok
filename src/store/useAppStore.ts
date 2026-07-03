import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { File } from "../types/file.ts"
import { open } from '@tauri-apps/plugin-dialog'
import { ReactNode } from "react";

interface AppStore {
  fileBuffer: String,
  files: File[],
  cursor: {
    offset: number,
    line: number,
    col: number,
  },
  fetchBuffer: (dir: String) => Promise<void>,
  moveCursor: (offset: number) => Promise<void>,
  openFolder: () => Promise<void>,
}

export const useAppStore = create<AppStore>((set, get) => ({
  files: [],
  fileBuffer: "",
  cursor: {
    offset: 0,
    line: 0,
    col: 0
  },
  
  moveCursor: async (offset) => {
    try {
      const [line, col] = await invoke<Array<number>>("get_offset_position", { offset: offset })
      set(() => ({
        cursor: {
          offset,
          line: line,
          col: col,
        }
      }))
    } catch (err) {
      console.error("Something went wrong", err)
    }
  },

  fetchBuffer: async (dir) => {
    try{
      const files = await invoke<string>("open_and_read_buffer", {filePath: dir})
      set({fileBuffer: files})
      get().moveCursor(0);
    } catch (err) {
      console.error("Something went wrong", err)
    }
  },

  openFolder: async () => {
    const path = await open({
      directory: true,
      multiple: false,
    })

    if (path) {
      const result = await invoke<File[]>('open_directory', { dirPath: path });
      set({files: result})
    }
  }
}))

