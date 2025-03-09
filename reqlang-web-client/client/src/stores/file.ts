import { ParsedRequestFile } from "reqlang-types";
import { create } from "zustand";

type LoadedFile = {
  fileName: string;
  text: string;
};

interface FileState {
  file: LoadedFile | null;
  parsedFile: ParsedRequestFile | null;

  setFile: (file: LoadedFile) => void;
  setParsedFile: (file: ParsedRequestFile) => void;
}

export const useFileStore = create<FileState>()((set) => ({
  file: null,
  parsedFile: null,
  setFile: (file: LoadedFile) => set({ file }),
  setParsedFile: (file: ParsedRequestFile) => set({ parsedFile: file }),
}));
