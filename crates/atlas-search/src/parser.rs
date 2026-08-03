//! Filter DSL: turns a query string like `report type:pdf size>10mb age<1y
//! in:downloads` into a structured `SearchQuery`.
//!
//! Grammar, informally:
//!
//! - `type:<ext>` restricts to files with that extension (no leading dot).
//! - `in:<substr>` restricts to files whose parent folder contains `<substr>`.
//! - `size>10mb`, `size<500kb`, `size>=1gb`, `size<=2tb` restrict by file size.
//!   Units: b, kb, mb, gb, tb (case-insensitive, default is bytes).
//! - `age>1y`, `age<30d`, `age>=6m`, `age<=2w` restrict by how long ago the
//!   file was last modified. Units: d (day), w (week, 7d), m (month, 30d),
//!   y (year, 365d). Months and years are approximate; this is a filter for
//!   "roughly how old", not a calendar computation.
//! - Anything else is free text, matched against file name and path.
//! - Double-quoted phrases (`"annual report"`) are kept together as one
//!   free-text token instead of being split on whitespace.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParseError {
    #[error("invalid size value in {0:?}")]
    InvalidSize(String),
    #[error("invalid age value in {0:?}")]
    InvalidAge(String),
    #[error("unclosed quote in query")]
    UnclosedQuote,
}

pub type Result<T> = std::result::Result<T, ParseError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cmp {
    Gt,
    Ge,
    Lt,
    Le,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Filter {
    Extension(String),
    InFolder(String),
    Size {
        cmp: Cmp,
        bytes: u64,
    },
    /// Age filters are expressed as "seconds ago" thresholds. `Gt`/`Ge` mean
    /// "at least this long ago" (older); `Lt`/`Le` mean "less than this long
    /// ago" (newer).
    Age {
        cmp: Cmp,
        seconds_ago: i64,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchQuery {
    pub text: Option<String>,
    pub filters: Vec<Filter>,
}

pub fn parse(input: &str) -> Result<SearchQuery> {
    let tokens = tokenize(input)?;
    let mut text_parts = Vec::new();
    let mut filters = Vec::new();

    for token in tokens {
        if let Some(rest) = token.strip_prefix("type:") {
            filters.push(Filter::Extension(
                rest.trim_start_matches('.').to_lowercase(),
            ));
        } else if let Some(rest) = token.strip_prefix("in:") {
            filters.push(Filter::InFolder(rest.to_string()));
        } else if let Some((cmp, rest)) = strip_cmp_prefix(&token, "size") {
            let bytes = parse_size(rest).ok_or_else(|| ParseError::InvalidSize(token.clone()))?;
            filters.push(Filter::Size { cmp, bytes });
        } else if let Some((cmp, rest)) = strip_cmp_prefix(&token, "age") {
            let seconds_ago =
                parse_age(rest).ok_or_else(|| ParseError::InvalidAge(token.clone()))?;
            filters.push(Filter::Age { cmp, seconds_ago });
        } else if !token.is_empty() {
            text_parts.push(token);
        }
    }

    Ok(SearchQuery {
        text: (!text_parts.is_empty()).then(|| text_parts.join(" ")),
        filters,
    })
}

fn strip_cmp_prefix<'a>(token: &'a str, key: &str) -> Option<(Cmp, &'a str)> {
    for (marker, cmp) in [
        (">=", Cmp::Ge),
        ("<=", Cmp::Le),
        (">", Cmp::Gt),
        ("<", Cmp::Lt),
    ] {
        let prefix = format!("{key}{marker}");
        if let Some(rest) = token.strip_prefix(&prefix) {
            return Some((cmp, rest));
        }
    }
    None
}

// A query someone types by hand never approaches u64/i64 range limits, so the
// float-to-int casts below cannot realistically overflow or go negative.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn parse_size(value: &str) -> Option<u64> {
    let (number, unit) = split_number_unit(value);
    let n: f64 = number.parse().ok()?;
    let multiplier: f64 = match unit.to_lowercase().as_str() {
        "" | "b" => 1.0,
        "kb" => 1024.0,
        "mb" => 1024.0 * 1024.0,
        "gb" => 1024.0 * 1024.0 * 1024.0,
        "tb" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    Some((n * multiplier).round() as u64)
}

const SECONDS_PER_DAY: i64 = 86_400;

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn parse_age(value: &str) -> Option<i64> {
    let (number, unit) = split_number_unit(value);
    let n: f64 = number.parse().ok()?;
    let days: f64 = match unit.to_lowercase().as_str() {
        "d" => 1.0,
        "w" => 7.0,
        "m" => 30.0,
        "y" => 365.0,
        _ => return None,
    };
    Some((n * days * SECONDS_PER_DAY as f64).round() as i64)
}

fn split_number_unit(value: &str) -> (&str, &str) {
    let split_at = value
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(value.len());
    value.split_at(split_at)
}

fn tokenize(input: &str) -> Result<Vec<String>> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }
        if c == '"' {
            chars.next();
            let mut phrase = String::new();
            let mut closed = false;
            for c in chars.by_ref() {
                if c == '"' {
                    closed = true;
                    break;
                }
                phrase.push(c);
            }
            if !closed {
                return Err(ParseError::UnclosedQuote);
            }
            tokens.push(phrase);
        } else {
            let mut word = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() {
                    break;
                }
                word.push(c);
                chars.next();
            }
            tokens.push(word);
        }
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_text_only() {
        let q = parse("resume final").unwrap();
        assert_eq!(q.text.as_deref(), Some("resume final"));
        assert!(q.filters.is_empty());
    }

    #[test]
    fn extension_filter() {
        let q = parse("type:PDF").unwrap();
        assert_eq!(q.filters, vec![Filter::Extension("pdf".into())]);
        assert!(q.text.is_none());
    }

    #[test]
    fn folder_filter() {
        let q = parse("in:downloads").unwrap();
        assert_eq!(q.filters, vec![Filter::InFolder("downloads".into())]);
    }

    #[test]
    fn size_filters_parse_units() {
        assert_eq!(
            parse("size>10mb").unwrap().filters,
            vec![Filter::Size {
                cmp: Cmp::Gt,
                bytes: 10 * 1024 * 1024
            }]
        );
        assert_eq!(
            parse("size<=1gb").unwrap().filters,
            vec![Filter::Size {
                cmp: Cmp::Le,
                bytes: 1024 * 1024 * 1024
            }]
        );
    }

    #[test]
    fn age_filters_parse_units() {
        let q = parse("age>1y").unwrap();
        assert_eq!(
            q.filters,
            vec![Filter::Age {
                cmp: Cmp::Gt,
                seconds_ago: 365 * SECONDS_PER_DAY
            }]
        );
    }

    #[test]
    fn quoted_phrase_stays_together() {
        let q = parse("\"annual report\" type:pdf").unwrap();
        assert_eq!(q.text.as_deref(), Some("annual report"));
        assert_eq!(q.filters, vec![Filter::Extension("pdf".into())]);
    }

    #[test]
    fn mixed_query_combines_text_and_filters() {
        let q = parse("resume type:pdf size>10kb age<1y in:downloads").unwrap();
        assert_eq!(q.text.as_deref(), Some("resume"));
        assert_eq!(q.filters.len(), 4);
    }

    #[test]
    fn unclosed_quote_is_an_error() {
        assert_eq!(parse("\"oops"), Err(ParseError::UnclosedQuote));
    }

    #[test]
    fn invalid_size_unit_is_an_error() {
        assert!(matches!(
            parse("size>10xy"),
            Err(ParseError::InvalidSize(_))
        ));
    }
}
