use axum::{
    extract::{rejection::JsonRejection, DefaultBodyLimit, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use editor_core::{validate, Language, SqlDialect};
use serde::Deserialize;
use std::{io::Write, path::PathBuf};

const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
type ApiError = (StatusCode, String);

#[derive(Deserialize)]
struct SaveRequest {
    file_name: String,
    content: String,
    language: String,
}

fn app(data_dir: PathBuf) -> Router {
    Router::new()
        .fallback(|uri: axum::http::Uri| async move {
            (StatusCode::NOT_FOUND, format!("Unknown route: {uri}"))
        })
        .route("/health", get(|| async { "ok" }))
        .route("/api/save", post(save))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(data_dir)
}

async fn save(
    State(data_dir): State<PathBuf>,
    payload: Result<Json<SaveRequest>, JsonRejection>,
) -> Result<String, ApiError> {
    let Json(request) = payload.map_err(|error| (error.status(), error.body_text()))?;
    let language = match (request.file_name.as_str(), request.language.as_str()) {
        ("config.yaml", "Yaml") => Language::Yaml,
        ("query.sql", "PostgreSql") => Language::Sql(SqlDialect::PostgreSql),
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Allowed file/language pairs: config.yaml/Yaml, query.sql/PostgreSql".into(),
            ));
        }
    };

    // Parsing and filesystem operations are blocking. Nothing touches disk until
    // authoritative server-side validation has succeeded.
    tokio::task::spawn_blocking(move || {
        let errors = validate(&request.content, language);
        if !errors.is_empty() {
            return Err((
                StatusCode::UNPROCESSABLE_ENTITY,
                errors
                    .into_iter()
                    .map(|error| error.message)
                    .collect::<Vec<_>>()
                    .join("\n"),
            ));
        }

        persist(&data_dir, &request).map_err(|error| {
            eprintln!("Save failed: {error}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unable to save document".into(),
            )
        })?;
        Ok(format!("Saved {}\n", request.file_name))
    })
    .await
    .map_err(|error| {
        eprintln!("Save task failed: {error}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Unable to save document".into(),
        )
    })?
}

fn persist(data_dir: &std::path::Path, request: &SaveRequest) -> std::io::Result<()> {
    std::fs::create_dir_all(data_dir)?;
    // Same-directory temporary files ensure the rename stays on one filesystem.
    // NamedTempFile removes the temporary file on errors.
    let mut temporary = tempfile::NamedTempFile::new_in(data_dir)?;
    temporary.write_all(request.content.as_bytes())?;
    temporary.as_file().sync_all()?;
    temporary.persist(data_dir.join(&request.file_name))?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = std::env::var_os("EDITOR_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./saved-documents"));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:39181").await?;
    println!("Editor save backend listening on http://127.0.0.1:39181");
    axum::serve(listener, app(data_dir)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use serde_json::json;
    use tower::ServiceExt;

    async fn send(router: Router, body: String) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/api/save")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn saves_and_atomically_replaces_both_documents() {
        let directory = tempfile::tempdir().unwrap();
        let data_dir = directory.path().join("documents");
        for (file, language, content) in [
            ("config.yaml", "Yaml", "enabled: true\n"),
            ("query.sql", "PostgreSql", "SELECT value::text FROM things;"),
            ("config.yaml", "Yaml", "enabled: false\n"),
        ] {
            let response = send(
                app(data_dir.clone()),
                json!({
                    "file_name": file, "language": language, "content": content
                })
                .to_string(),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(
                response.headers()["content-type"],
                "text/plain; charset=utf-8"
            );
            assert_eq!(
                std::fs::read_to_string(data_dir.join(file)).unwrap(),
                content
            );
        }
        assert_eq!(std::fs::read_dir(data_dir).unwrap().count(), 2);
    }

    #[tokio::test]
    async fn rejects_invalid_requests_before_creating_directory() {
        let directory = tempfile::tempdir().unwrap();
        let data_dir = directory.path().join("not-created");
        for (file, language, content, status) in [
            (
                "config.yaml",
                "Yaml",
                "key: [",
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            (
                "query.sql",
                "PostgreSql",
                "SELECT * FROM",
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            (
                "config.yaml",
                "PostgreSql",
                "SELECT 1",
                StatusCode::BAD_REQUEST,
            ),
            ("query.sql", "Yaml", "key: value", StatusCode::BAD_REQUEST),
            (
                "../config.yaml",
                "Yaml",
                "key: value",
                StatusCode::BAD_REQUEST,
            ),
            (
                "/tmp/query.sql",
                "PostgreSql",
                "SELECT 1",
                StatusCode::BAD_REQUEST,
            ),
            ("other.yaml", "Yaml", "key: value", StatusCode::BAD_REQUEST),
        ] {
            let response = send(
                app(data_dir.clone()),
                json!({
                    "file_name": file, "language": language, "content": content
                })
                .to_string(),
            )
            .await;
            assert_eq!(response.status(), status);
            assert_eq!(
                response.headers()["content-type"],
                "text/plain; charset=utf-8"
            );
            assert!(!to_bytes(response.into_body(), MAX_BODY_BYTES)
                .await
                .unwrap()
                .is_empty());
            assert!(!data_dir.exists());
        }
    }

    #[tokio::test]
    async fn invalid_content_does_not_overwrite_existing_document() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.yaml");
        std::fs::write(&path, "original: true").unwrap();
        let response = send(
            app(directory.path().into()),
            json!({
                "file_name": "config.yaml", "language": "Yaml", "content": "key: ["
            })
            .to_string(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(std::fs::read_to_string(path).unwrap(), "original: true");
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[tokio::test]
    async fn rejects_malformed_and_oversized_bodies_without_persistence() {
        let directory = tempfile::tempdir().unwrap();
        let data_dir = directory.path().join("not-created");
        for (body, status) in [
            ("{".to_string(), StatusCode::BAD_REQUEST),
            ("{}".to_string(), StatusCode::UNPROCESSABLE_ENTITY),
            (
                "x".repeat(MAX_BODY_BYTES + 1),
                StatusCode::PAYLOAD_TOO_LARGE,
            ),
        ] {
            let response = send(app(data_dir.clone()), body).await;
            assert_eq!(response.status(), status);
            assert_eq!(
                response.headers()["content-type"],
                "text/plain; charset=utf-8"
            );
            assert!(!data_dir.exists());
        }
    }

    #[tokio::test]
    async fn persistence_failure_is_a_plain_text_server_error() {
        let directory = tempfile::tempdir().unwrap();
        let data_dir = directory.path().join("a-file");
        std::fs::write(&data_dir, "not a directory").unwrap();
        let response = send(
            app(data_dir),
            json!({
                "file_name": "query.sql", "language": "PostgreSql", "content": "SELECT 1;"
            })
            .to_string(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            response.headers()["content-type"],
            "text/plain; charset=utf-8"
        );
    }
}
