use crate::{Language, SqlDialect};
use sqlparser::{
    dialect::{Dialect, GenericDialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect},
    parser::Parser,
};

/// A validation diagnostic. Locations, when available, are one-based parser
/// coordinates (not UTF-16 edit offsets).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

/// Validate every YAML document or a sequence of SQL statements. SQL parsing
/// reports its first error; this does not check database schemas or semantics.
pub fn validate(content: &str, language: Language) -> Vec<ValidationError> {
    match language {
        Language::Yaml => {
            use serde_yaml_ng::Value;
            for document in serde_yaml_ng::Deserializer::from_str(content) {
                if let Err(error) = <Value as serde::Deserialize>::deserialize(document) {
                    let location = error.location();
                    // The stream cannot advance reliably after a syntax error.
                    return vec![ValidationError {
                        message: error.to_string(),
                        line: location.as_ref().map(|loc| loc.line()),
                        column: location.as_ref().map(|loc| loc.column()),
                    }];
                }
            }
            Vec::new()
        }
        Language::Sql(dialect) => {
            let dialect: Box<dyn Dialect> = match dialect {
                SqlDialect::Generic => Box::new(GenericDialect {}),
                SqlDialect::PostgreSql => Box::new(PostgreSqlDialect {}),
                SqlDialect::MySql => Box::new(MySqlDialect {}),
                SqlDialect::SQLite => Box::new(SQLiteDialect {}),
            };
            match Parser::parse_sql(dialect.as_ref(), content) {
                Ok(_) => Vec::new(),
                Err(error) => {
                    let message = error.to_string();
                    let (line, column) = sql_location(&message);
                    vec![ValidationError {
                        message,
                        line,
                        column,
                    }]
                }
            }
        }
    }
}

// sqlparser's public ParserError stores its diagnostic as a string.
fn sql_location(message: &str) -> (Option<usize>, Option<usize>) {
    let Some((_, location)) = message.rsplit_once("Line: ") else {
        return (None, None);
    };
    let Some((line, column)) = location.split_once(", Column: ") else {
        return (None, None);
    };
    (
        line.parse().ok(),
        column
            .split_whitespace()
            .next()
            .and_then(|value| value.parse().ok()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaml_documents_and_errors() {
        assert!(validate("", Language::Yaml).is_empty());
        assert!(validate("name: 😀\n---\n- one\n- two", Language::Yaml).is_empty());
        let errors = validate("key: [\n", Language::Yaml);
        assert!(!errors.is_empty());
        assert!(errors[0].line.is_some());
        assert!(errors[0].column.is_some());
        assert!(!validate("ok: true\n---\nkey: [", Language::Yaml).is_empty());
    }

    #[test]
    fn sql_dialects_and_multiple_statements() {
        for dialect in [
            SqlDialect::Generic,
            SqlDialect::PostgreSql,
            SqlDialect::MySql,
            SqlDialect::SQLite,
        ] {
            assert!(validate("SELECT 1; SELECT '😀';", Language::Sql(dialect)).is_empty());
            assert!(validate("-- comment", Language::Sql(dialect)).is_empty());
            assert!(!validate("SELECT * FROM", Language::Sql(dialect)).is_empty());
        }
        assert!(validate(
            "SELECT `hello` FROM `table`",
            Language::Sql(SqlDialect::MySql)
        )
        .is_empty());
        assert!(validate(
            "SELECT value::text FROM things",
            Language::Sql(SqlDialect::PostgreSql)
        )
        .is_empty());
    }

    #[test]
    fn sql_error_location() {
        let errors = validate(
            "SELECT 1;\nSELECT * FROM ;",
            Language::Sql(SqlDialect::Generic),
        );
        assert_eq!(errors[0].line, Some(2));
        assert!(errors[0].column.is_some());
    }
}
