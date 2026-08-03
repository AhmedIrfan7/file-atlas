import { formatBytes, formatRelativeAge } from "../lib/format";
import { keepPathFor } from "../store/duplicatesStore";
import type { DuplicateGroup } from "../types";

interface Props {
  group: DuplicateGroup;
  keepOverride: string | undefined;
  onChangeKeep: (path: string) => void;
}

export default function DuplicateGroupCard({ group, keepOverride, onChangeKeep }: Props) {
  const keepPath = keepPathFor(group, keepOverride ? { [group.hash]: keepOverride } : {});

  return (
    <div className="rounded-lg border border-[color:var(--color-atlas-border)] p-4">
      <div className="flex items-baseline justify-between mb-3">
        <p className="text-sm">
          {group.members.length} copies of {formatBytes(group.size_bytes)}
        </p>
        <p className="text-sm text-[color:var(--color-atlas-accent)]">
          {formatBytes(group.wasted_bytes)} wasted
        </p>
      </div>
      <p className="text-xs text-[color:var(--color-atlas-muted)] mb-3">{group.keep_reason}</p>
      <ul className="space-y-1">
        {group.members.map((member) => {
          const isKeep = member.file.path === keepPath;
          return (
            <li
              key={member.file.path}
              className={`flex items-center gap-3 rounded-md px-2 py-1.5 ${
                isKeep ? "bg-[color:var(--color-atlas-accent)]/10" : ""
              }`}
            >
              <input
                type="radio"
                name={`keep-${group.hash}`}
                checked={isKeep}
                onChange={() => onChangeKeep(member.file.path)}
                className="accent-[color:var(--color-atlas-accent)]"
                aria-label={`Keep ${member.file.name}`}
              />
              <div className="min-w-0 flex-1">
                <p className="text-sm truncate" title={member.file.path}>
                  {member.file.name}
                </p>
                <p className="text-xs text-[color:var(--color-atlas-muted)] truncate">
                  {member.file.path}
                </p>
              </div>
              <div className="shrink-0 text-right text-xs text-[color:var(--color-atlas-muted)]">
                <p>{formatRelativeAge(member.file.modified_at)}</p>
                {isKeep && (
                  <p className="text-[color:var(--color-atlas-accent)] font-medium">Keep</p>
                )}
              </div>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
