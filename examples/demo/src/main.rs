use editor_leptos::{CodeEditor, Language, SqlDialect, Theme, EDITOR_CSS};
use leptos::prelude::*;
fn main() {
    leptos::mount::mount_to_body(|| {
        let make_save = |file_name: &'static str, language: &'static str| {
            Callback::new(move |(text, ack): (String, Callback<Result<(), String>>)| {
                leptos::task::spawn_local(async move {
                    let result = async {
                    let body = serde_json::json!({"file_name": file_name, "content": text, "language": language});
                    let response = gloo_net::http::Request::post("/api/save").json(&body).map_err(|e| e.to_string())?.send().await.map_err(|e| e.to_string())?;
                    if response.ok() { Ok(()) } else { Err(response.text().await.unwrap_or_else(|_| "Save failed".into())) }
                }.await;
                    ack.run(result);
                });
            })
        };
        let save_yaml = make_save("config.yaml", "Yaml");
        let save_sql = make_save("query.sql", "PostgreSql");
        view! {
            <style>{EDITOR_CSS}</style>
            <main style="max-width:960px;margin:40px auto;display:grid;gap:24px">
                <h1>"YAML and SQL editors"</h1>
                <h2>"Light theme"</h2>
                <CodeEditor initial_content="name: Nodeberries\nsettings:\n  enabled: true\n" language=Language::Yaml file_name="config.yaml" theme=Theme::Light on_save=save_yaml />
                <CodeEditor initial_content="SELECT id, name\nFROM customers\nWHERE active = true;\n" language=Language::Sql(SqlDialect::PostgreSql) file_name="query.sql" theme=Theme::Light on_save=save_sql />
                <p>"SQL validation checks syntax only, not database schemas, permissions, or execution safety."</p>
                <h2>"Dark theme — editable"</h2>
                <p>"These examples save to the same config.yaml and query.sql files as the light examples."</p>
                <CodeEditor initial_content="name: Nodeberries\nsettings:\n  enabled: true\n" language=Language::Yaml file_name="config.yaml (dark)" theme=Theme::Dark on_save=save_yaml />
                <CodeEditor initial_content="SELECT id, name\nFROM customers\nWHERE active = true;\n" language=Language::Sql(SqlDialect::PostgreSql) file_name="query.sql (dark)" theme=Theme::Dark on_save=save_sql />
                <h2>"Read-only reference"</h2>
                <p>"Viewing only: editing and saving are disabled in this example."</p>
                <CodeEditor initial_content="example: read-only\n" language=Language::Yaml file_name="reference.yaml" read_only=true theme=Theme::Dark height="140px" on_save=save_yaml />
            </main>
        }
    });
}
