# CodeAtlas

A continuously updated index of a codebase: files, symbols, relationships, and answers that cite source.

This is the MVP. It stops at a working loop.

```
Git repository
      ↓
     Index
      ↓
Explore files / symbols
      ↓
Dependency graph
      ↓
Search
      ↓
Ask → answer with citations
```

If that loop works, do not add product surface. Polish it. Measure it. Document it.

## What you can do

1. Point CodeAtlas at a Git URL or a local path.
2. Wait for the index job (files parsed, symbols extracted, graph built).
3. Open **Files** — tree, Monaco, symbol inspector.
4. Open **Graph** — imports, calls, references.
5. **Search** (`⌘K`) by name or meaning.
6. **Ask** a question. The answer cites `path:start-end`. Click a citation to open the source.

Languages in this cut: TypeScript, JavaScript, Rust, Python.

The model never receives the whole repository. Context is retrieved (semantic, symbol, graph), ranked, redacted, then answered. Citations are validated against that pack.

## Run

```bash
cargo test --workspace
cargo run -p codeatlas-api
```

API: `http://127.0.0.1:8080`

```bash
cd dashboard && npm install && npm run dev
```

Dashboard: `http://localhost:5173` (or the port Vite prints). It proxies `/api` to the Axum process.

Index from the dashboard, or:

```bash
curl -X POST http://127.0.0.1:8080/api/repositories \
  -H 'content-type: application/json' \
  -d '{"source":"/absolute/path/to/repo"}'
```

The request returns `202` and a job id. Watch `GET /api/jobs/:id/events`.

## Workstation routes

```
/
├── /dashboard
├── /repositories
└── /repositories/:id
    ├── /overview
    ├── /files
    ├── /graph
    ├── /search
    └── /ai
```

`/insights` exists for impact walkthroughs. It is not required for the MVP loop.

## How the index works

```
source → walk (ignore + limits) → tree-sitter parse → symbols / imports / calls
      → graph → semantic units → embeddings → SQLite
```

Reindex uses the git range from commit A to B (`added`, `modified`, `deleted`, `renamed`) and only reparses affected files. Content hashes still skip unchanged files.

Blocking parse and git work run off the Axum thread. CPU parse uses a bounded rayon pool.

## Answers

```
Question → classifier → retriever (semantic / symbol / graph)
        → context pack → rank → prompt → model
        → citation validator → [citation:abc123] → path:18-42
```

Each answer includes confidence (high / medium / low) and a line such as:

`CodeAtlas found 6 relevant symbols across 4 files.`

## Limits and secrets

Default caps (`CODEATLAS_MAX_FILE_BYTES`, `CODEATLAS_MAX_FILE_COUNT`, `CODEATLAS_MAX_REPO_BYTES`):

- 1 MiB per file
- 100,000 files
- 512 MiB repository

Ignored: `.git`, `node_modules`, `target`, `dist`, `build`, `.cache`.

Never indexed: `.env`, private keys, credential files.

`API_KEY=`, `SECRET=`, `PASSWORD=`, and `PRIVATE_KEY` are redacted before any pack is sent to an external LLM.

## Tests

```bash
cargo test --workspace
```

- Unit tests in each crate
- Parser fixtures (`crates/parser/fixtures/`, including `projects/{react-app,rust-project,python-project,mixed-project}`)
- Indexer integration (`crates/indexer/tests`)
- Graph tests
- API ask + citation test (`crates/api/tests/ask_citations.rs`)

## Benchmarks

Measured on this machine with `cargo run -p codeatlas-benchmarks --bin measure`. Do not copy these numbers onto another host and call them yours.

```
Repository       Files   Symbols   Index     Search    Ask
--------------------------------------------------------------
mini-repo           13        27    0.01s     0.0ms    0.9ms
CodeAtlas          138       935    0.10s     4.1ms   21.3ms
```

Criterion harnesses (files/sec, MB/sec) live in `benchmarks/`:

```bash
cargo bench -p codeatlas-benchmarks
CODEATLAS_BENCH_FILES=10000,50000,100000 cargo bench -p codeatlas-benchmarks
```

## Configuration

| Variable | Default |
|---|---|
| `CODEATLAS_BIND` | `127.0.0.1:8080` |
| `CODEATLAS_DATA_DIR` | `.data` |
| `CODEATLAS_DASHBOARD_DIR` | `dashboard/dist` |
| `CODEATLAS_DATABASE_URL` | SQLite under the data dir; `postgres://…` selects PostgreSQL |
| `OPENAI_API_KEY` | unset → extractive answers, no external LLM |

## Not in this MVP

Streaming answers, always-on file watching, AI code actions, and a TanStack Start host. Axum is the API. Add those only after this loop is boringly solid.
