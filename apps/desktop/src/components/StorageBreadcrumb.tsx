import type { Breadcrumb } from "../store/storageMapStore";

interface Props {
  breadcrumbs: Breadcrumb[];
  onJumpTo: (index: number) => void;
}

export default function StorageBreadcrumb({ breadcrumbs, onJumpTo }: Props) {
  return (
    <nav className="flex flex-wrap items-center gap-1 text-sm mb-4">
      {breadcrumbs.map((crumb, index) => (
        <span key={crumb.path ?? "root"} className="flex items-center gap-1">
          {index > 0 && <span className="text-[color:var(--color-atlas-muted)]">/</span>}
          <button
            type="button"
            onClick={() => onJumpTo(index)}
            disabled={index === breadcrumbs.length - 1}
            className={
              index === breadcrumbs.length - 1
                ? "text-[color:var(--color-atlas-fg)] font-medium"
                : "text-[color:var(--color-atlas-muted)] hover:text-[color:var(--color-atlas-fg)]"
            }
          >
            {crumb.label}
          </button>
        </span>
      ))}
    </nav>
  );
}
