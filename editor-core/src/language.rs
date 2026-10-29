/// Languages understood by the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Yaml,
    Sql(SqlDialect),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlDialect {
    Generic,
    PostgreSql,
    MySql,
    SQLite,
}

impl Language {
    pub const fn default_indent(self) -> usize {
        match self {
            Self::Yaml => 2,
            Self::Sql(_) => 4,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Yaml => "YAML",
            Self::Sql(_) => "SQL",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_metadata() {
        assert_eq!(Language::Yaml.default_indent(), 2);
        assert_eq!(Language::Yaml.label(), "YAML");
        for dialect in [
            SqlDialect::Generic,
            SqlDialect::PostgreSql,
            SqlDialect::MySql,
            SqlDialect::SQLite,
        ] {
            assert_eq!(Language::Sql(dialect).default_indent(), 4);
            assert_eq!(Language::Sql(dialect).label(), "SQL");
        }
    }
}
