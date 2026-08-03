import { create } from "zustand";

import type { StorageNode } from "../types";

export interface Breadcrumb {
  path: string | null;
  label: string;
}

const ROOT_BREADCRUMB: Breadcrumb = { path: null, label: "All" };

interface StorageMapStore {
  breadcrumbs: Breadcrumb[];
  category: string | null;
  sinceDays: number | null;
  nodes: StorageNode[];
  totalBytes: number;
  loading: boolean;
  error: string | null;
  drillInto: (path: string, label: string) => void;
  jumpTo: (index: number) => void;
  setCategory: (category: string | null) => void;
  setSinceDays: (sinceDays: number | null) => void;
  setResult: (nodes: StorageNode[], totalBytes: number) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
}

export const useStorageMapStore = create<StorageMapStore>((set) => ({
  breadcrumbs: [ROOT_BREADCRUMB],
  category: null,
  sinceDays: null,
  nodes: [],
  totalBytes: 0,
  loading: false,
  error: null,
  drillInto: (path, label) =>
    set((state) => ({ breadcrumbs: [...state.breadcrumbs, { path, label }] })),
  jumpTo: (index) => set((state) => ({ breadcrumbs: state.breadcrumbs.slice(0, index + 1) })),
  setCategory: (category) => set({ category }),
  setSinceDays: (sinceDays) => set({ sinceDays }),
  setResult: (nodes, totalBytes) => set({ nodes, totalBytes }),
  setLoading: (loading) => set({ loading }),
  setError: (error) => set({ error }),
}));

export function currentPath(breadcrumbs: Breadcrumb[]): string | null {
  return breadcrumbs[breadcrumbs.length - 1]?.path ?? null;
}
