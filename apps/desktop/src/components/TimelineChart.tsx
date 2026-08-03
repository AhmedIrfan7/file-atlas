import { formatBytes } from "../lib/format";
import type { Granularity, TimelineBucket } from "../types";

interface Props {
  buckets: TimelineBucket[];
  granularity: Granularity;
}

const HEIGHT = 180;

function formatLabel(periodStart: number, granularity: Granularity): string {
  const date = new Date(periodStart * 1000);
  if (granularity === "month") {
    return date.toLocaleDateString(undefined, { month: "short", year: "numeric" });
  }
  return date.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

export default function TimelineChart({ buckets, granularity }: Props) {
  if (buckets.length === 0) {
    return (
      <p className="text-sm text-[color:var(--color-atlas-muted)]">
        No files created in this window yet.
      </p>
    );
  }

  const maxCount = Math.max(...buckets.map((b) => b.file_count));

  return (
    <div
      className="flex items-end gap-1 overflow-x-auto rounded-lg border border-[color:var(--color-atlas-border)] p-4"
      style={{ height: HEIGHT }}
    >
      {buckets.map((bucket) => {
        const barHeight =
          maxCount > 0 ? Math.max(2, (bucket.file_count / maxCount) * (HEIGHT - 48)) : 2;
        return (
          <div
            key={bucket.period_start}
            className="flex min-w-[10px] flex-1 flex-col items-center justify-end gap-1"
            title={`${formatLabel(bucket.period_start, granularity)}: ${bucket.file_count.toLocaleString()} files, ${formatBytes(bucket.total_bytes)}`}
          >
            <div
              className="w-full rounded-t bg-[color:var(--color-atlas-accent)]"
              style={{ height: barHeight }}
            />
            <span className="text-[10px] text-[color:var(--color-atlas-muted)] whitespace-nowrap">
              {formatLabel(bucket.period_start, granularity)}
            </span>
          </div>
        );
      })}
    </div>
  );
}
