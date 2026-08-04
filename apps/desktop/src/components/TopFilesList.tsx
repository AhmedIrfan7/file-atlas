import { formatBytes, formatRelativeAge } from "../lib/format";
import type { FileSummary } from "../types";

interface Props {
  title: string;
  files: FileSummary[];
  valueMode: "size" | "age";
  emptyLabel: string;
}

export default function TopFilesList({ title, files, valueMode, emptyLabel }: Props) {
  return (
    <div>
      <h2 className="text-sm font-medium text-[color:var(--color-atlas-muted)] mb-3">{title}</h2>
      {files.length === 0 ? (
        <p className="text-sm text-[color:var(--color-atlas-muted)]">{emptyLabel}</p>
      ) : (
        <ul className="space-y-2">
          {files.map((file) => (
            <li key={file.path} className="flex items-center justify-between gap-4">
              <span className="min-w-0">
                <span className="block text-sm truncate">{file.name}</span>
                <span className="block text-xs text-[color:var(--color-atlas-muted)] truncate">
                  {file.path}
                </span>
              </span>
              <span className="shrink-0 text-sm text-[color:var(--color-atlas-muted)] tabular-nums">
                {valueMode === "size"
                  ? formatBytes(file.size_bytes)
                  : formatRelativeAge(file.modified_at)}
              </span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
