import { useEffect, useRef, useState } from "react";

import { deleteSavedSearch, listSavedSearches, saveSearch, searchFiles } from "../lib/atlas";
import { useSearchStore } from "../store/searchStore";
import SavedSearchesPanel from "./SavedSearchesPanel";
import SearchBar from "./SearchBar";
import SearchResultsList from "./SearchResultsList";

const RESULT_LIMIT = 100;
const DEBOUNCE_MS = 250;

export default function SearchView() {
  const queryText = useSearchStore((s) => s.queryText);
  const results = useSearchStore((s) => s.results);
  const savedSearches = useSearchStore((s) => s.savedSearches);
  const loading = useSearchStore((s) => s.loading);
  const error = useSearchStore((s) => s.error);
  const setQueryText = useSearchStore((s) => s.setQueryText);
  const setResults = useSearchStore((s) => s.setResults);
  const setSavedSearches = useSearchStore((s) => s.setSavedSearches);
  const setLoading = useSearchStore((s) => s.setLoading);
  const setError = useSearchStore((s) => s.setError);

  const [savingName, setSavingName] = useState<string | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const refreshSavedSearches = () => {
    listSavedSearches()
      .then(setSavedSearches)
      .catch((err: unknown) => setError(String(err)));
  };

  useEffect(() => {
    refreshSavedSearches();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    if (queryText.trim().length === 0) {
      setResults([]);
      return;
    }
    debounceRef.current = setTimeout(() => {
      setLoading(true);
      searchFiles(queryText, RESULT_LIMIT)
        .then(setResults)
        .catch((err: unknown) => setError(String(err)))
        .finally(() => setLoading(false));
    }, DEBOUNCE_MS);
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [queryText]);

  function handleSaveClick() {
    setSavingName(queryText.trim());
  }

  function confirmSave() {
    const name = savingName?.trim();
    if (!name) return;
    saveSearch(name, queryText)
      .then(() => {
        setSavingName(null);
        refreshSavedSearches();
      })
      .catch((err: unknown) => setError(String(err)));
  }

  function handleRunSaved(text: string) {
    setQueryText(text);
  }

  function handleDeleteSaved(id: number) {
    deleteSavedSearch(id)
      .then(refreshSavedSearches)
      .catch((err: unknown) => setError(String(err)));
  }

  return (
    <main className="min-h-screen px-6 py-10 max-w-3xl mx-auto">
      <p className="text-xs uppercase tracking-widest text-[color:var(--color-atlas-muted)] mb-2">
        Search
      </p>
      <h1 className="text-2xl font-semibold mb-6">Find anything you have indexed</h1>

      <SearchBar
        value={queryText}
        onChange={setQueryText}
        onSave={handleSaveClick}
        canSave={queryText.trim().length > 0}
      />

      {savingName !== null && (
        <div className="mt-3 flex items-center gap-2">
          <input
            type="text"
            autoFocus
            value={savingName}
            onChange={(e) => setSavingName(e.target.value)}
            placeholder="Name this search"
            className="flex-1 rounded-lg border border-[color:var(--color-atlas-border)] bg-transparent px-3 py-2 text-sm focus:outline-none focus:border-[color:var(--color-atlas-accent)]"
          />
          <button
            type="button"
            onClick={confirmSave}
            className="rounded-lg bg-[color:var(--color-atlas-accent)] text-[#0b0d10] text-sm font-medium px-3 py-2"
          >
            Save
          </button>
          <button
            type="button"
            onClick={() => setSavingName(null)}
            className="text-sm text-[color:var(--color-atlas-muted)] px-2"
          >
            Cancel
          </button>
        </div>
      )}

      {error && <p className="mt-3 text-sm text-red-400">{error}</p>}

      <div className="grid grid-cols-1 md:grid-cols-3 gap-8 mt-8">
        <section className="md:col-span-2">
          <SearchResultsList
            results={results}
            loading={loading}
            hasQuery={queryText.trim().length > 0}
          />
        </section>
        <section>
          <h2 className="text-sm font-medium text-[color:var(--color-atlas-muted)] mb-3">
            Saved searches
          </h2>
          <SavedSearchesPanel
            savedSearches={savedSearches}
            onRun={handleRunSaved}
            onDelete={handleDeleteSaved}
          />
        </section>
      </div>
    </main>
  );
}
