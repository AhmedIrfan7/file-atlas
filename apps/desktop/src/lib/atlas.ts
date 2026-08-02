/**
 * Typed wrappers around `invoke()` for every Tauri command. Nothing here
 * does more than call through; keep business logic in Rust and rendering
 * logic in components.
 */

import { invoke } from "@tauri-apps/api/core";

import type { FileSummary, HomeSummary, StaleBucket, SuggestedRoot } from "../types";

export function getDefaultRoots(): Promise<SuggestedRoot[]> {
  return invoke("get_default_roots");
}

export function startScan(roots: string[]): Promise<void> {
  return invoke("start_scan", { roots });
}

export function cancelScan(): Promise<void> {
  return invoke("cancel_scan");
}

export function isScanning(): Promise<boolean> {
  return invoke("is_scanning");
}

export function getHomeSummary(): Promise<HomeSummary> {
  return invoke("get_home_summary");
}

export function getTopLargest(limit: number): Promise<FileSummary[]> {
  return invoke("get_top_largest", { limit });
}

export function getTopOldest(limit: number): Promise<FileSummary[]> {
  return invoke("get_top_oldest", { limit });
}

export function getStaleBucket(minAgeDays: number, sampleLimit: number): Promise<StaleBucket> {
  return invoke("get_stale_bucket", {
    minAgeDays,
    sampleLimit,
  });
}
