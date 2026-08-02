import { formatBytes } from "../lib/format";
import type { StaleBucket } from "../types";

interface Props {
  bucket: StaleBucket;
}

export default function StaleBucketCard({ bucket }: Props) {
  if (bucket.file_count === 0) {
    return (
      <p className="text-sm text-[color:var(--color-atlas-muted)]">
        Nothing has been sitting untouched for over a year. Nice.
      </p>
    );
  }

  return (
    <div>
      <p className="text-sm mb-3">
        <span className="font-semibold">{bucket.file_count.toLocaleString()} files</span>{" "}
        <span className="text-[color:var(--color-atlas-muted)]">
          ({formatBytes(bucket.total_bytes)}) have not been touched in over a year.
        </span>
      </p>
      <ul className="space-y-1">
        {bucket.sample.slice(0, 5).map((file) => (
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
