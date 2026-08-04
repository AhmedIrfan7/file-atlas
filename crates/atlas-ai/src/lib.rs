//! atlas-ai
//!
//! The optional local-first AI layer (M9): local embeddings and semantic
//! search over file names via Ollama, natural-language search translated
//! into `atlas_search`'s existing filter DSL rather than raw SQL, and an
//! opt-in cloud fallback that never sends anything but the user's own typed
//! query text. Nothing in this crate runs unless a caller explicitly builds
//! a provider and calls into it: there is no background service, no
//! always-on network activity, and no data leaves the machine unless the
//! user has both enabled cloud AI in settings and confirmed that specific
//! request. See ADR 0011.
//!
//! ## Module map
//!
//! - `provider` the `EmbeddingProvider`/`ChatProvider` traits and shared error type
//! - `ollama` local provider talking to a local Ollama instance
//! - `cloud` opt-in OpenAI-compatible cloud provider
//! - `embeddings` the embedding index and cosine-similarity semantic search
//! - `query_translation` natural language to filter-DSL translation
//! - `settings` AI configuration storage

pub mod cloud;
pub mod embeddings;
pub mod ollama;
pub mod provider;
pub mod query_translation;
pub mod settings;

pub use cloud::CloudProvider;
pub use embeddings::{
    build_embedding_index, index_status, rank_by_similarity, semantic_search, EmbedProgress,
    EmbedStats, SimilarFile,
};
pub use ollama::OllamaProvider;
pub use provider::{AiError, ChatProvider, EmbeddingProvider, Result};
pub use query_translation::{translate, TranslatedQuery};
pub use settings::{get_settings, set_settings, AiSettings};
