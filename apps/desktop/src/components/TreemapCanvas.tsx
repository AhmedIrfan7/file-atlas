import { useEffect, useRef, useState } from "react";

import { formatBytes } from "../lib/format";
import { squarify } from "../lib/treemap";
import type { StorageNode } from "../types";

const HEIGHT = 480;
const MIN_LABEL_WIDTH = 56;
const MIN_LABEL_HEIGHT = 32;

interface Props {
  nodes: StorageNode[];
  onDrillInto: (node: StorageNode) => void;
}

export default function TreemapCanvas({ nodes, onDrillInto }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (entry) setWidth(entry.contentRect.width);
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const rects = squarify(
    nodes.map((n) => ({ key: n.path, value: n.size_bytes })),
    width,
    HEIGHT,
  );

  return (
    <div
      ref={containerRef}
      className="relative w-full rounded-lg border border-[color:var(--color-atlas-border)] overflow-hidden"
      style={{ height: HEIGHT }}
    >
      {rects.map((rect) => {
        const node = nodes.find((n) => n.path === rect.key);
        if (!node) return null;
        const showLabel = rect.width >= MIN_LABEL_WIDTH && rect.height >= MIN_LABEL_HEIGHT;
        return (
          <button
            key={rect.key}
            type="button"
            disabled={!node.is_dir}
            onClick={() => node.is_dir && onDrillInto(node)}
            title={`${node.name}\n${formatBytes(node.size_bytes)}`}
            className={`absolute flex flex-col items-start justify-start overflow-hidden border p-1.5 text-left transition-colors ${
              node.is_dir
                ? "border-[color:var(--color-atlas-border)] bg-[color:var(--color-atlas-accent)]/10 hover:bg-[color:var(--color-atlas-accent)]/20 cursor-pointer"
                : "border-dashed border-[color:var(--color-atlas-border)] bg-white/5 cursor-default"
            }`}
            style={{
              left: rect.x,
              top: rect.y,
              width: Math.max(rect.width - 2, 0),
              height: Math.max(rect.height - 2, 0),
            }}
          >
            {showLabel && (
              <>
                <span className="text-xs font-medium truncate w-full">{node.name}</span>
                <span className="text-xs text-[color:var(--color-atlas-muted)] truncate w-full">
                  {formatBytes(node.size_bytes)}
                </span>
              </>
            )}
          </button>
        );
      })}
      {nodes.length === 0 && (
        <p className="absolute inset-0 flex items-center justify-center text-sm text-[color:var(--color-atlas-muted)]">
          Nothing here.
        </p>
      )}
    </div>
  );
}
