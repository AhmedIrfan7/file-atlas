/**
 * A squarified treemap layout (Bruls, Huizing, van Wijk 1999): given items
 * with a value and a container size, produce rectangles whose areas are
 * proportional to value, keeping aspect ratios close to square so small
 * items stay legible instead of degenerating into slivers.
 *
 * Pure layout math, no rendering. Zero or negative values are dropped before
 * layout since they would produce zero-area or inverted rectangles.
 */

export interface TreemapInput {
  key: string;
  value: number;
}

export interface TreemapRect extends TreemapInput {
  x: number;
  y: number;
  width: number;
  height: number;
}

interface Bounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export function squarify(items: TreemapInput[], width: number, height: number): TreemapRect[] {
  const positive = items.filter((i) => i.value > 0);
  if (positive.length === 0 || width <= 0 || height <= 0) return [];

  const total = positive.reduce((sum, i) => sum + i.value, 0);
  const area = width * height;
  // Scale every value into "area units" so row-fitting math (which compares
  // areas, not raw values) is unit-independent of the caller's value scale.
  const scaled = positive
    .map((i) => ({ key: i.key, value: (i.value / total) * area }))
    .sort((a, b) => b.value - a.value);

  return layout(scaled, { x: 0, y: 0, width, height });
}

function layout(items: TreemapInput[], bounds: Bounds): TreemapRect[] {
  if (items.length === 0) return [];

  const shortSide = Math.min(bounds.width, bounds.height);
  let row: TreemapInput[] = [items[0]];
  let rest = items.slice(1);

  while (rest.length > 0 && worst(row, shortSide) >= worst([...row, rest[0]], shortSide)) {
    row = [...row, rest[0]];
    rest = rest.slice(1);
  }

  const rowSum = row.reduce((s, i) => s + i.value, 0);
  const { rects, remaining } = placeRow(row, rowSum, bounds, shortSide);
  return [...rects, ...layout(rest, remaining)];
}

/** Worst (least square-like) aspect ratio if `row` were laid out along `length`. */
function worst(row: TreemapInput[], length: number): number {
  if (row.length === 0) return Infinity;
  const sum = row.reduce((s, i) => s + i.value, 0);
  if (sum <= 0 || length <= 0) return Infinity;
  const max = Math.max(...row.map((i) => i.value));
  const min = Math.min(...row.map((i) => i.value));
  const lengthSq = length * length;
  const sumSq = sum * sum;
  return Math.max((lengthSq * max) / sumSq, sumSq / (lengthSq * min));
}

function placeRow(
  row: TreemapInput[],
  rowSum: number,
  bounds: Bounds,
  shortSide: number,
): { rects: TreemapRect[]; remaining: Bounds } {
  // The row always spans the container's shorter side in full, and consumes
  // a `thickness` slice of the longer side. When width is the longer side
  // (a "column" of items stacked top to bottom, full height, shrinking
  // width) vs. height being the longer side (a "strip" of items side by
  // side, full width, shrinking height).
  const columnMode = bounds.width >= bounds.height;
  const thickness = shortSide > 0 ? rowSum / shortSide : 0;
  const rects: TreemapRect[] = [];
  let offset = 0;

  for (const item of row) {
    const extent = rowSum > 0 ? (item.value / rowSum) * shortSide : 0;
    if (columnMode) {
      rects.push({ ...item, x: bounds.x, y: bounds.y + offset, width: thickness, height: extent });
    } else {
      rects.push({ ...item, x: bounds.x + offset, y: bounds.y, width: extent, height: thickness });
    }
    offset += extent;
  }

  const remaining: Bounds = columnMode
    ? {
        x: bounds.x + thickness,
        y: bounds.y,
        width: Math.max(0, bounds.width - thickness),
        height: bounds.height,
      }
    : {
        x: bounds.x,
        y: bounds.y + thickness,
        width: bounds.width,
        height: Math.max(0, bounds.height - thickness),
      };

  return { rects, remaining };
}
