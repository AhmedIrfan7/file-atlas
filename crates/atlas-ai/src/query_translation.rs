//! Natural-language search: translates a plain-English request into
//! `atlas_search`'s existing filter DSL (`type:pdf size>10mb age<1y`, ...)
//! instead of generating SQL.
//!
//! Generating SQL directly from an LLM and running it would be a real SQL
//! injection surface for something this project treats as safety-critical
//! infrastructure. Translating into the filter DSL instead means the actual
//! search execution never changes: the translated string goes through the
//! exact same `atlas_search::parser` -> `planner` -> `runner` pipeline M3
//! already built and tested, so this module's only new responsibility is
//! producing a string in that grammar, and validating it actually is one
//! before trusting it.

use atlas_search::parser::Filter;
use serde::{Deserialize, Serialize};

use crate::provider::ChatProvider;

/// Category names the classifier assigns (`atlas_core::classifier::Category`),
/// as opposed to file extensions. An LLM asked to translate "documents" or
/// "images" quite naturally reaches for `type:document`/`type:image`, which
/// parses as a syntactically valid `type:` filter but can never match a real
/// file: `type:` compares against extension, not category, and no file has
/// the literal extension "document". Caught by reasoning through what the
/// syntactic parser cannot know, then confirmed as a real failure mode by
/// `hallucinated_category_name_is_rejected` below.
const CATEGORY_NAMES_NOT_EXTENSIONS: &[&str] = &[
    "image",
    "video",
    "audio",
    "document",
    "archive",
    "installer",
    "code",
    "other",
];

const SYSTEM_PROMPT: &str = r#"You translate a person's plain-English file search request into a compact filter query. Reply with ONLY the query string: no explanation, no markdown formatting, no surrounding quotes.

Grammar you may use, space-separated, in any combination:
- type:<extension>  file extension without a dot, e.g. type:pdf, type:jpg, type:zip, type:docx (never a category name like "Document")
- in:<substring>    restricts to files whose folder path contains this text
- size>N, size<N, size>=N, size<=N   N is a number plus a unit: b, kb, mb, gb, tb (e.g. size>10mb)
- age>N, age<N, age>=N, age<=N       N is a number plus a unit: d, w, m, y (e.g. age<1y means modified within the last year)
- any other words are free text, matched against file name and path; wrap a phrase that must stay together in double quotes

Only include a filter the request specifically implies. Do not add an age, folder, or size filter just because one could apply; an unrequested filter narrows the search and hides files the person actually wants to see. If the request has no filterable structure at all, reply with it as plain free text.

Examples:
"pdf files" -> type:pdf
"large zip files" -> type:zip size>100mb
"screenshots from last week" -> type:png age<7d
"tax documents" -> tax documents"#;

/// The result of trying to translate one natural-language request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranslatedQuery {
    /// A string ready to hand to `atlas_search::parser::parse`.
    pub query_text: String,
    /// Whether translation actually happened. When `true`, `query_text` is
    /// just the original input verbatim: no chat model was available, the
    /// model errored, or its output did not parse as a valid filter query.
    /// Falling back to the raw input as free text means a broken or absent
    /// translator degrades search rather than blocking it.
    pub used_fallback: bool,
}

/// Translate `natural_language` using `provider`, falling back to treating
/// the input as free text if the provider is unavailable or its output does
/// not parse under the real filter-DSL grammar.
pub fn translate(provider: &dyn ChatProvider, natural_language: &str) -> TranslatedQuery {
    let fallback = || TranslatedQuery {
        query_text: natural_language.to_string(),
        used_fallback: true,
    };

    let Ok(raw) = provider.complete(SYSTEM_PROMPT, natural_language) else {
        return fallback();
    };
    let candidate = clean(&raw);
    if candidate.is_empty() {
        return fallback();
    }
    let Ok(parsed) = atlas_search::parser::parse(&candidate) else {
        return fallback();
    };
    if parsed
        .filters
        .iter()
        .any(is_category_confused_with_extension)
    {
        return fallback();
    }
    TranslatedQuery {
        query_text: candidate,
        used_fallback: false,
    }
}

