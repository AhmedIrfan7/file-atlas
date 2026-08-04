import { openInFileManager } from "../lib/atlas";
import { formatBytes, formatRelativeAge } from "../lib/format";
import type { SimilarFile } from "../types";

interface Props {
  results: SimilarFile[];
  hasIndex: boolean;
}

export default function SemanticResultsList({ results, hasIndex }: Props) {
  if (results.length === 0) {
    return (
      <p className="text-sm text-[color:var(--color-atlas-muted)]">
        {hasIndex
          ? "No matches. Try describing it differently."
          : "No matches yet. Build the search index above, then try a description of what you're looking for."}
      </p>
    );
  }

  return (
    <ul className="space-y-1">
      {results.map((hit) => (
        <li
          key={hit.path}
          className="flex items-center justify-between gap-4 rounded-lg px-3 py-2 hover:bg-white/5 transition-colors"
          title={hit.path}
        >
          <div className="min-w-0">
            <p className="text-sm truncate">{hit.name}</p>
            <p className="text-xs text-[color:var(--color-atlas-muted)] truncate">{hit.path}</p>
          </div>
          <div className="shrink-0 flex items-center gap-4">
            <div className="text-right text-xs text-[color:var(--color-atlas-muted)] tabular-nums">
              <p>{formatBytes(hit.size_bytes)}</p>
              <p>{formatRelativeAge(hit.modified_at)}</p>
              <p title="Similarity score">{(hit.score * 100).toFixed(0)}% match</p>
            </div>
            <button
              type="button"
              onClick={() => void openInFileManager(hit.path).catch(console.error)}
              className="text-xs text-[color:var(--color-atlas-accent)] hover:underline"
            >
              Show in folder
            </button>
          </div>
        </li>
      ))}
    </ul>
  );
}
