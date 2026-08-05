import { formatBytes, formatRelativeAge } from "../lib/format";
import type { Confidence, Recommendation } from "../types";
import Badge from "./ui/Badge";
import Panel from "./ui/Panel";

const CONFIDENCE_TONE: Record<Confidence, "accent" | "warning" | "neutral"> = {
  High: "accent",
  Medium: "warning",
  Low: "neutral",
};

interface Props {
  recommendation: Recommendation;
  selectedPaths: Set<string>;
  onToggleItem: (path: string) => void;
  onToggleAll: (selected: boolean) => void;
}

export default function RecommendationCard({
  recommendation,
  selectedPaths,
  onToggleItem,
  onToggleAll,
}: Props) {
  const allSelected = recommendation.items.every((i) => selectedPaths.has(i.path));

  return (
    <Panel className="p-4">
      <div className="flex items-center justify-between gap-3 mb-1">
        <p className="text-sm font-medium">{recommendation.title}</p>
        <Badge tone={CONFIDENCE_TONE[recommendation.confidence]}>
          {recommendation.confidence} confidence
        </Badge>
      </div>
      <p className="text-xs text-[color:var(--color-atlas-muted)] mb-3">
        {recommendation.explanation}
      </p>
      <button
        type="button"
        onClick={() => onToggleAll(!allSelected)}
        className="text-xs text-[color:var(--color-atlas-accent)] hover:underline mb-2"
      >
        {allSelected ? "Deselect all" : "Select all"}
      </button>
      <ul className="space-y-1">
        {recommendation.items.map((item) => (
          <li
            key={item.path}
            className="flex items-center gap-3 rounded-md px-2 py-1.5 hover:bg-[color:var(--color-atlas-surface-hover)]"
          >
            <input
              type="checkbox"
              checked={selectedPaths.has(item.path)}
              onChange={() => onToggleItem(item.path)}
              className="accent-[color:var(--color-atlas-accent)]"
              aria-label={`Select ${item.name}`}
            />
            <div className="min-w-0 flex-1">
              <p className="text-sm truncate" title={item.path}>
                {item.name}
              </p>
              <p className="text-xs text-[color:var(--color-atlas-muted)] truncate">{item.path}</p>
            </div>
            <div className="shrink-0 text-right text-xs text-[color:var(--color-atlas-muted)]">
              {item.size_bytes > 0 && <p>{formatBytes(item.size_bytes)}</p>}
              <p>{formatRelativeAge(item.modified_at)}</p>
            </div>
          </li>
        ))}
      </ul>
    </Panel>
  );
}
