//! The seam between this crate's logic and whichever AI backend answers a
//! request.
//!
//! Mirrors `atlas_platform::PlatformFs`: core logic depends on these traits,
//! never on a concrete provider, so "which service answers this" stays
//! swappable and testable with a fake.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("local AI service is not reachable: {0}")]
    Unavailable(String),
    #[error("no chat model is configured or installed")]
    NoChatModel,
    #[error("request failed: {0}")]
    Request(String),
    #[error("could not parse the model's response: {0}")]
    InvalidResponse(String),
}

pub type Result<T> = std::result::Result<T, AiError>;

/// Turns text into a vector for semantic similarity search.
pub trait EmbeddingProvider: Send + Sync {
    fn embed(&self, text: &str) -> Result<Vec<f32>>;
    fn model_name(&self) -> &str;
}

/// Turns a system + user prompt pair into a text completion.
pub trait ChatProvider: Send + Sync {
    fn complete(&self, system_prompt: &str, user_message: &str) -> Result<String>;
    fn model_name(&self) -> &str;
}
