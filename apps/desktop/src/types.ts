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

export interface SearchHit {
  path: string;
  name: string;
  size_bytes: number;
  modified_at: number | null;
  category: string | null;
  is_dir: boolean;
}

export interface SavedSearch {
  id: number;
  name: string;
  query_text: string;
  created_at: number;
}

export interface DuplicateMember {
  file: FileSummary;
  suggested_keep: boolean;
}

export interface DuplicateGroup {
  hash: string;
  size_bytes: number;
  wasted_bytes: number;
  keep_reason: string;
  members: DuplicateMember[];
}

export interface TrashOutcome {
  path: string;
  ok: boolean;
  reason: string | null;
  action_id: number | null;
}

export interface RestoreOutcome {
  action_id: number;
  ok: boolean;
  reason: string | null;
  restored_path: string | null;
}

export interface ActionRow {
  id: number;
  ts: number;
  op: string;
  path_from: string | null;
  path_to: string | null;
  metadata: string | null;
  reversible: boolean;
  undo_ref: string | null;
}

export type Confidence = "High" | "Medium" | "Low";

export interface RecommendationItem {
  path: string;
  name: string;
  size_bytes: number;
  modified_at: number | null;
}

export interface Recommendation {
  kind: string;
  title: string;
  explanation: string;
  confidence: Confidence;
  total_bytes: number;
  items: RecommendationItem[];
}

export interface StorageNode {
  path: string;
  name: string;
  is_dir: boolean;
  size_bytes: number;
}

export interface StorageMapResponse {
  scope_path: string | null;
  total_bytes: number;
  nodes: StorageNode[];
}

export interface HashProgressEvent {
  files_hashed: number;
  files_total: number;
}

export interface HashFinishedEvent {
  files_hashed: number;
  errors: number;
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
