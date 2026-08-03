//! Translates a `SearchQuery` into parameterized SQL against the `files` and
//! `files_fts` tables.
//!
//! Pure and DB-free so the SQL shape can be unit tested without a
//! connection; `runner` is the thin layer that actually executes it.

use rusqlite::types::{ToSqlOutput, ValueRef};
use rusqlite::ToSql;

use crate::parser::{Cmp, Filter, SearchQuery};

/// A parameter value bound into the planned SQL. A small closed enum instead
/// of `Box<dyn ToSql>` so callers can inspect and test plans without trait
/// object friction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Param {
    Text(String),
    Int(i64),
}

impl ToSql for Param {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(match self {
            Self::Text(s) => ToSqlOutput::Borrowed(ValueRef::Text(s.as_bytes())),
            Self::Int(i) => ToSqlOutput::Owned(rusqlite::types::Value::Integer(*i)),
        })
    }
}

/// A fully built query: SQL text plus the parameters bound to its `?N`
/// placeholders, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedQuery {
    pub sql: String,
    pub params: Vec<Param>,
}

const SELECT_COLUMNS: &str = "f.path, f.name, f.size_bytes, f.modified_at, f.category, f.is_dir";

/// Build the SQL and parameters for `query`, ready to hand to
/// `runner::execute`. `now_unix` anchors age filters; `limit` bounds result
/// count.
#[must_use]
pub fn plan(query: &SearchQuery, now_unix: i64, limit: u32) -> PlannedQuery {
    let mut params: Vec<Param> = Vec::new();
    let mut conditions: Vec<String> = vec!["f.removed_at IS NULL".to_string()];
    let has_text = query.text.as_deref().is_some_and(|t| !t.trim().is_empty());

    let from_clause = if has_text {
        "FROM files f JOIN files_fts ON files_fts.rowid = f.id".to_string()
    } else {
        "FROM files f".to_string()
    };

    if let Some(text) = query.text.as_deref().filter(|t| !t.trim().is_empty()) {
        conditions.push(format!("files_fts MATCH ?{}", params.len() + 1));
        params.push(Param::Text(fts_match_expr(text)));
    }

    for filter in &query.filters {
        match filter {
            Filter::Extension(ext) => {
                conditions.push(format!("LOWER(f.extension) = ?{}", params.len() + 1));
                params.push(Param::Text(ext.clone()));
            }
            Filter::InFolder(substr) => {
                conditions.push(format!("f.parent LIKE ?{}", params.len() + 1));
                params.push(Param::Text(format!("%{substr}%")));
            }
            Filter::Size { cmp, bytes } => {
                let op = cmp_op(*cmp);
                conditions.push(format!("f.size_bytes {op} ?{}", params.len() + 1));
                params.push(Param::Int(i64::try_from(*bytes).unwrap_or(i64::MAX)));
            }
            Filter::Age { cmp, seconds_ago } => {
                // "older than N" (age > N) means modified_at is BEFORE the
                // cutoff, so the comparison direction is the inverse of the
                // user-facing operator.
                let op = cmp_op(invert_cmp(*cmp));
                let cutoff = now_unix - seconds_ago;
                conditions.push(format!(
                    "f.modified_at IS NOT NULL AND f.modified_at {op} ?{}",
                    params.len() + 1
                ));
                params.push(Param::Int(cutoff));
            }
        }
    }

    let order_by = if has_text {
        "ORDER BY bm25(files_fts) ASC"
    } else {
        "ORDER BY f.modified_at DESC NULLS LAST"
    };

    let limit_param_index = params.len() + 1;
    params.push(Param::Int(i64::from(limit)));

    let sql = format!(
        "SELECT {SELECT_COLUMNS} {from_clause} WHERE {conditions} {order_by} LIMIT ?{limit_param_index}",
        conditions = conditions.join(" AND "),
    );

    PlannedQuery { sql, params }
}

