import { create } from "zustand";

import type { SavedSearch, SearchHit } from "../types";

interface SearchStore {
  queryText: string;
  results: SearchHit[];
  savedSearches: SavedSearch[];
  loading: boolean;
  error: string | null;
  setQueryText: (queryText: string) => void;
  setResults: (results: SearchHit[]) => void;
  setSavedSearches: (savedSearches: SavedSearch[]) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
}

export const useSearchStore = create<SearchStore>((set) => ({
  queryText: "",
  results: [],
  savedSearches: [],
  loading: false,
  error: null,
  setQueryText: (queryText) => set({ queryText }),
  setResults: (results) => set({ results }),
  setSavedSearches: (savedSearches) => set({ savedSearches }),
  setLoading: (loading) => set({ loading }),
  setError: (error) => set({ error }),
}));
