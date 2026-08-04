import { buildSearchIndex, cancelSearchIndex } from "../lib/atlas";
import { useAiStore } from "../store/aiStore";

export default function AiStatusBanner() {
  const status = useAiStore((s) => s.status);
  const embedProgress = useAiStore((s) => s.embedProgress);

  if (!status) {
    return null;
  }

  if (!status.ollama_available) {
    return (
      <div className="rounded-lg border border-red-500/30 bg-red-500/10 px-4 py-3 text-sm text-red-300 mb-6">
        Ollama is not running. Start it locally to enable natural-language search and semantic
        search. Nothing here ever leaves your machine unless you turn on cloud AI below.
      </div>
    );
  }

  const building = embedProgress !== null;

  return (
    <div className="rounded-lg border border-[color:var(--color-atlas-border)] px-4 py-3 mb-6 text-sm">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="text-[color:var(--color-atlas-muted)]">
          <span className="text-[color:var(--color-atlas-fg)]">Ollama connected.</span>{" "}
          {status.embedding_model_installed ? (
            <>Search index: {status.files_embedded.toLocaleString()} files embedded</>
          ) : (
            <span className="text-amber-400">
              Embedding model &quot;{status.embedding_model}&quot; is not installed.
            </span>
          )}
          {status.files_pending > 0 && status.embedding_model_installed && (
            <>, {status.files_pending.toLocaleString()} pending</>
          )}
          {status.chat_model === null && (
            <span className="block text-amber-400 mt-1">
              No chat model configured, so natural-language translation will fall back to plain
              free-text search.
            </span>
          )}
        </div>
        {status.embedding_model_installed && status.files_pending > 0 && (
          <button
            type="button"
            onClick={() => void buildSearchIndex()}
            disabled={building}
            className="shrink-0 rounded-lg border border-[color:var(--color-atlas-border)] px-3 py-1.5 text-xs text-[color:var(--color-atlas-fg)] hover:border-[color:var(--color-atlas-accent)] disabled:opacity-40 transition-colors"
          >
            {building ? "Building..." : "Build search index"}
          </button>
        )}
      </div>
      {building && embedProgress && (
        <div className="mt-3">
          <div className="h-1.5 w-full rounded-full bg-white/10 overflow-hidden">
            <div
              className="h-full bg-[color:var(--color-atlas-accent)] transition-all"
              style={{
                width:
                  embedProgress.filesTotal > 0
                    ? `${Math.min(100, (embedProgress.filesEmbedded / embedProgress.filesTotal) * 100)}%`
                    : "0%",
              }}
            />
          </div>
          <div className="flex items-center justify-between mt-1 text-xs text-[color:var(--color-atlas-muted)]">
            <span>
              {embedProgress.filesEmbedded.toLocaleString()} /{" "}
              {embedProgress.filesTotal.toLocaleString()}
            </span>
            <button
              type="button"
              onClick={() => void cancelSearchIndex()}
              className="hover:text-[color:var(--color-atlas-fg)]"
            >
              Cancel
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
