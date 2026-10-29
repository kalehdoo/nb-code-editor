# Demo save backend

Local-only Axum backend included in the workspace.

From the repository root:

```sh
cargo run -p editor-demo-server
# Optional: choose a trusted, server-owned storage directory.
EDITOR_DATA_DIR=./my-documents cargo run -p editor-demo-server
cargo test -p editor-demo-server
```

Binds only `127.0.0.1:39181`. Storage defaults to `./saved-documents`, relative to
the process working directory. The directory is created only after successful
validation. Keep it private to the server; do not point it at an untrusted or
shared writable directory.

```sh
curl --fail-with-body http://127.0.0.1:39181/api/save \
  -H 'Content-Type: application/json' \
  --data '{"file_name":"config.yaml","content":"enabled: true\n","language":"Yaml"}'

curl --fail-with-body http://127.0.0.1:39181/api/save \
  -H 'Content-Type: application/json' \
  --data '{"file_name":"query.sql","content":"SELECT 1::text;","language":"PostgreSql"}'
```

Only those exact file/language pairs are accepted. The entire JSON body is
limited to 2 MiB (including JSON encoding overhead). Responses are plain text:
200 on success, 400 for disallowed pairs, 422 for invalid document syntax,
413 for oversized bodies, and 500 for persistence failures. Malformed JSON,
missing fields, or incorrect content types also receive non-2xx plain-text
errors. Validation uses `editor-core` on the server; SQL validation checks syntax,
not database schemas or execution safety. SQL is never executed.

Writes use a uniquely named temporary file in the storage directory, flush it
to disk, then atomically rename it over the destination. Failed writes clean up
the temporary file. Concurrent saves are last-rename-wins; this is not a
versioned store. Atomic replacement does not promise survival of the directory
entry across power loss.

No authentication or CORS is added. Use a same-origin development proxy for
`/api/save` to `http://127.0.0.1:39181` when connecting a browser frontend. The included demo uses this proxy and endpoint.
