//! Editor-independent validation and UTF-16 text operations.

pub mod indentation;
pub mod language;
pub mod text_operations;
pub mod validation;

pub use indentation::indent;
pub use language::{Language, SqlDialect};
pub use text_operations::{newline, TextEdit};
pub use validation::{validate, ValidationError};
