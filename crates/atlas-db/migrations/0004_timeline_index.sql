-- Life timeline (M7) buckets and bursts by file creation time. Without this
-- index, grouping by created_at means a full table scan of every live file.

CREATE INDEX idx_files_created ON files(created_at);
