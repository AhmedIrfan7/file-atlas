const CATEGORIES = ["Image", "Video", "Audio", "Document", "Archive", "Installer", "Code", "Other"];

const TIME_WINDOWS: { label: string; days: number | null }[] = [
  { label: "All time", days: null },
  { label: "7 days", days: 7 },
  { label: "30 days", days: 30 },
  { label: "1 year", days: 365 },
];

interface Props {
  category: string | null;
  sinceDays: number | null;
  onCategoryChange: (category: string | null) => void;
  onSinceDaysChange: (days: number | null) => void;
}

export default function StorageFilters({
  category,
  sinceDays,
  onCategoryChange,
  onSinceDaysChange,
}: Props) {
  return (
    <div className="flex flex-wrap items-center gap-4 mb-4">
      <select
        value={category ?? ""}
        onChange={(e) => onCategoryChange(e.target.value || null)}
        className="rounded-lg border border-[color:var(--color-atlas-border)] bg-transparent px-3 py-1.5 text-sm text-[color:var(--color-atlas-fg)]"
      >
        <option value="">All categories</option>
        {CATEGORIES.map((cat) => (
          <option key={cat} value={cat}>
            {cat}
          </option>
        ))}
      </select>

      <div className="flex items-center gap-1 rounded-lg border border-[color:var(--color-atlas-border)] p-1">
        {TIME_WINDOWS.map((tw) => (
          <button
            key={tw.label}
            type="button"
            onClick={() => onSinceDaysChange(tw.days)}
            className={`rounded-md px-2.5 py-1 text-xs transition-colors ${
              sinceDays === tw.days
                ? "bg-white/10 text-[color:var(--color-atlas-fg)]"
                : "text-[color:var(--color-atlas-muted)] hover:text-[color:var(--color-atlas-fg)]"
            }`}
          >
            {tw.label}
          </button>
        ))}
      </div>
    </div>
  );
}
