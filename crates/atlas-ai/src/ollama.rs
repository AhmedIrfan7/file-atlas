//! Local AI via Ollama's HTTP API.
//!
//! This is the "local, private by default" half of M9: everything here talks
//! to `http://localhost:11434` (or whatever base URL is configured), never
//! leaves the machine, and degrades to a clear `Unavailable`/`NoChatModel`
//! error rather than blocking anything else in the app when Ollama is not
//! running or a model is not installed.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::provider::{AiError, ChatProvider, EmbeddingProvider, Result};

const DEFAULT_BASE_URL: &str = "http://localhost:11434";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug)]
pub struct OllamaProvider {
    base_url: String,
    client: reqwest::blocking::Client,
    embed_model: String,
    chat_model: Option<String>,
}

impl OllamaProvider {
    #[must_use]
    pub fn new(
        base_url: impl Into<String>,
        embed_model: impl Into<String>,
        chat_model: Option<String>,
    ) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .unwrap_or_default();
        Self {
            base_url: base_url.into(),
            client,
            embed_model: embed_model.into(),
            chat_model,
        }
    }

    /// The usual local defaults: Ollama on its default port, `nomic-embed-text`
    /// for embeddings. `chat_model` is left to the caller to fill in from
    /// whatever is actually installed (see `installed_models`).
    #[must_use]
    pub fn local_default(chat_model: Option<String>) -> Self {
        Self::new(DEFAULT_BASE_URL, "nomic-embed-text", chat_model)
    }

    /// Whether Ollama answers at all right now. Does not imply any specific
    /// model is installed.
    #[must_use]
    pub fn is_available(&self) -> bool {
        self.client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .is_ok_and(|r| r.status().is_success())
    }

    /// Names of every model Ollama currently has installed.
    pub fn installed_models(&self) -> Result<Vec<String>> {
        #[derive(Deserialize)]
        struct ModelEntry {
            name: String,
        }
        #[derive(Deserialize)]
        struct TagsResponse {
            models: Vec<ModelEntry>,
        }

        let resp = self
            .client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .map_err(|e| AiError::Unavailable(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(AiError::Unavailable(format!(
                "ollama returned {}",
                resp.status()
            )));
        }
        let body: TagsResponse = resp
            .json()
            .map_err(|e| AiError::InvalidResponse(e.to_string()))?;
        Ok(body.models.into_iter().map(|m| m.name).collect())
    }
}

impl EmbeddingProvider for OllamaProvider {
    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        #[derive(Serialize)]
        struct Req<'a> {
            model: &'a str,
            prompt: &'a str,
        }
        #[derive(Deserialize)]
        struct Resp {
            embedding: Vec<f32>,
        }

        let resp = self
            .client
            .post(format!("{}/api/embeddings", self.base_url))
            .json(&Req {
                model: &self.embed_model,
                prompt: text,
            })
            .send()
            .map_err(|e| AiError::Unavailable(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(AiError::Request(format!(
                "ollama embeddings returned {}",
                resp.status()
            )));
        }
        let body: Resp = resp
            .json()
            .map_err(|e| AiError::InvalidResponse(e.to_string()))?;
        Ok(body.embedding)
    }

    fn model_name(&self) -> &str {
        &self.embed_model
    }
}

impl ChatProvider for OllamaProvider {
    fn complete(&self, system_prompt: &str, user_message: &str) -> Result<String> {
        #[derive(Serialize)]
        struct Msg<'a> {
            role: &'a str,
            content: &'a str,
        }
        #[derive(Serialize)]
        struct Options {
            temperature: f32,
        }
        #[derive(Serialize)]
        struct Req<'a> {
            model: &'a str,
            messages: Vec<Msg<'a>>,
            stream: bool,
            options: Options,
        }
        #[derive(Deserialize)]
        struct RespMessage {
            content: String,
        }
        #[derive(Deserialize)]
        struct Resp {
            message: RespMessage,
        }

        let model = self.chat_model.as_ref().ok_or(AiError::NoChatModel)?;

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&Req {
                model,
                messages: vec![
                    Msg {
                        role: "system",
                        content: system_prompt,
                    },
                    Msg {
                        role: "user",
                        content: user_message,
                    },
                ],
                stream: false,
                // This is a structured-translation task (plain English into
                // a fixed filter grammar), not creative writing: it wants the
                // model's single best answer, reliably, not a random sample
                // from its output distribution. Without this, Ollama's
                // default temperature (~0.8) means the exact same query can
                // translate correctly on one call and produce nonsense on
                // the next, which is real, reported behavior: the same
                // request translated to a real query, then garbage, then a
                // different garbage, across repeated tries.
                options: Options { temperature: 0.0 },
            })
            .send()
            .map_err(|e| AiError::Unavailable(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(AiError::Request(format!(
                "ollama chat returned {}",
                resp.status()
            )));
        }
        let body: Resp = resp
            .json()
            .map_err(|e| AiError::InvalidResponse(e.to_string()))?;
        Ok(body.message.content)
    }

    fn model_name(&self) -> &str {
        self.chat_model.as_deref().unwrap_or("none")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests talk to a REAL local Ollama instance. They are marked
    // `ignore` so `cargo test` and CI stay side-effect-free and do not
    // depend on the developer's machine having Ollama running with specific
    // models pulled; run explicitly with `cargo test -p atlas-ai -- --ignored`
    // to verify manually.

    #[test]
    #[ignore = "requires a real local Ollama instance"]
    fn real_ollama_reports_available_and_lists_installed_models() {
        let provider = OllamaProvider::local_default(None);
        assert!(
            provider.is_available(),
            "expected Ollama to be running locally"
        );
        let models = provider.installed_models().expect("list installed models");
        assert!(!models.is_empty(), "expected at least one installed model");
    }

    #[test]
    #[ignore = "requires nomic-embed-text pulled locally"]
    fn real_ollama_produces_a_nonempty_embedding() {
        let provider = OllamaProvider::local_default(None);
        let vector = provider
            .embed("invoice.pdf in folder Documents")
            .expect("embed");
        assert!(!vector.is_empty());
        // nomic-embed-text produces 768-dimensional embeddings; a gross
        // sanity check that we are not silently getting a truncated response.
        assert_eq!(vector.len(), 768);
    }

    #[test]
    #[ignore = "requires llama3.2:1b pulled locally"]
    fn real_ollama_chat_completes_a_prompt() {
        let provider = OllamaProvider::local_default(Some("llama3.2:1b".to_string()));
        let reply = provider
            .complete("Reply with exactly one word.", "Say hello.")
            .expect("chat completion");
        assert!(!reply.trim().is_empty());
    }
}