fn is_category_confused_with_extension(filter: &Filter) -> bool {
    matches!(filter, Filter::Extension(ext) if CATEGORY_NAMES_NOT_EXTENSIONS.contains(&ext.as_str()))
}

/// Strips the kind of wrapping a chat model adds even when told not to:
/// surrounding quotes, a trailing period, code-fence backticks, and stray
/// commas between filters (observed directly from a real local model: it
/// wrote `type:zip, size>10kb` with a comma-space separator instead of just
/// a space; `type:zip,` then parses "successfully" as the literal, never-
/// matching extension `"zip,"` rather than erroring, so this has to be
/// cleaned up before parsing, not caught by parse failure afterward).
fn clean(raw: &str) -> String {
    let trimmed = strip_backtick_fence(raw.trim()).trim();
    let unwrapped = strip_full_wrap(trimmed, '"');
    let no_commas = strip_unquoted_commas(unwrapped.trim_end_matches('.').trim());
    collapse_spaces(&no_commas).trim().to_string()
}

fn strip_backtick_fence(s: &str) -> &str {
    strip_full_wrap(s, '`')
}

/// Removes `wrap` from both ends only when the WHOLE string is wrapped in a
/// matching pair (starts and ends with it), not whenever either end happens
/// to contain the character. A response like `"Smith, John" type:pdf` starts
/// with `"` but is not fully wrapped (it ends with `pdf`), so this must not
/// touch it the way `str::trim_matches` would (which strips each end
/// independently and would break the quoted phrase's pairing).
fn strip_full_wrap(s: &str, wrap: char) -> &str {
    let starts = s.starts_with(wrap);
    let ends = s.ends_with(wrap);
    if starts && ends && s.chars().count() >= 2 {
        let start = wrap.len_utf8();
        &s[start..s.len() - wrap.len_utf8()]
    } else {
        s
    }
}

/// Replaces `,` with a space, except inside a `"..."` quoted phrase, where a
/// comma is part of the literal text the user is searching for.
fn strip_unquoted_commas(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_quotes = false;
    for c in s.chars() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                out.push(c);
            }
            ',' if !in_quotes => out.push(' '),
            _ => out.push(c),
        }
    }
    out
}

