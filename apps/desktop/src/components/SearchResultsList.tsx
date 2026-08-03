import { openInFileManager } from "../lib/atlas";
import { formatBytes, formatRelativeAge } from "../lib/format";
import type { SearchHit } from "../types";

interface Props {
  results: SearchHit[];
  loading: boolean;
  hasQuery: boolean;
}

export default function SearchResultsList({ results, loading, hasQuery }: Props) {
  if (loading) {
    return <p className="text-sm text-[color:var(--color-atlas-muted)]">Searching...</p>;
  }

  if (!hasQuery) {
    return (
      <p className="text-sm text-[color:var(--color-atlas-muted)]">
        Type a name, or combine filters like <code>type:pdf</code>, <code>size&gt;10mb</code>,{" "}
        <code>age&lt;1y</code>, <code>in:downloads</code>.
      </p>
    );
  }

  if (results.length === 0) {
    return <p className="text-sm text-[color:var(--color-atlas-muted)]">No matches.</p>;
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
              <p>{hit.is_dir ? "Folder" : formatBytes(hit.size_bytes)}</p>
              <p>{formatRelativeAge(hit.modified_at)}</p>
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
