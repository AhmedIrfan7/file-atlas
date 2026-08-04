import { useEffect, useRef, useState } from "react";

import { formatBytes } from "../lib/format";
import { squarify, type TreemapRect } from "../lib/treemap";
import type { StorageNode } from "../types";

const HEIGHT = 480;
const MIN_LABEL_WIDTH = 56;
const MIN_LABEL_HEIGHT = 32;
const TOOLTIP_WIDTH = 200;

interface Props {
  nodes: StorageNode[];
  onDrillInto: (node: StorageNode) => void;
}

/** How vivid a folder's tint gets, scaled by its share of the largest node in view. */
function colorMixPercent(node: StorageNode, maxSize: number): number {
  const ratio = maxSize > 0 ? node.size_bytes / maxSize : 0;
  // sqrt so mid-sized folders are still visually distinct from tiny ones,
  // instead of the gradient being dominated by one giant outlier.
  return 12 + Math.sqrt(ratio) * 45;
}

export default function TreemapCanvas({ nodes, onDrillInto }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  const [hoveredKey, setHoveredKey] = useState<string | null>(null);

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

  // The loose-files bucket's own `path` is always the scope folder itself
  // (see storage_map.rs::drill_down), which never collides with a real
  // child folder's path, so `path` alone is already a unique key here.
  const rects = squarify(
    nodes.map((n) => ({ key: n.path, value: n.size_bytes })),
    width,
    HEIGHT,
  );
  const maxSize = Math.max(1, ...nodes.map((n) => n.size_bytes));
  const hovered = rects.find((r) => r.key === hoveredKey);
  const hoveredNode = hovered && nodes.find((n) => n.path === hovered.key);

  return (
    <div>
      <div ref={containerRef} className="relative w-full" style={{ height: HEIGHT }}>
        <div className="absolute inset-0 rounded-lg border border-[color:var(--color-atlas-border)] overflow-hidden">
          {rects.map((rect) => {
            const node = nodes.find((n) => n.path === rect.key);
            if (!node) return null;
            const showLabel = rect.width >= MIN_LABEL_WIDTH && rect.height >= MIN_LABEL_HEIGHT;
            const background = node.is_dir
              ? `color-mix(in srgb, var(--color-atlas-accent) ${colorMixPercent(node, maxSize)}%, var(--color-atlas-bg))`
              : "color-mix(in srgb, var(--color-atlas-fg) 10%, var(--color-atlas-bg))";
            return (
              <button
                key={rect.key}
                type="button"
                // Not `disabled`: a disabled button suppresses hover/focus
                // events in most browsers, which would silently break the
                // tooltip for the one node type (the loose-files bucket)
                // that most needs it, since it never shows an inline label.
                // onClick already no-ops for it below.
                onClick={() => node.is_dir && onDrillInto(node)}
                onMouseEnter={() => setHoveredKey(rect.key)}
                onMouseLeave={() => setHoveredKey(null)}
                onFocus={() => setHoveredKey(rect.key)}
                onBlur={() => setHoveredKey(null)}
                className={`absolute flex flex-col items-start justify-start overflow-hidden border p-1.5 text-left transition-colors ${
                  node.is_dir
                    ? "border-[color:var(--color-atlas-border)] hover:border-[color:var(--color-atlas-accent)] cursor-pointer"
                    : "border-[color:var(--color-atlas-border)] cursor-default"
                }`}
                style={{
                  left: rect.x,
                  top: rect.y,
                  width: Math.max(rect.width - 2, 0),
                  height: Math.max(rect.height - 2, 0),
                  background,
                }}
              >
                {showLabel && (
                  <>
                    <span className="text-xs font-medium truncate w-full">
                      {node.is_dir ? node.name : "Other files here"}
                    </span>
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

        {hovered && hoveredNode && (
          <div
            className="absolute z-10 rounded-md border border-[color:var(--color-atlas-border)] bg-[color:var(--color-atlas-bg)] px-3 py-2 shadow-lg pointer-events-none"
            style={{
              width: TOOLTIP_WIDTH,
              left: clampTooltipLeft(hovered, width),
              top: hovered.y > 48 ? hovered.y - 8 : hovered.y + hovered.height + 8,
              transform: hovered.y > 48 ? "translateY(-100%)" : undefined,
            }}
          >
            <p className="text-sm font-medium truncate">
              {hoveredNode.is_dir ? hoveredNode.name : "Other files here"}
            </p>
            <p className="text-xs text-[color:var(--color-atlas-muted)]">
              {formatBytes(hoveredNode.size_bytes)}
              {hoveredNode.is_dir ? " · click to open" : ""}
            </p>
          </div>
        )}
      </div>

      <p className="mt-3 text-xs text-[color:var(--color-atlas-muted)]">
        Box size and shade both scale with folder size, largest and darkest first. Click a folder to
        drill in.
      </p>
    </div>
  );
}

function clampTooltipLeft(rect: TreemapRect, containerWidth: number): number {
  const raw = rect.x + rect.width / 2 - TOOLTIP_WIDTH / 2;
  return Math.min(Math.max(raw, 4), Math.max(4, containerWidth - TOOLTIP_WIDTH - 4));
}
