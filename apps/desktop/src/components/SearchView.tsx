import { useEffect, useRef, useState } from "react";

import { deleteSavedSearch, listSavedSearches, saveSearch, searchFiles } from "../lib/atlas";
import { useSearchStore } from "../store/searchStore";
import SavedSearchesPanel from "./SavedSearchesPanel";
import SearchBar from "./SearchBar";
import SearchResultsList from "./SearchResultsList";
import Button from "./ui/Button";
import Input from "./ui/Input";
import PageHeader from "./ui/PageHeader";
import Panel from "./ui/Panel";

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
  const requestIdRef = useRef(0);

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
      const requestId = ++requestIdRef.current;
      setLoading(true);
      searchFiles(queryText, RESULT_LIMIT)
        .then((hits) => {
          // A slower, older request can resolve after a newer one if the
          // user kept typing past the debounce window; only the latest
          // request's results should ever be allowed to win.
          if (requestId === requestIdRef.current) setResults(hits);
        })
        .catch((err: unknown) => {
          if (requestId === requestIdRef.current) setError(String(err));
        })
        .finally(() => {
          if (requestId === requestIdRef.current) setLoading(false);
        });
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
      <PageHeader eyebrow="Search" title="Find anything you have indexed" />

      <SearchBar
        value={queryText}
        onChange={setQueryText}
        onSave={handleSaveClick}
        canSave={queryText.trim().length > 0}
      />

      {savingName !== null && (
        <div className="mt-3 flex items-center gap-2">
          <Input
            type="text"
            autoFocus
            value={savingName}
            onChange={(e) => setSavingName(e.target.value)}
            placeholder="Name this search"
            className="flex-1"
          />
          <Button variant="primary" onClick={confirmSave}>
            Save
          </Button>
          <Button variant="ghost" onClick={() => setSavingName(null)}>
            Cancel
          </Button>
        </div>
      )}

      {error && <p className="mt-3 text-sm text-[color:var(--color-atlas-danger)]">{error}</p>}

      <div className="grid grid-cols-1 md:grid-cols-3 gap-8 mt-8">
        <section className="md:col-span-2">
          <SearchResultsList
            results={results}
            loading={loading}
            hasQuery={queryText.trim().length > 0}
          />
        </section>
        <Panel className="p-4 h-fit">
          <h2 className="text-sm font-medium text-[color:var(--color-atlas-muted)] mb-3">
            Saved searches
          </h2>
          <SavedSearchesPanel
            savedSearches={savedSearches}
            onRun={handleRunSaved}
            onDelete={handleDeleteSaved}
          />
        </Panel>
      </div>
    </main>
  );
}
