import { describe, expect, it } from "vitest";

import { squarify } from "./treemap";

describe("squarify", () => {
  it("returns one rectangle filling the whole container for a single item", () => {
    const rects = squarify([{ key: "a", value: 100 }], 200, 100);
    expect(rects).toHaveLength(1);
    expect(rects[0]).toMatchObject({ x: 0, y: 0, width: 200, height: 100 });
  });

  it("splits area proportionally to value for two equal items", () => {
    const rects = squarify(
      [
        { key: "a", value: 50 },
        { key: "b", value: 50 },
      ],
      200,
      100,
    );
    expect(rects).toHaveLength(2);
    const areaA = rects[0].width * rects[0].height;
    const areaB = rects[1].width * rects[1].height;
    expect(areaA).toBeCloseTo(areaB, 5);
  });

  it("conserves total area across any number of items", () => {
    const items = [
      { key: "a", value: 500 },
      { key: "b", value: 300 },
      { key: "c", value: 150 },
      { key: "d", value: 40 },
      { key: "e", value: 10 },
    ];
    const rects = squarify(items, 400, 300);
    const totalArea = rects.reduce((sum, r) => sum + r.width * r.height, 0);
    expect(totalArea).toBeCloseTo(400 * 300, 3);
  });

  it("never produces negative or NaN dimensions", () => {
    const items = Array.from({ length: 12 }, (_, i) => ({
      key: `item-${i}`,
      value: (i + 1) * 7,
    }));
    const rects = squarify(items, 500, 250);
    for (const r of rects) {
      expect(r.width).toBeGreaterThanOrEqual(0);
      expect(r.height).toBeGreaterThanOrEqual(0);
      expect(Number.isNaN(r.width)).toBe(false);
      expect(Number.isNaN(r.height)).toBe(false);
      expect(r.x).toBeGreaterThanOrEqual(0);
      expect(r.y).toBeGreaterThanOrEqual(0);
    }
  });

  it("keeps every rectangle within the container bounds", () => {
    const items = [
      { key: "a", value: 900 },
      { key: "b", value: 50 },
      { key: "c", value: 50 },
    ];
    const rects = squarify(items, 300, 200);
    for (const r of rects) {
      expect(r.x + r.width).toBeLessThanOrEqual(300 + 1e-6);
      expect(r.y + r.height).toBeLessThanOrEqual(200 + 1e-6);
    }
  });

  it("drops zero and negative values instead of producing degenerate rectangles", () => {
    const rects = squarify(
      [
        { key: "a", value: 100 },
        { key: "b", value: 0 },
        { key: "c", value: -5 },
      ],
      200,
      100,
    );
    expect(rects).toHaveLength(1);
    expect(rects[0].key).toBe("a");
  });

  it("returns an empty layout for an empty input or zero-sized container", () => {
    expect(squarify([], 200, 100)).toEqual([]);
    expect(squarify([{ key: "a", value: 10 }], 0, 100)).toEqual([]);
  });
});
