import { ParseResult } from "reqlang-types";
import { create } from "zustand";

type LoadedFile = {
  fileName: string;
  text: string;
};

interface FileState {
  file: LoadedFile | null;
  parsedFile: ParseResult | null;

  setFile: (file: LoadedFile) => void;
  setParsedFile: (file: ParseResult) => void;
}

export const useFileStore = create<FileState>()((set) => ({
  file: null,
  parsedFile: null,
  setFile: (file: LoadedFile) => set({ file }),
  setParsedFile: (file: ParseResult) => set({ parsedFile: file }),
}));
