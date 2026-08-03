/**
 * Typed wrappers around `invoke()` for every Tauri command. Nothing here
 * does more than call through; keep business logic in Rust and rendering
 * logic in components.
 */

import { invoke } from "@tauri-apps/api/core";

import type {
  ActionRow,
  DuplicateGroup,
  FileSummary,
  HomeSummary,
  Recommendation,
  RestoreOutcome,
  SavedSearch,
  SearchHit,
  StaleBucket,
  StorageMapResponse,
  SuggestedRoot,
  TrashOutcome,
} from "../types";

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

export function searchFiles(queryText: string, limit: number): Promise<SearchHit[]> {
  return invoke("search_files", { queryText, limit });
}

export function saveSearch(name: string, queryText: string): Promise<number> {
  return invoke("save_search", { name, queryText });
}

export function listSavedSearches(): Promise<SavedSearch[]> {
  return invoke("list_saved_searches");
}

export function deleteSavedSearch(id: number): Promise<void> {
  return invoke("delete_saved_search", { id });
}

export function hashDuplicates(): Promise<void> {
  return invoke("hash_duplicates");
}

export function cancelHash(): Promise<void> {
  return invoke("cancel_hash");
}

export function getDuplicateGroups(limit: number): Promise<DuplicateGroup[]> {
  return invoke("get_duplicate_groups", { limit });
}

export function trashSelectedPaths(paths: string[]): Promise<TrashOutcome[]> {
  return invoke("trash_selected_paths", { paths });
}

export function restoreTrashAction(actionId: number): Promise<RestoreOutcome> {
  return invoke("restore_trash_action", { actionId });
}

export function listRecentActions(limit: number): Promise<ActionRow[]> {
  return invoke("list_recent_actions", { limit });
}

export function getCleanupRecommendations(): Promise<Recommendation[]> {
  return invoke("get_cleanup_recommendations");
}

export function getStorageMapView(
  path: string | null,
  category: string | null,
  sinceDays: number | null,
): Promise<StorageMapResponse> {
  return invoke("get_storage_map_view", { path, category, sinceDays });
}
