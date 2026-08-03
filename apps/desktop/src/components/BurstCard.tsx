import { fileNameFromPath, formatBytes } from "../lib/format";
import type { Burst } from "../types";

interface Props {
  burst: Burst;
}

function title(burst: Burst): string {
  const date = new Date(burst.period_start * 1000).toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
  if (burst.kind === "screenshot_burst") {
    return `${burst.file_count.toLocaleString()} screenshots on ${date}`;
  }
  return `${burst.file_count.toLocaleString()} files added to ${fileNameFromPath(burst.folder ?? "")} on ${date}`;
}

export default function BurstCard({ burst }: Props) {
  return (
    <div className="rounded-lg border border-[color:var(--color-atlas-border)] p-4">
      <p className="text-sm font-medium mb-1">{title(burst)}</p>
      <p className="text-xs text-[color:var(--color-atlas-muted)] mb-3">
        {formatBytes(burst.total_bytes)}
        {burst.folder ? ` in ${burst.folder}` : ""}
      </p>
      <ul className="space-y-1">
        {burst.sample.slice(0, 5).map((file) => (
          <li
            key={file.path}
            className="text-sm text-[color:var(--color-atlas-muted)] truncate"
            title={file.path}
          >
            {file.name}
          </li>
        ))}
      </ul>
    </div>
  );
}
