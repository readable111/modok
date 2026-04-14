import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { File } from "../types/file.ts"
import { ReactNode } from "react";

interface AppStore {
  fileBuffer: String,
  fetchBuffer: (dir: String) => Promise<void>,
}

export const useAppStore = create<AppStore>((set, get) => ({
  fileBuffer: "",

  fetchBuffer: async (dir) =>{
    try{
      const files = await invoke<string>("open_and_read_buffer", {filePath: dir})
      set({fileBuffer: files})
    } catch (err) {
      console.error("Something went wrong", err)
    }
  }
}))

