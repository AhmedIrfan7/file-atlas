import type { SavedSearch } from "../types";

interface Props {
  savedSearches: SavedSearch[];
  onRun: (queryText: string) => void;
  onDelete: (id: number) => void;
}

export default function SavedSearchesPanel({ savedSearches, onRun, onDelete }: Props) {
  if (savedSearches.length === 0) {
    return (
      <p className="text-sm text-[color:var(--color-atlas-muted)]">
        Save a search to find it again quickly.
      </p>
    );
  }

  return (
    <ul className="space-y-2">
      {savedSearches.map((saved) => (
        <li
          key={saved.id}
          className="flex items-center justify-between gap-2 rounded-lg border border-[color:var(--color-atlas-border)] px-3 py-2"
        >
          <button
            type="button"
            onClick={() => onRun(saved.query_text)}
            className="min-w-0 flex-1 text-left"
          >
            <p className="text-sm font-medium truncate">{saved.name}</p>
            <p className="text-xs text-[color:var(--color-atlas-muted)] truncate">
              {saved.query_text}
            </p>
          </button>
          <button
            type="button"
            onClick={() => onDelete(saved.id)}
            className="shrink-0 text-xs text-[color:var(--color-atlas-muted)] hover:text-red-400 transition-colors"
            aria-label={`Delete saved search ${saved.name}`}
          >
            Delete
          </button>
        </li>
      ))}
    </ul>
  );
}
