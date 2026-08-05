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
    <ul className="divide-y divide-[color:var(--color-atlas-border)]">
      {actions.map((action) => (
        <li
          key={action.id}
          className="flex items-center justify-between gap-2 rounded-md px-2 py-2 -mx-2 hover:bg-[color:var(--color-atlas-surface-hover)] transition-colors"
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
