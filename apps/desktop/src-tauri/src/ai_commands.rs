//! Tauri command handlers for the optional local-first AI layer (M9):
//! semantic search over file names, natural-language query translation, and
//! AI settings. Same thin-adapter contract as the rest of the command
//! modules.
//!
//! Cloud calls only ever happen when `translate_natural_language_query` is
//! called with `use_cloud: true`, and that is re-checked against settings
//! here rather than trusted blindly: the UI's own per-request confirmation
//! dialog is what makes `use_cloud: true` meaningful, not this command
//! alone, so both layers agreeing is the actual guarantee.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use atlas_ai::{
    build_embedding_index, get_settings, index_status, rank_by_similarity, set_settings, translate,
    AiSettings, CloudProvider, EmbedProgress, EmbedStats, EmbeddingProvider, OllamaProvider,
    SimilarFile, TranslatedQuery,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use time::OffsetDateTime;

use crate::state::AppState;

const EMBEDDING_MODEL: &str = "nomic-embed-text";

#[derive(Debug, Clone, Serialize)]
pub struct EmbedProgressEvent {
    pub files_embedded: u64,
    pub files_total: u64,
}

/// Local AI availability plus embedding-index progress, for the AI search
/// settings screen. Four independent yes/no facts about the AI layer, not a
/// combined state machine, so they stay separate bools rather than an enum.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Serialize)]
pub struct AiStatus {
    pub ollama_available: bool,
    pub installed_models: Vec<String>,
    pub embedding_model: String,
    pub embedding_model_installed: bool,
    pub chat_model: Option<String>,
    pub chat_model_installed: bool,
    pub files_embedded: i64,
    pub files_pending: i64,
    pub cloud_enabled: bool,
}

fn ollama_provider(chat_model: Option<String>) -> OllamaProvider {
    OllamaProvider::local_default(chat_model)
}

#[tauri::command]
pub fn get_ai_status(state: State<'_, AppState>) -> Result<AiStatus, String> {
    // The database mutex is only ever held for the two plain SQL reads
    // below, never across the network calls in between: holding it across
    // an HTTP request to Ollama would freeze every other DB-touching Tauri
    // command for the duration of that request.
    let settings = {
        let conn = state.db.lock();
        get_settings(&conn).map_err(|e| e.to_string())?
    };

    let provider = ollama_provider(settings.chat_model.clone());
    let ollama_available = provider.is_available();
    let installed_models = if ollama_available {
        provider.installed_models().unwrap_or_default()
    } else {
        Vec::new()
    };

    let (files_embedded, files_pending) = {
        let conn = state.db.lock();
        index_status(&conn, EMBEDDING_MODEL).map_err(|e| e.to_string())?
    };

    Ok(AiStatus {
        embedding_model_installed: installed_models
            .iter()
            .any(|m| m.starts_with(EMBEDDING_MODEL)),
        chat_model_installed: settings
            .chat_model
            .as_ref()
            .is_some_and(|m| installed_models.iter().any(|installed| installed == m)),
        chat_model: settings.chat_model,
        cloud_enabled: settings.cloud_enabled,
        ollama_available,
        installed_models,
        embedding_model: EMBEDDING_MODEL.to_string(),
        files_embedded,
        files_pending,
    })
}

#[tauri::command]
pub fn get_ai_settings(state: State<'_, AppState>) -> Result<AiSettings, String> {
    let conn = state.db.lock();
    get_settings(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_ai_settings(settings: AiSettings, state: State<'_, AppState>) -> Result<(), String> {
    let conn = state.db.lock();
    set_settings(&conn, &settings).map_err(|e| e.to_string())
}

/// Start (or resume) building the local semantic-search index: embeds every
/// live file with no embedding yet under the current embedding model.
/// Returns immediately; progress arrives via `embed-progress` and
/// `embed-finished` events, matching `start_scan`/`hash_duplicates`.
#[tauri::command]
pub fn build_search_index(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    if state.embed_running.swap(true, Ordering::SeqCst) {
        return Err("A search-index build is already running.".into());
    }
    state.embed_cancel.store(false, Ordering::SeqCst);

    let cancel = Arc::clone(&state.embed_cancel);
    let running = Arc::clone(&state.embed_running);
    let app_for_thread = app.clone();

    std::thread::spawn(move || {
        let state = app_for_thread.state::<AppState>();
        let conn = state.db.lock();
        let settings = get_settings(&conn).unwrap_or_default();
        let provider = ollama_provider(settings.chat_model);
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let app_for_events = app_for_thread.clone();
        let result = build_embedding_index(
            &conn,
            &provider,
            now,
            cancel.as_ref(),
            move |progress: EmbedProgress| {
                let _ = app_for_events.emit(
                    "embed-progress",
                    EmbedProgressEvent {
                        files_embedded: progress.files_embedded,
                        files_total: progress.files_total,
                    },
                );
            },
        );
        drop(conn);
        running.store(false, Ordering::SeqCst);
        let embed_stats = result.unwrap_or(EmbedStats {
            files_embedded: 0,
            errors: 0,
        });
        let _ = app_for_thread.emit("embed-finished", embed_stats);
    });

    Ok(())
}

/// Cancel an in-progress search-index build. No-op if none is running.
#[tauri::command]
pub fn cancel_search_index(state: State<'_, AppState>) {
    state.embed_cancel.store(true, Ordering::SeqCst);
}

/// Semantic search over file names: the query and every result are compared
/// by embedding similarity, not the filter DSL search uses.
#[tauri::command]
pub fn semantic_search_files(
    query: String,
    limit: u32,
    state: State<'_, AppState>,
) -> Result<Vec<SimilarFile>, String> {
    let chat_model = {
        let conn = state.db.lock();
        get_settings(&conn).map_err(|e| e.to_string())?.chat_model
    };

    // Embed the query (a network call) before taking the connection lock,
    // same reasoning as get_ai_status: the shared connection should never
    // sit locked for the duration of a request to the embedding provider.
    let provider = ollama_provider(chat_model);
    let query_vector = provider.embed(&query).map_err(|e| e.to_string())?;

    let conn = state.db.lock();
    rank_by_similarity(&conn, &query_vector, provider.model_name(), limit)
        .map_err(|e| e.to_string())
}

/// Translate `query` into the filter-DSL grammar `search_files` already
/// understands. `use_cloud` requires cloud AI to already be enabled in
/// settings (see module docs for why that is re-checked here).
#[tauri::command]
pub fn translate_natural_language_query(
    query: String,
    use_cloud: bool,
    state: State<'_, AppState>,
) -> Result<TranslatedQuery, String> {
    let conn = state.db.lock();
    let settings = get_settings(&conn).map_err(|e| e.to_string())?;
    drop(conn);

    if use_cloud {
        if !settings.cloud_enabled {
            return Err("Cloud AI is not enabled in settings.".to_string());
        }
        let (Some(base_url), Some(api_key), Some(model)) = (
            settings.cloud_base_url,
            settings.cloud_api_key,
            settings.cloud_model,
        ) else {
            return Err("Cloud AI is enabled but not fully configured.".to_string());
        };
        let provider = CloudProvider::new(base_url, api_key, model);
        Ok(translate(&provider, &query))
    } else {
        let provider = ollama_provider(settings.chat_model);
        Ok(translate(&provider, &query))
    }
}
