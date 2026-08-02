/**
 * Mirrors the DTOs returned by Tauri commands in
 * apps/desktop/src-tauri/src/commands.rs and the domain types in
 * crates/atlas-core. Keep these in sync by hand; there are few enough
 * fields that generating bindings would be overkill for now.
 */

export interface SuggestedRoot {
  label: string;
  path: string;
}

export interface CategoryTotal {
  category: string;
  file_count: number;
  total_bytes: number;
}

export interface HomeSummary {
  live_file_count: number;
  live_folder_count: number;
  total_bytes: number;
  categories: CategoryTotal[];
}

export interface FileSummary {
  path: string;
  name: string;
  size_bytes: number;
  modified_at: number | null;
  category: string | null;
}

export interface StaleBucket {
  min_age_days: number;
  file_count: number;
  total_bytes: number;
  sample: FileSummary[];
}

export interface ScanProgressEvent {
  root: string;
  files_seen: number;
  bytes_seen: number;
}

export interface ScanFinishedEvent {
  roots_scanned: number;
  total_entries_persisted: number;
  total_removed_marked: number;
  total_errors: number;
  cancelled: boolean;
}