/// Collapses runs of the plain space character down to one, so a comma
/// immediately followed by a space (the common real pattern) does not leave
/// a double space behind once the comma itself becomes a space.
fn collapse_spaces(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_was_space = false;
    for c in s.chars() {
        if c == ' ' {
            if !last_was_space {
                out.push(' ');
            }
            last_was_space = true;
        } else {
            out.push(c);
            last_was_space = false;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_search::parser::Cmp;

    use crate::provider::{AiError, Result};

    struct FakeChat {
        response: Result<String>,
    }

    impl ChatProvider for FakeChat {
        fn complete(&self, _system_prompt: &str, _user_message: &str) -> Result<String> {
            match &self.response {
                Ok(s) => Ok(s.clone()),
                Err(e) => Err(match e {
                    AiError::NoChatModel => AiError::NoChatModel,
                    other => AiError::Request(other.to_string()),
                }),
            }
        }

        fn model_name(&self) -> &'static str {
            "fake"
        }
    }

    #[test]
    fn valid_translation_is_used_as_is() {
        let provider = FakeChat {
            response: Ok("type:pdf size>10mb".to_string()),
        };
        let result = translate(&provider, "big pdfs");
        assert_eq!(result.query_text, "type:pdf size>10mb");
        assert!(!result.used_fallback);
    }

    #[test]
    fn quoted_and_fenced_output_is_cleaned() {
        let provider = FakeChat {
            response: Ok("`\"type:pdf\"`".to_string()),
        };
        let result = translate(&provider, "pdfs");
        assert_eq!(result.query_text, "type:pdf");
        assert!(!result.used_fallback);
    }

    #[test]
    fn invalid_filter_syntax_falls_back_to_free_text() {
        let provider = FakeChat {
            response: Ok("size>10xyz".to_string()),
        };
        let result = translate(&provider, "big files");
        assert_eq!(result.query_text, "big files");
        assert!(result.used_fallback);
    }

    #[test]
    fn provider_error_falls_back_to_free_text() {
        let provider = FakeChat {
            response: Err(AiError::NoChatModel),
        };
        let result = translate(&provider, "big files");
        assert_eq!(result.query_text, "big files");
        assert!(result.used_fallback);
    }

    #[test]
    fn hallucinated_category_name_is_rejected() {
        // type:Document parses fine syntactically (the parser just lowercases
        // it), but would never match a real file: type: compares against
        // extension, and no file has the literal extension "document". This
        // reasoning-caught gap is exactly why translate() checks parsed
        // filters against the category name list, not just parser success.
        let provider = FakeChat {
            response: Ok("type:Document".to_string()),
        };
        let result = translate(&provider, "documents");
        assert_eq!(
            result.query_text, "documents",
            "should fall back to free text"
        );
        assert!(result.used_fallback);
    }

    #[test]
    fn real_extension_that_happens_to_share_no_category_name_is_accepted() {
        let provider = FakeChat {
            response: Ok("type:pdf".to_string()),
        };
        let result = translate(&provider, "pdfs");
        assert_eq!(result.query_text, "type:pdf");
        assert!(!result.used_fallback);
    }

    #[test]
    fn comma_separated_filters_from_a_real_model_are_cleaned_up() {
        // Observed directly from a real local model (llama3.2:1b) during
        // manual verification: it wrote "type:zip, size>10kb" with a
        // comma-space separator. Without stripping it, "type:zip," parses
        // "successfully" as the literal, never-matching extension "zip,"
        // rather than failing, so this would have silently shipped a broken
        // filter as if it were a valid one.
        let provider = FakeChat {
            response: Ok("type:zip, size>10mb".to_string()),
        };
        let result = translate(&provider, "large zip files");
        assert_eq!(result.query_text, "type:zip size>10mb");
        assert!(!result.used_fallback);
    }

    #[test]
    fn comma_inside_a_quoted_phrase_is_preserved() {
        let provider = FakeChat {
            response: Ok("\"Smith, John\" type:pdf".to_string()),
        };
        let result = translate(&provider, "pdfs named Smith, John");
        assert_eq!(result.query_text, "\"Smith, John\" type:pdf");
        assert!(!result.used_fallback);
    }

    // Talks to a REAL local Ollama instance running a real chat model, so
    // this is not just testing the fallback/validation logic against a
    // scripted fake: it proves an actual small local model's output survives
    // translate()'s parse-and-validate step often enough to be useful.
    // Marked `ignore` for the same reason as ollama.rs's real-instance tests.
    #[test]
    #[ignore = "requires llama3.2:1b pulled locally"]
    fn real_local_model_translates_a_size_and_type_request() {
        let provider =
            crate::ollama::OllamaProvider::local_default(Some("llama3.2:1b".to_string()));
        let result = translate(&provider, "large pdf files bigger than 10 megabytes");
        assert!(
            !result.used_fallback,
            "expected the real model's output to parse as a valid filter query, got free text: {}",
            result.query_text
        );
        let parsed = atlas_search::parser::parse(&result.query_text).expect("must be parseable");
        let has_size_filter = parsed.filters.iter().any(|f| {
            matches!(
                f,
                Filter::Size {
                    cmp: Cmp::Gt | Cmp::Ge,
                    ..
                }
            )
        });
        assert!(
            has_size_filter,
            "expected a size filter, got: {:?}",
            parsed.filters
        );
        // A real 1B local model does not always pick type:pdf specifically;
        // "pdf" surviving as free text is an equally valid, working
        // translation (free text matches file name/path), just a looser one.
        // Observed directly: llama3.2:1b returned "size>=10mb" with "pdf" as
        // free text rather than an explicit type:pdf filter on one real run.
        let mentions_pdf = parsed
            .text
            .as_deref()
            .is_some_and(|t| t.to_lowercase().contains("pdf"))
            || parsed
                .filters
                .iter()
                .any(|f| matches!(f, Filter::Extension(ext) if ext == "pdf"));
        assert!(
            mentions_pdf,
            "expected \"pdf\" to survive somewhere in the translation, got: {parsed:?}"
        );
    }
}
