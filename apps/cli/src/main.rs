//! atlas-cli
//!
//! Terminal harness for the File Atlas engine. Exists to prove out the core
//! before the UI arrives, and to give power users a scripting surface.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use atlas_core::{index_run, record_volume, root_prefix, scan, ScanConfig, ScanMeta, SkipRules};
use atlas_db::{apply_migrations, open, VolumeRow};
use atlas_platform::{current, PlatformFs, Volume};
use time::OffsetDateTime;

#[derive(Debug, Parser)]
#[command(
    name = "atlas",
    about = "File Atlas engine harness (scan, index, inspect).",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Scan a directory tree and update the index.
    Scan {
        /// Root directory to walk.
        path: PathBuf,
        /// Path to the SQLite index file.
        #[arg(long, default_value = "atlas.db")]
        db: PathBuf,
        /// Ignore all skip rules; walk everything (slow).
        #[arg(long)]
        permissive: bool,
        /// Emit machine-readable JSON on completion.
        #[arg(long)]
        json: bool,
    },
    /// Print index statistics.
    Stats {
        #[arg(long, default_value = "atlas.db")]
        db: PathBuf,
    },
    /// List detected volumes (drives).
    Volumes,
    /// Substring search over indexed file names and paths.
    Search {
        /// Search text.
        query: String,
        #[arg(long, default_value = "atlas.db")]
        db: PathBuf,
        /// Maximum results.
        #[arg(long, default_value_t = 25)]
        limit: usize,
    },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Scan {
            path,
            db,
            permissive,
            json,
        } => cmd_scan(&path, &db, permissive, json),
        Command::Stats { db } => cmd_stats(&db),
        Command::Volumes => cmd_volumes(),
        Command::Search { query, db, limit } => cmd_search(&query, &db, limit),
    }
}

fn cmd_scan(
    path: &std::path::Path,
    db_path: &std::path::Path,
    permissive: bool,
    json: bool,
) -> Result<()> {
    let mut conn = open(db_path).with_context(|| format!("open db at {}", db_path.display()))?;
    apply_migrations(&mut conn).context("migrate")?;

    let volume = resolve_volume_for(path);
    let volume_id = volume.id.clone();
    record_volume(&mut conn, &to_volume_row(&volume)).context("record volume")?;

    let rules = if permissive {
        SkipRules::permissive()
    } else {
        SkipRules::default()
    };
    let cfg = ScanConfig {
        root: path.to_path_buf(),
        volume_id: volume_id.clone(),
        rules,
    };

    let (tx, rx) = crossbeam_channel::unbounded();
    let cancel = std::sync::Arc::new(AtomicBool::new(false));
    let cancel_bg = std::sync::Arc::clone(&cancel);
    let cfg_bg = cfg;
    let handle = std::thread::spawn(move || scan(&cfg_bg, &cancel_bg, &tx));

    let meta = ScanMeta::now(root_prefix(path), volume_id);
    let stats = index_run(&mut conn, rx, &meta).context("indexer")?;
    let report = handle.join().expect("scanner thread");

    if json {
        let out = serde_json::json!({
            "root": path.display().to_string(),
            "entries_persisted": stats.entries_persisted,
            "removed_marked": stats.removed_marked,
            "live_files_after": stats.live_files_after,
            "errors": stats.errors,
            "files_seen": report.files_seen,
            "bytes_seen": report.bytes_seen,
            "duration_ms": report.duration_ms,
            "cancelled": report.cancelled,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("Scan complete.");
        println!("  root:              {}", path.display());
        println!("  entries persisted: {}", stats.entries_persisted);
        println!("  removed marked:    {}", stats.removed_marked);
        println!("  live files after:  {}", stats.live_files_after);
        println!("  errors:            {}", stats.errors);
        println!("  scanner files:     {}", report.files_seen);
        println!("  bytes seen:        {}", report.bytes_seen);
        println!("  duration:          {} ms", report.duration_ms);
    }
    Ok(())
}

fn cmd_stats(db_path: &std::path::Path) -> Result<()> {
    let conn = open(db_path)?;
    let live: i64 = conn.query_row(
        "SELECT COUNT(*) FROM files WHERE removed_at IS NULL",
        [],
        |r| r.get(0),
    )?;
    let removed: i64 = conn.query_row(
        "SELECT COUNT(*) FROM files WHERE removed_at IS NOT NULL",
        [],
        |r| r.get(0),
    )?;
    let bytes: i64 = conn.query_row(
        "SELECT COALESCE(SUM(size_bytes), 0) FROM files WHERE removed_at IS NULL AND is_dir = 0",
        [],
        |r| r.get(0),
    )?;

    println!("Index at {}", db_path.display());
    println!("  live entries:    {live}");
    println!("  removed entries: {removed}");
    println!("  live bytes:      {bytes}");
    println!();
    println!("Top 10 largest files:");
    let mut stmt = conn.prepare(
        "SELECT path, size_bytes FROM files WHERE removed_at IS NULL AND is_dir = 0
         ORDER BY size_bytes DESC LIMIT 10",
    )?;
    for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))? {
        let (path, size) = row?;
        println!("  {size:>14}  {path}");
    }
    Ok(())
}

