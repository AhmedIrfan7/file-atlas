import { formatBytes } from "../lib/format";
import type { CategoryTotal } from "../types";

const CATEGORY_COLORS: Record<string, string> = {
  Image: "#6ea8fe",
  Video: "#f5a623",
  Audio: "#bb86fc",
  Document: "#4ade80",
  Archive: "#facc15",
  Installer: "#f87171",
  Code: "#38bdf8",
  Folder: "#8a929c",
  Other: "#5c6470",
};

interface Props {
  categories: CategoryTotal[];
  totalBytes: number;
}

export default function CategoryBreakdown({ categories, totalBytes }: Props) {
  if (categories.length === 0) {
    return (
      <p className="text-sm text-[color:var(--color-atlas-muted)]">No categorized files yet.</p>
    );
  }

  return (
    <ul className="space-y-3">
      {categories.map((cat) => {
        const pct = totalBytes > 0 ? (cat.total_bytes / totalBytes) * 100 : 0;
        const color = CATEGORY_COLORS[cat.category] ?? CATEGORY_COLORS.Other;
        return (
          <li key={cat.category}>
            <div className="flex items-center justify-between text-sm mb-1">
              <span className="flex items-center gap-2">
                <span
                  className="h-2.5 w-2.5 rounded-full"
                  style={{ backgroundColor: color }}
                  aria-hidden="true"
                />
                {cat.category}
                <span className="text-[color:var(--color-atlas-muted)]">
                  ({cat.file_count.toLocaleString()})
                </span>
              </span>
              <span className="text-[color:var(--color-atlas-muted)]">
                {formatBytes(cat.total_bytes)}
              </span>
            </div>
            <div className="h-1.5 rounded-full bg-[color:var(--color-atlas-border)] overflow-hidden">
              <div
                className="h-full rounded-full"
                style={{ width: `${pct}%`, backgroundColor: color }}
              />
            </div>
          </li>
        );
      })}
    </ul>
  );
}
