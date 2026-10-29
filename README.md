# Nodeberries code editor

A small Rust + Leptos YAML/SQL editor built on a native HTML `textarea`. No JavaScript editor framework, syntax highlighting, or custom cursor engine.

## Run the examples

Requires Rust, the `wasm32-unknown-unknown` target, and [Trunk](https://trunkrs.dev/).

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
# Terminal 1, from the repository root:
cargo run -p editor-demo-server
# Terminal 2:
cd examples/demo
trunk serve --address 127.0.0.1 --port 8080
```

Open `http://127.0.0.1:8080`. The page contains editable YAML and PostgreSQL examples in both light and dark themes, plus a separately labeled read-only reference. Light and dark examples save to the same backend files. The Trunk proxy forwards saves to `127.0.0.1:39181`. Valid saves are atomically written under `saved-documents/` in the backend working directory; set `EDITOR_DATA_DIR` to override it. Examples start from their fixture content on reload; they do not load saved files.

The backend validates again before writing. Its file names and languages are allowlisted, bodies are limited to 2 MiB, and SQL is never executed. This is a **local demonstration**, not an authenticated production storage API. Add authentication, authorization, CSRF protection, application-specific validation, and concurrency control before production integration. SQL syntax validation does not check database objects, permissions, or query safety.

## Workspace

- `editor-core/src/`: framework-independent language metadata, indentation, UTF-16 selection operations, YAML stream validation (`serde_yaml_ng`), and SQL dialect parsing (`sqlparser`).
- `editor-leptos/src/`: reusable `CodeEditor`, line gutter, toolbar, validation panel, status, and stylesheet.
- `examples/demo/`: Leptos CSR browser example.
- `examples/server/`: authoritative validation and atomic persistence example.
- `tests/editor.spec.js`: Chromium, Firefox, and WebKit integration tests.

## Component API

```rust
use editor_leptos::{CodeEditor, Language, Theme, EDITOR_CSS};
use leptos::prelude::*;

// Supply your own async server request. Acknowledge Ok only AFTER the server
// validates and durably persists the submitted content.
let save = Callback::new(|(content, acknowledge):
    (String, Callback<Result<(), String>>)| {
    // Spawn your request here; invoke acknowledge.run(result) on completion.
    let _ = content;
    acknowledge.run(Err("Connect your save endpoint".into()));
});
let editor = view! {
    <style>{EDITOR_CSS}</style>
    <CodeEditor
        initial_content="enabled: true\n"
        language=Language::Yaml
        file_name="config.yaml"
        theme=Theme::Auto
        height="360px"
        on_save=save
    />
};
```

| Prop | Behavior |
| --- | --- |
| `initial_content` | Initial-only string; remount to open a different document |
| `language` | `Language::Yaml` or `Language::Sql(SqlDialect::…)` |
| `file_name` | Header and accessible textarea label; defaults to `untitled` |
| `read_only` | Native textarea read-only; Save disabled |
| `theme` | `Light`, `Dark`, or `Auto` (system preference) |
| `height` | CSS height of editing area; defaults to `320px` |
| `indentation_size` | Optional spaces, clamped to 1–16; defaults to YAML 2 / SQL 4 |
| `on_save` | Required callback `(String, Callback<Result<(), String>>)` |
| `on_validation` | Optional `Callback<Vec<ValidationError>>` |
| `on_change` | Optional `Callback<String>` per native input event, including undo |

Configuration props are initial/static, not reactive signals. Save stays pending until acknowledged; repeated saves are disabled. Editing during a save is allowed and newer changes remain dirty after acknowledgement. A failed save retains unsaved state and displays the server error. Initial content is considered the saved baseline. Line endings are normalized to LF, as required by HTML textarea behavior. Character counts are Unicode scalar values, not UTF-16 units or grapheme clusters.

Use `--surface`, `--text`, `--border`, and `--text-muted` CSS custom properties to inherit application tokens. Standalone light/dark fallbacks are included because this repository had no existing token system. Include `EDITOR_CSS` once in your application (prefer a stylesheet under strict CSP).

## Keyboard and browser behavior

- Native selection, clipboard, cursor movement, and undo/redo remain browser-owned.
- **Ctrl+S / Cmd+S** saves the focused editor through the same validation and backend callback as the Save button. The browser's Save Page action is suppressed inside the textarea, even when saving is unavailable. Unchanged, read-only, pending-save, and held-key repeat events do not trigger saves. Shift/Alt variants and IME composition are left untouched.
- Tab inserts spaces; Shift+Tab removes leading indentation; selected blocks affect whole lines. Enter preserves the indentation prefix before the cursor.
- **Escape then Tab** leaves the editor, avoiding a keyboard focus trap. Read-only editors retain native Tab navigation. IME composition and other Ctrl/Meta/Alt shortcuts are not intercepted.
- Custom edits use the browser's deprecated but widely supported `execCommand` editing transaction to preserve native undo. `setRangeText`/assigning `.value` would discard or bypass undo history. If the browser rejects the command, the original selection and native key behavior are retained; indentation is not guaranteed on unsupported engines. Browsers may coalesce consecutive edits into one undo transaction.
- Text is not controlled/reassigned on each keystroke. One input pass updates counts/dirty state; line-number text changes only when line count changes. Parsing occurs only on Validate/Save. Very large documents still have O(n) counting and validation costs; this is not a virtualized editor, and parsing currently runs synchronously in the browser.

## Tests

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
# From examples/demo:
trunk build
# From the repository root (Node.js needed only for browser tests):
npm ci
npx playwright install
npm run test:browser
```

Playwright starts and stops its own finite-lifetime demo servers. Do not run another application on its configured ports. Tests cover Unicode/block selection, indentation, native undo/redo, focus escape, large-input statistics, scroll alignment, validation, save success/failure, read-only behavior, and theme configuration. Rust tests also cover CRLF, SQL dialects, YAML streams, backend validation failures, size limits, safe paths, and atomic replacement.
