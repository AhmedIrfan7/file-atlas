import SegmentedControl from "./ui/SegmentedControl";

const CATEGORIES = ["Image", "Video", "Audio", "Document", "Archive", "Installer", "Code", "Other"];

const TIME_WINDOWS: { key: string; label: string; days: number | null }[] = [
  { key: "all", label: "All time", days: null },
  { key: "7d", label: "Last 7 days", days: 7 },
  { key: "30d", label: "Last 30 days", days: 30 },
  { key: "1y", label: "Last year", days: 365 },
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

      <span className="text-xs text-[color:var(--color-atlas-muted)]">Changed within:</span>
      <SegmentedControl
        options={TIME_WINDOWS}
        activeKey={TIME_WINDOWS.find((tw) => tw.days === sinceDays)?.key ?? "all"}
        onSelect={(key) =>
          onSinceDaysChange(TIME_WINDOWS.find((tw) => tw.key === key)?.days ?? null)
        }
      />
    </div>
  );
}
