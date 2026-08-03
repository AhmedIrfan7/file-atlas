import { formatRelativeAge } from "../lib/format";
import type { ActionRow } from "../types";

interface Props {
  actions: ActionRow[];
  onRestore: (actionId: number) => void;
}

export default function RecentActionsPanel({ actions, onRestore }: Props) {
  if (actions.length === 0) {
    return (
      <p className="text-sm text-[color:var(--color-atlas-muted)]">
        Files you delete here show up so you can undo them.
      </p>
    );
  }

  return (
    <ul className="space-y-2">
      {actions.map((action) => (
        <li
          key={action.id}
          className="flex items-center justify-between gap-2 rounded-lg border border-[color:var(--color-atlas-border)] px-3 py-2"
        >
          <div className="min-w-0 flex-1">
            <p className="text-sm truncate" title={action.path_from ?? undefined}>
              {action.path_from}
            </p>
            <p className="text-xs text-[color:var(--color-atlas-muted)]">
              {formatRelativeAge(action.ts)}
            </p>
          </div>
          <button
            type="button"
            onClick={() => onRestore(action.id)}
            className="shrink-0 text-xs text-[color:var(--color-atlas-accent)] hover:underline"
          >
            Restore
          </button>
        </li>
      ))}
    </ul>
  );
}
