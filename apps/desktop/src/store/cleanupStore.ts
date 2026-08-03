import { create } from "zustand";

import type { Recommendation } from "../types";

interface CleanupStore {
  recommendations: Recommendation[];
  selectedPaths: Set<string>;
  loading: boolean;
  error: string | null;
  setRecommendations: (recommendations: Recommendation[]) => void;
  togglePath: (path: string) => void;
  setGroupSelected: (paths: string[], selected: boolean) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
}

/** High-confidence items are pre-selected; Medium/Low are left for manual review. */
function defaultSelection(recommendations: Recommendation[]): Set<string> {
  const selected = new Set<string>();
  for (const rec of recommendations) {
    if (rec.confidence === "High") {
      for (const item of rec.items) selected.add(item.path);
    }
  }
  return selected;
}

export const useCleanupStore = create<CleanupStore>((set) => ({
  recommendations: [],
  selectedPaths: new Set(),
  loading: false,
  error: null,
  setRecommendations: (recommendations) =>
    set({ recommendations, selectedPaths: defaultSelection(recommendations) }),
  togglePath: (path) =>
    set((state) => {
      const next = new Set(state.selectedPaths);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return { selectedPaths: next };
    }),
  setGroupSelected: (paths, selected) =>
    set((state) => {
      const next = new Set(state.selectedPaths);
      for (const path of paths) {
        if (selected) {
          next.add(path);
        } else {
          next.delete(path);
        }
      }
      return { selectedPaths: next };
    }),
  setLoading: (loading) => set({ loading }),
  setError: (error) => set({ error }),
}));

export function bytesForSelection(
  recommendations: Recommendation[],
  selected: Set<string>,
): number {
  return recommendations.reduce(
    (total, rec) =>
      total + rec.items.filter((i) => selected.has(i.path)).reduce((s, i) => s + i.size_bytes, 0),
    0,
  );
}
