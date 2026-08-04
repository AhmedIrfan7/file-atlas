import { useEffect, useState } from "react";

import {
  getAiSettings,
  getAiStatus,
  searchFiles,
  semanticSearchFiles,
  translateNaturalLanguageQuery,
} from "../lib/atlas";
import { useAiStore } from "../store/aiStore";
import AiSettingsPanel from "./AiSettingsPanel";
import AiStatusBanner from "./AiStatusBanner";
import CloudConfirmDialog from "./CloudConfirmDialog";
import SearchResultsList from "./SearchResultsList";
import SemanticResultsList from "./SemanticResultsList";

export default function AiSearchView() {
  const settings = useAiStore((s) => s.settings);
  const mode = useAiStore((s) => s.mode);
  const query = useAiStore((s) => s.query);
  const translatedQueryText = useAiStore((s) => s.translatedQueryText);
  const usedFallback = useAiStore((s) => s.usedFallback);
  const filterResults = useAiStore((s) => s.filterResults);
  const semanticResults = useAiStore((s) => s.semanticResults);
  const loading = useAiStore((s) => s.loading);
  const error = useAiStore((s) => s.error);
  const pendingCloudConfirm = useAiStore((s) => s.pendingCloudConfirm);
  const setStatus = useAiStore((s) => s.setStatus);
  const setSettings = useAiStore((s) => s.setSettings);
  const setMode = useAiStore((s) => s.setMode);
  const setQuery = useAiStore((s) => s.setQuery);
  const setTranslation = useAiStore((s) => s.setTranslation);
  const setFilterResults = useAiStore((s) => s.setFilterResults);
  const setSemanticResults = useAiStore((s) => s.setSemanticResults);
  const setLoading = useAiStore((s) => s.setLoading);
  const setError = useAiStore((s) => s.setError);
  const setPendingCloudConfirm = useAiStore((s) => s.setPendingCloudConfirm);

  const [hasSearched, setHasSearched] = useState(false);

  useEffect(() => {
    getAiStatus()
      .then(setStatus)
      .catch((err: unknown) => setError(String(err)));
    getAiSettings()
      .then(setSettings)
      .catch(() => undefined);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function runTranslate(useCloud: boolean) {
    setLoading(true);
    setError(null);
    try {
      const translated = await translateNaturalLanguageQuery(query, useCloud);
      setTranslation(translated.query_text, translated.used_fallback);
      const hits = await searchFiles(translated.query_text, 50);
      setFilterResults(hits);
    } catch (err: unknown) {
      setError(String(err));
    } finally {
      setLoading(false);
      setHasSearched(true);
    }
  }

  async function runSemantic() {
    setLoading(true);
    setError(null);
    try {
      const hits = await semanticSearchFiles(query, 30);
      setSemanticResults(hits);
    } catch (err: unknown) {
      setError(String(err));
    } finally {
      setLoading(false);
      setHasSearched(true);
    }
  }

  function handleRun() {
    if (!query.trim()) return;
    if (mode === "translate") {
      if (settings.cloud_enabled) {
        setPendingCloudConfirm(true);
        return;
      }
      void runTranslate(false);
    } else {
      void runSemantic();
    }
  }

  return (
    <main className="min-h-screen px-6 py-10 max-w-4xl mx-auto">
      <p className="text-xs uppercase tracking-widest text-[color:var(--color-atlas-muted)] mb-2">
        AI Search
      </p>
      <h1 className="text-2xl font-semibold mb-1">Search in plain English</h1>
      <p className="text-sm text-[color:var(--color-atlas-muted)] mb-6">
        Runs against a local model by default. Nothing leaves this machine unless you turn on cloud
        AI and confirm it for a specific request.
      </p>

      <AiStatusBanner />
      <AiSettingsPanel />

      <div className="flex items-center gap-1 rounded-lg border border-[color:var(--color-atlas-border)] p-1 mb-4 w-fit">
        {(["translate", "semantic"] as const).map((m) => (
          <button
            key={m}
            type="button"
            onClick={() => setMode(m)}
            className={`rounded-md px-2.5 py-1 text-xs transition-colors ${
              mode === m
                ? "bg-white/10 text-[color:var(--color-atlas-fg)]"
                : "text-[color:var(--color-atlas-muted)] hover:text-[color:var(--color-atlas-fg)]"
            }`}
          >
            {m === "translate" ? "Ask in plain English" : "Semantic search"}
          </button>
        ))}
      </div>

      <div className="flex gap-2 mb-2">
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && handleRun()}
          placeholder={
            mode === "translate"
              ? "e.g. large pdfs from last year"
              : "e.g. tax documents, vacation photos"
          }
          className="flex-1 rounded-lg border border-[color:var(--color-atlas-border)] bg-transparent px-4 py-3 text-sm placeholder:text-[color:var(--color-atlas-muted)] focus:outline-none focus:border-[color:var(--color-atlas-accent)]"
        />
        <button
          type="button"
          onClick={handleRun}
          disabled={loading || !query.trim()}
          className="shrink-0 rounded-lg bg-[color:var(--color-atlas-accent)] text-[#0b0d10] text-sm font-medium px-4 py-3 disabled:opacity-40"
        >
          {loading ? "Working..." : "Search"}
        </button>
      </div>

      {error && <p className="text-sm text-red-400 mb-4">{error}</p>}

      {mode === "translate" && translatedQueryText !== null && (
        <p className="text-xs text-[color:var(--color-atlas-muted)] mb-4">
          {usedFallback ? "Searched as free text: " : "Translated to: "}
          <code className="text-[color:var(--color-atlas-fg)]">{translatedQueryText}</code>
        </p>
      )}

      {hasSearched &&
        (mode === "translate" ? (
          <SearchResultsList results={filterResults} loading={loading} hasQuery />
        ) : (
          <SemanticResultsList results={semanticResults} />
        ))}

      {pendingCloudConfirm && (
        <CloudConfirmDialog
          query={query}
          cloudModel={settings.cloud_model}
          onCancel={() => setPendingCloudConfirm(false)}
          onConfirm={() => {
            setPendingCloudConfirm(false);
            void runTranslate(true);
          }}
        />
      )}
    </main>
  );
}
