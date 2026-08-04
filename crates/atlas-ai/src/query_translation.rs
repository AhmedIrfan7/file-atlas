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

/// Words that can never be a genuine file extension, as opposed to what an
/// LLM sometimes reaches for anyway. Two distinct sources feed this list:
///
/// - Classifier category names (`atlas_core::classifier::Category`): asked to
///   translate "documents" or "images", a model quite naturally produces
///   `type:document`/`type:image`, which parses as a syntactically valid
///   `type:` filter but can never match a real file, since `type:` compares
///   against extension, not category. Confirmed as a real failure mode by
///   `hallucinated_category_name_is_rejected` below.
/// - Structural nouns from the request itself ("folder(s)", "file(s)"): a
///   request like "large folders where size > 50 mb" has no extension to
///   translate at all (there is no folder-size filter in this grammar), and
///   a small model asked anyway sometimes uses the noun itself as the
///   extension (`type:folders`) rather than recognizing there is nothing to
///   fill that slot with. Observed directly from llama3.2:1b; see
///   `real_local_model_does_not_invent_a_type_filter_for_folders`.
const CATEGORY_NAMES_NOT_EXTENSIONS: &[&str] = &[
    "image",
    "video",
    "audio",
    "document",
    "archive",
    "installer",
    "code",
    "other",
    "folder",
    "folders",
    "file",
    "files",
];

const SYSTEM_PROMPT: &str = r#"You translate a person's plain-English file search request into a compact filter query. Reply with ONLY the query string on a single line: no explanation, no markdown formatting, no surrounding quotes, and never repeat any part of these instructions back.

Grammar you may use, space-separated, in any combination:
- type:<extension>  file extension without a dot, glued directly to "type:" with no space, e.g. type:pdf, type:jpg, type:zip, type:docx (never a category name like "Document", never left empty)
- in:<substring>    restricts to files whose folder path contains this text, glued directly to "in:" with no space
- size>N, size<N, size>=N, size<=N   glued together with no spaces anywhere in the token; N is a positive number plus a unit: b, kb, mb, gb, tb (e.g. size>10mb)
- age>N, age<N, age>=N, age<=N       glued together with no spaces anywhere in the token; N is a positive number plus a unit: d, w, m, y (e.g. age<1y means modified within the last year); never negative
- any other words are free text, matched against file name and path; wrap a phrase that must stay together in double quotes

Only include a filter the request specifically implies. Do not add an age, folder, or size filter just because one could apply; an unrequested filter narrows the search and hides files the person actually wants to see. If the request has no filterable structure at all, reply with it as plain free text. If no filter in this grammar applies to a word in the request (for example "folders", which this grammar has no filter for), leave that word out rather than inventing an unrelated filter for it.

Examples:
"pdf files" -> type:pdf
"large zip files" -> type:zip size>100mb
"large folders" -> size>100mb
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
    if parsed.filters.iter().any(is_filter_semantically_useless) {
        return fallback();
    }
    // A genuine translation is short: a handful of filter tokens plus at most
    // a few free-text words. A small local model that cannot translate a
    // request sometimes rambles instead, or echoes part of its own system
    // prompt back (observed directly: a real response ended with "...any
    // other words are free text wrapped in double quotes: \"screenshots from
    // last week\"", a fragment lifted straight from this module's own
    // SYSTEM_PROMPT). None of that fails to parse, since anything not
    // recognized as a filter is deliberately accepted as free text, so it
    // has to be caught here instead of as a parse error.
    // Off-by-one on purpose: observed directly from a real model, a garbage
    // response ("size:gb size:kb size:mb size:tb folder:name largest folders
    // larger than 20mb", none of which are recognized filter prefixes) came
    // in at exactly 10 free-text words and slipped through a `>` check.
    if free_text_word_count(&parsed) >= MAX_FREE_TEXT_WORDS {
        return fallback();
    }
    TranslatedQuery {
        query_text: candidate,
        used_fallback: false,
    }
}

const MAX_FREE_TEXT_WORDS: usize = 10;

fn free_text_word_count(query: &atlas_search::parser::SearchQuery) -> usize {
    query
        .text
        .as_deref()
        .map_or(0, |t| t.split_whitespace().count())
}