const fn cmp_op(cmp: Cmp) -> &'static str {
    match cmp {
        Cmp::Gt => ">",
        Cmp::Ge => ">=",
        Cmp::Lt => "<",
        Cmp::Le => "<=",
    }
}

const fn invert_cmp(cmp: Cmp) -> Cmp {
    match cmp {
        Cmp::Gt => Cmp::Lt,
        Cmp::Ge => Cmp::Le,
        Cmp::Lt => Cmp::Gt,
        Cmp::Le => Cmp::Ge,
    }
}

/// Build a safe FTS5 MATCH expression from free text. Each whitespace-split
/// word becomes a quoted prefix query (`"word"*`), so special FTS5 syntax
/// characters in the input (colons, hyphens, parentheses) are treated as
/// literal text rather than query operators. Multiple words are implicitly
/// ANDed by FTS5, matching "contains all these words" expectations.
fn fts_match_expr(text: &str) -> String {
    text.split_whitespace()
        .map(|word| format!("\"{}\"*", word.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn text_only_uses_fts_join_and_bm25_order() {
        let q = parse("resume").unwrap();
        let plan = plan(&q, 1_000, 50);
        assert!(plan.sql.contains("JOIN files_fts"));
        assert!(plan.sql.contains("files_fts MATCH ?1"));
        assert!(plan.sql.contains("ORDER BY bm25(files_fts) ASC"));
        assert_eq!(plan.params[0], Param::Text("\"resume\"*".to_string()));
    }

    #[test]
    fn no_text_orders_by_modified_desc_without_fts_join() {
        let q = parse("type:pdf").unwrap();
        let plan = plan(&q, 1_000, 50);
        assert!(!plan.sql.contains("files_fts"));
        assert!(plan.sql.contains("ORDER BY f.modified_at DESC"));
    }

    #[test]
    fn extension_filter_lowercases_comparison() {
        let q = parse("type:PDF").unwrap();
        let plan = plan(&q, 1_000, 50);
        assert!(plan.sql.contains("LOWER(f.extension) = ?"));
        assert!(plan.params.contains(&Param::Text("pdf".to_string())));
    }

    #[test]
    fn size_filter_binds_byte_count() {
        let q = parse("size>10mb").unwrap();
        let plan = plan(&q, 1_000, 50);
        assert!(plan.sql.contains("f.size_bytes > ?"));
        assert!(plan.params.contains(&Param::Int(10 * 1024 * 1024)));
    }

    #[test]
    fn age_gt_inverts_to_modified_before_cutoff() {
        let q = parse("age>1y").unwrap();
        let now = 100_000_000i64;
        let plan = plan(&q, now, 50);
        assert!(plan.sql.contains("f.modified_at <"));
        let expected_cutoff = now - 365 * 86_400;
        assert!(plan.params.contains(&Param::Int(expected_cutoff)));
    }

    #[test]
    fn age_lt_inverts_to_modified_after_cutoff() {
        let q = parse("age<30d").unwrap();
        let now = 100_000_000i64;
        let plan = plan(&q, now, 50);
        assert!(plan.sql.contains("f.modified_at >"));
    }

    #[test]
    fn folder_filter_wraps_in_wildcards() {
        let q = parse("in:downloads").unwrap();
        let plan = plan(&q, 1_000, 50);
        assert!(plan
            .params
            .contains(&Param::Text("%downloads%".to_string())));
    }

    #[test]
    fn limit_is_always_the_last_parameter() {
        let q = parse("type:pdf size>1kb").unwrap();
        let plan = plan(&q, 1_000, 25);
        assert_eq!(plan.params.last(), Some(&Param::Int(25)));
    }

    #[test]
    fn fts_expr_neutralizes_special_characters() {
        let expr = fts_match_expr("foo:bar (baz)");
        assert_eq!(expr, "\"foo:bar\"* \"(baz)\"*");
    }
}