fn cmd_volumes() -> Result<()> {
    let plat = current();
    let vols = plat.list_volumes().context("list volumes")?;
    for v in vols {
        println!(
            "{:<20} {:<8} {:<10} {}",
            v.id,
            v.label.unwrap_or_else(|| "-".into()),
            v.fs_type.unwrap_or_else(|| "-".into()),
            v.mount.display()
        );
    }
    Ok(())
}

fn cmd_search(query: &str, db_path: &std::path::Path, limit: usize) -> Result<()> {
    let conn = open(db_path)?;
    let pattern = format!("%{query}%");
    let mut stmt = conn.prepare(
        "SELECT path, size_bytes FROM files
         WHERE removed_at IS NULL AND (name LIKE ?1 OR path LIKE ?1)
         ORDER BY size_bytes DESC LIMIT ?2",
    )?;
    let limit_i64 = i64::try_from(limit).unwrap_or(i64::MAX);
    let rows = stmt.query_map(rusqlite::params![pattern, limit_i64], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
    })?;
    for row in rows {
        let (path, size) = row?;
        println!("{size:>14}  {path}");
    }
    Ok(())
}

fn resolve_volume_for(path: &std::path::Path) -> Volume {
    let plat = current();
    if let Ok(vols) = plat.list_volumes() {
        let path_str = path.to_string_lossy();
        let mut best: Option<Volume> = None;
        for v in vols {
            let mount_str = v.mount.to_string_lossy();
            let trimmed = mount_str.trim_end_matches(['\\', '/']);
            if starts_with_ci(&path_str, trimmed) {
                let longer = best
                    .as_ref()
                    .is_none_or(|b| b.mount.as_os_str().len() < v.mount.as_os_str().len());
                if longer {
                    best = Some(v);
                }
            }
        }
        if let Some(v) = best {
            return v;
        }
    }
    Volume {
        id: "vol:unknown".to_string(),
        label: None,
        fs_type: None,
        mount: path.to_path_buf(),
        total_bytes: None,
        free_bytes: None,
    }
}

fn to_volume_row(v: &Volume) -> VolumeRow {
    let now = OffsetDateTime::now_utc().unix_timestamp();
    VolumeRow {
        id: v.id.clone(),
        label: v.label.clone(),
        fs_type: v.fs_type.clone(),
        mount: v.mount.to_string_lossy().into_owned(),
        total_bytes: v.total_bytes.and_then(|n| i64::try_from(n).ok()),
        first_seen: now,
        last_seen: now,
    }
}

fn starts_with_ci(haystack: &str, needle: &str) -> bool {
    if haystack.len() < needle.len() {
        return false;
    }
    haystack
        .chars()
        .zip(needle.chars())
        .all(|(a, b)| a.eq_ignore_ascii_case(&b))
}