/// A filter that parses successfully but can never match a real file: an
/// extension that is actually a category name (`type:` compares against
/// extension, not category), an extension/folder filter left empty because
/// the model glued a space after the prefix (`type: zip`, `in: `) instead of
/// directly against it, or an `in:` substring containing `*`/`?`. This
/// grammar's `in:` is a plain substring match, not a glob, and `*`/`?` are
/// reserved characters no real Windows path can ever contain, so a value
/// like `*folders*` (observed directly from llama3.2:1b, presumably
/// mimicking shell glob syntax) can never match anything either.
fn is_filter_semantically_useless(filter: &Filter) -> bool {
    match filter {
        Filter::Extension(ext) => {
            ext.is_empty() || CATEGORY_NAMES_NOT_EXTENSIONS.contains(&ext.as_str())
        }
        Filter::InFolder(sub) => sub.is_empty() || sub.contains(['*', '?']),
        Filter::Size { .. } | Filter::Age { .. } => false,
    }
}

/// Strips the kind of wrapping a chat model adds even when told not to:
/// surrounding quotes, a trailing period, code-fence backticks, and stray
/// commas between filters (observed directly from a real local model: it
/// wrote `type:zip, size>10kb` with a comma-space separator instead of just
/// a space; `type:zip,` then parses "successfully" as the literal, never-
/// matching extension `"zip,"` rather than erroring, so this has to be
/// cleaned up before parsing, not caught by parse failure afterward).
fn clean(raw: &str) -> String {
    // A well-formed reply is one line; a model that ignores the
    // one-line-only instruction and appends an explanation or trails off
    // into commentary puts that on later lines, which this drops rather
    // than folding into the query.
    let first_line = raw.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    let trimmed = strip_backtick_fence(first_line.trim()).trim();
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
    fn ten_word_garbage_response_falls_back_to_free_text() {
        // Observed directly from a real model asked to translate "folders
        // larger than 20 mb": it returned exactly this, ten words long, none
        // of them a recognized filter prefix (size: and folder: are not
        // size>/in:). A `> 10` check lets exactly-10 slip through; this must
        // be `>= 10`.
        let provider = FakeChat {
            response: Ok(
                "size:gb size:kb size:mb size:tb folder:name largest folders larger than 20mb"
                    .to_string(),
            ),
        };
        let result = translate(&provider, "folders larger than 20 mb");
        assert_eq!(result.query_text, "folders larger than 20 mb");
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
    fn leaked_prompt_instructions_fall_back_to_free_text() {
        // Observed directly from a real local model (llama3.2:1b): asked to
        // translate "large folders more than 50 gbs", it produced valid
        // filters followed by a fragment of this module's own SYSTEM_PROMPT
        // ("...any other words are free text wrapped in double quotes:
        // \"screenshots from last week\""), which is itself one of the
        // prompt's own examples. This parses fine (anything not a recognized
        // filter is deliberately accepted as free text), so only the
        // free-text length cap catches it.
        let provider = FakeChat {
            response: Ok(
                "type:zip size>50gb any other words are free text wrapped in double quotes here"
                    .to_string(),
            ),
        };
        let result = translate(&provider, "large folders more than 50 gbs");
        assert_eq!(result.query_text, "large folders more than 50 gbs");
        assert!(result.used_fallback);
    }

    #[test]
    fn commentary_on_a_second_line_is_dropped_not_folded_in() {
        let provider = FakeChat {
            response: Ok(
                "type:pdf\nHope that helps! Let me know if you want more filters.".to_string(),
            ),
        };
        let result = translate(&provider, "pdfs");
        assert_eq!(result.query_text, "type:pdf");
        assert!(!result.used_fallback);
    }

    #[test]
    fn empty_extension_from_a_stray_space_after_the_colon_is_rejected() {
        // "type: zip" (space after the colon) tokenizes as "type:" (an empty
        // extension) and a separate free-text word "zip", not as the
        // extension filter the model presumably meant.
        let provider = FakeChat {
            response: Ok("type: zip".to_string()),
        };
        let result = translate(&provider, "zip files");
        assert_eq!(result.query_text, "zip files");
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

    #[test]
    fn glob_wildcard_in_folder_filter_is_rejected() {
        // Observed directly from llama3.2:1b: "in:*folders*", presumably
        // mimicking shell glob syntax. This grammar's in: is a plain
        // substring match, and `*`/`?` are reserved characters no real
        // Windows path can ever contain, so this can never match anything.
        let provider = FakeChat {
            response: Ok("size>=50mb in:*folders*".to_string()),
        };
        let result = translate(&provider, "large folders where size > 50 mb");
        assert_eq!(result.query_text, "large folders where size > 50 mb");
        assert!(result.used_fallback);
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

    // Reproduces a real bug report against the real model: asked to
    // translate "large folders more than 50 gbs", llama3.2:1b produced valid
    // filters followed by a chunk of this module's own SYSTEM_PROMPT text.
    // There is no folder-size filter in the grammar at all (Storage Map, not
    // Search, owns that concept), so the only two safe outcomes here are a
    // real, meaningful filter query or a clean fallback to free text; a
    // leaked-instructions query text is neither and must never come back.
    #[test]
    #[ignore = "requires llama3.2:1b pulled locally"]
    fn real_local_model_never_leaks_prompt_text_for_an_unsupported_query() {
        let provider =
            crate::ollama::OllamaProvider::local_default(Some("llama3.2:1b".to_string()));
        let result = translate(&provider, "large folders more than 50 gbs");
        println!(
            "used_fallback={} query_text={:?}",
            result.used_fallback, result.query_text
        );
        let suspicious = ["free text", "wrapped in", "double quotes", "system prompt"];
        for phrase in suspicious {
            assert!(
                !result.query_text.to_lowercase().contains(phrase),
                "translation leaked prompt text ({phrase:?}): {}",
                result.query_text
            );
        }
        if !result.used_fallback {
            let parsed =
                atlas_search::parser::parse(&result.query_text).expect("must be parseable");
            assert!(
                !parsed.filters.iter().any(is_filter_semantically_useless),
                "translation produced a semantically useless filter: {parsed:?}"
            );
        }
    }

    // Reproduces a second real bug report, same root cause, different
    // symptom: the exact phrasing "large folders where size > 50 mb" made
    // llama3.2:1b invent `type:zip`, presumably pattern-matching against the
    // "large zip files" -> "type:zip size>100mb" example rather than
    // reasoning about "folders" specifically. Fixed by adding a directly
    // relevant demonstrated example ("large folders" -> "size>100mb") rather
    // than only describing the rule in prose, since a tiny model follows a
    // shown example far more reliably than an instruction about one.
    #[test]
    #[ignore = "requires llama3.2:1b pulled locally"]
    fn real_local_model_does_not_invent_a_type_filter_for_folders() {
        let provider =
            crate::ollama::OllamaProvider::local_default(Some("llama3.2:1b".to_string()));
        let result = translate(&provider, "large folders where size > 50 mb");
        println!(
            "used_fallback={} query_text={:?}",
            result.used_fallback, result.query_text
        );
        assert!(
            !result.query_text.to_lowercase().contains("type:"),
            "\"folders\" has no extension to match, so any type: filter here is invented: {}",
            result.query_text
        );
    }

    // Reproduces the actual complaint, not just a single bad output: the
    // exact same query, asked repeatedly, gave a real translation once, a
    // ten-word garbage response another time, and a different garbage
    // response a third time. That is Ollama's default sampling temperature
    // (~0.8) doing exactly what it is designed to do for creative text,
    // applied to a task that instead wants the model's single best answer
    // reliably. With temperature fixed at 0 in ollama.rs, the same input
    // should produce the same output every time.
    #[test]
    #[ignore = "requires llama3.2:1b pulled locally"]
    fn real_local_model_is_deterministic_across_repeated_identical_queries() {
        let provider =
            crate::ollama::OllamaProvider::local_default(Some("llama3.2:1b".to_string()));
        let query = "folders larger than 20 mb";
        let first = translate(&provider, query);
        for attempt in 2..=5 {
            let repeat = translate(&provider, query);
            assert_eq!(
                repeat, first,
                "attempt {attempt} produced a different result than the first call for the exact same query"
            );
        }
    }
}
