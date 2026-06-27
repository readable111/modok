import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { File } from "../types/file.ts"
import { ReactNode } from "react";

interface AppStore {
  fileBuffer: String,
  cursor: {
    offset: number,
    line: number,
    col: number,
  },
  fetchBuffer: (dir: String) => Promise<void>,
  moveCursor: (offset: number) => Promise<void>,
}

export const useAppStore = create<AppStore>((set, get) => ({
  fileBuffer: "",
  cursor: {
    offset: 0,
    line: 0,
    col: 0
  },

  fetchBuffer: async (dir) => {
    try{
      const files = await invoke<string>("open_and_read_buffer", {filePath: dir})
      set({fileBuffer: files})
    } catch (err) {
      console.error("Something went wrong", err)
    }
  },

  moveCursor: async (offset) => {
    try {
      const [line, col] = await invoke<Array<number>>("get_offset_position", { offset: offset })
      set(() => ({
        cursor: {
          offset: offset,
          line: line,
          col: col,
        }
      }))
    } catch (err) {
      console.error("Something went wrong", err)
    }
  }
}))

