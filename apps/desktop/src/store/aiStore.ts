import { create } from "zustand";

import type { AiSettings, AiStatus, SearchHit, SimilarFile } from "../types";

export type AiSearchMode = "translate" | "semantic";

const DEFAULT_SETTINGS: AiSettings = {
  cloud_enabled: false,
  cloud_base_url: null,
  cloud_model: null,
  cloud_api_key: null,
  chat_model: null,
};

interface AiStore {
  status: AiStatus | null;
  settings: AiSettings;
  mode: AiSearchMode;
  query: string;
  translatedQueryText: string | null;
  usedFallback: boolean;
  filterResults: SearchHit[];
  semanticResults: SimilarFile[];
  embedProgress: { filesEmbedded: number; filesTotal: number } | null;
  loading: boolean;
  error: string | null;
  pendingCloudConfirm: boolean;

  setStatus: (status: AiStatus) => void;
  setSettings: (settings: AiSettings) => void;
  setMode: (mode: AiSearchMode) => void;
  setQuery: (query: string) => void;
  setTranslation: (queryText: string, usedFallback: boolean) => void;
  setFilterResults: (results: SearchHit[]) => void;
  setSemanticResults: (results: SimilarFile[]) => void;
  setEmbedProgress: (progress: { filesEmbedded: number; filesTotal: number } | null) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  setPendingCloudConfirm: (pending: boolean) => void;
}

export const useAiStore = create<AiStore>((set) => ({
  status: null,
  settings: DEFAULT_SETTINGS,
  mode: "translate",
  query: "",
  translatedQueryText: null,
  usedFallback: false,
  filterResults: [],
  semanticResults: [],
  embedProgress: null,
  loading: false,
  error: null,
  pendingCloudConfirm: false,

  setStatus: (status) => set({ status }),
  setSettings: (settings) => set({ settings }),
  setMode: (mode) =>
    set({ mode, filterResults: [], semanticResults: [], translatedQueryText: null }),
  setQuery: (query) => set({ query }),
  setTranslation: (queryText, usedFallback) =>
    set({ translatedQueryText: queryText, usedFallback }),
  setFilterResults: (filterResults) => set({ filterResults }),
  setSemanticResults: (semanticResults) => set({ semanticResults }),
  setEmbedProgress: (embedProgress) => set({ embedProgress }),
  setLoading: (loading) => set({ loading }),
  setError: (error) => set({ error }),
  setPendingCloudConfirm: (pendingCloudConfirm) => set({ pendingCloudConfirm }),
}));
