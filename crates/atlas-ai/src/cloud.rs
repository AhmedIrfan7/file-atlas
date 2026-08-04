//! Opt-in cloud AI: an OpenAI-compatible chat completions client.
//!
//! This is the one place in File Atlas that can send data off-device, and it
//! only ever sends the exact text a caller passes to `complete` (the user's
//! own typed query, never file names, paths, or file contents). Constructing
//! a `CloudProvider` does not itself send anything; the desktop layer only
//! constructs one after the user has enabled cloud AI in settings, and the
//! UI asks for a fresh per-request confirmation before every call. See ADR
//! 0011 for the full reasoning.
//!
//! "OpenAI-compatible" rather than one named vendor's SDK because the
//! `/chat/completions` shape is now a de facto standard: OpenAI, most
//! self-hosted gateways, and many third-party providers all speak it, so one
//! client covers all of them via a configurable base URL rather than locking
//! into a single vendor's API.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::provider::{AiError, ChatProvider, Result};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug)]
pub struct CloudProvider {
    base_url: String,
    api_key: String,
    model: String,
    client: reqwest::blocking::Client,
}

impl CloudProvider {
    #[must_use]
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .unwrap_or_default();
        Self {
            base_url: base_url.into(),
            api_key: api_key.into(),
            model: model.into(),
            client,
        }
    }
}

impl ChatProvider for CloudProvider {
    fn complete(&self, system_prompt: &str, user_message: &str) -> Result<String> {
        #[derive(Serialize)]
        struct Msg<'a> {
            role: &'a str,
            content: &'a str,
        }
        #[derive(Serialize)]
        struct Req<'a> {
            model: &'a str,
            messages: Vec<Msg<'a>>,
            temperature: f32,
        }
        #[derive(Deserialize)]
        struct RespMessage {
            content: String,
        }
        #[derive(Deserialize)]
        struct Choice {
            message: RespMessage,
        }
        #[derive(Deserialize)]
        struct Resp {
            choices: Vec<Choice>,
        }

        let resp = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&Req {
                model: &self.model,
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
                // Same reasoning as ollama.rs: translating into a fixed
                // filter grammar wants the model's single best answer
                // reliably, not a random sample from its output
                // distribution. Left unset, most OpenAI-compatible APIs
                // default to 0.7-1.0.
                temperature: 0.0,
            })
            .send()
            .map_err(|e| AiError::Request(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(AiError::Request(format!(
                "cloud provider returned {}",
                resp.status()
            )));
        }
        let body: Resp = resp
            .json()
            .map_err(|e| AiError::InvalidResponse(e.to_string()))?;
        body.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| AiError::InvalidResponse("empty choices in response".to_string()))
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}
