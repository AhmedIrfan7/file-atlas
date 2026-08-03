import { create } from "zustand";

export type ScreenState = "loading" | "onboarding" | "scanning" | "home" | "search" | "duplicates";

interface ScanProgressState {
  currentRoot: string | null;
  filesSeen: number;
  bytesSeen: number;
}

interface ScanStore {
  screen: ScreenState;
  progress: ScanProgressState;
  lastError: string | null;
  setScreen: (screen: ScreenState) => void;
  setProgress: (progress: ScanProgressState) => void;
  setError: (message: string | null) => void;
  resetProgress: () => void;
}

const initialProgress: ScanProgressState = {
  currentRoot: null,
  filesSeen: 0,
  bytesSeen: 0,
};

export const useScanStore = create<ScanStore>((set) => ({
  screen: "loading",
  progress: initialProgress,
  lastError: null,
  setScreen: (screen) => set({ screen }),
  setProgress: (progress) => set({ progress }),
  setError: (lastError) => set({ lastError }),
  resetProgress: () => set({ progress: initialProgress }),
}));
