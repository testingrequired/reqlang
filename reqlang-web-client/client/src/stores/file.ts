import { create } from "zustand";

type LoadedFile = {
  fileName: string;
  text: string;
};

interface FileState {
  file: LoadedFile | null;

  setFile: (file: LoadedFile) => void;
}

export const useFileStore = create<FileState>()((set) => ({
  file: null,
  setFile: (file: LoadedFile) => set({ file }),
}));
