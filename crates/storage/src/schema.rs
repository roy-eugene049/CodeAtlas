pub const SQLITE_SCHEMA: &str = "
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
CREATE TABLE IF NOT EXISTS repositories (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    branch TEXT NOT NULL,
    commit_sha TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT 0,
    line_count INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS files (
    id TEXT PRIMARY KEY,
    repository_id TEXT NOT NULL,
    path TEXT NOT NULL,
    language TEXT NOT NULL,
    hash TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    FOREIGN KEY(repository_id) REFERENCES repositories(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS symbols (
    id TEXT PRIMARY KEY,
    file_id TEXT NOT NULL,
    repository_id TEXT,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    FOREIGN KEY(file_id) REFERENCES files(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS relationships (
    source_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    repository_id TEXT NOT NULL,
    FOREIGN KEY(repository_id) REFERENCES repositories(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS chunks (
    id TEXT PRIMARY KEY,
    symbol_id TEXT NOT NULL,
    content TEXT NOT NULL,
    token_count INTEGER NOT NULL,
    FOREIGN KEY(symbol_id) REFERENCES symbols(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS embeddings (
    chunk_id TEXT PRIMARY KEY,
    repository_id TEXT NOT NULL,
    dim INTEGER NOT NULL,
    vector BLOB NOT NULL,
    FOREIGN KEY(repository_id) REFERENCES repositories(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS analysis_runs (
    id TEXT PRIMARY KEY,
    repository_id TEXT,
    source TEXT NOT NULL,
    status TEXT NOT NULL,
    stage TEXT NOT NULL,
    files_discovered INTEGER NOT NULL DEFAULT 0,
    files_parsed INTEGER NOT NULL DEFAULT 0,
    files_skipped INTEGER NOT NULL DEFAULT 0,
    symbols_extracted INTEGER NOT NULL DEFAULT 0,
    relationships_built INTEGER NOT NULL DEFAULT 0,
    chunks_embedded INTEGER NOT NULL DEFAULT 0,
    message TEXT NOT NULL DEFAULT '',
    error TEXT,
    started_at INTEGER NOT NULL DEFAULT 0,
    finished_at INTEGER
);
CREATE TABLE IF NOT EXISTS git_evolution (
    repository_id TEXT PRIMARY KEY,
    payload TEXT NOT NULL,
    FOREIGN KEY(repository_id) REFERENCES repositories(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_files_repo ON files(repository_id);
CREATE INDEX IF NOT EXISTS idx_symbols_file ON symbols(file_id);
CREATE INDEX IF NOT EXISTS idx_relationships_repo ON relationships(repository_id);
CREATE INDEX IF NOT EXISTS idx_chunks_symbol ON chunks(symbol_id);
CREATE INDEX IF NOT EXISTS idx_embeddings_repo ON embeddings(repository_id);
CREATE INDEX IF NOT EXISTS idx_runs_repo ON analysis_runs(repository_id);
";

pub const POSTGRES_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS repositories (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    branch TEXT NOT NULL,
    commit_sha TEXT NOT NULL,
    created_at BIGINT NOT NULL DEFAULT 0,
    line_count BIGINT NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS files (
    id TEXT PRIMARY KEY,
    repository_id TEXT NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    language TEXT NOT NULL,
    hash TEXT NOT NULL,
    size_bytes BIGINT NOT NULL
);
CREATE TABLE IF NOT EXISTS symbols (
    id TEXT PRIMARY KEY,
    file_id TEXT NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS relationships (
    source_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    repository_id TEXT NOT NULL REFERENCES repositories(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS chunks (
    id TEXT PRIMARY KEY,
    symbol_id TEXT NOT NULL REFERENCES symbols(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    token_count INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS embeddings (
    chunk_id TEXT PRIMARY KEY,
    repository_id TEXT NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    dim INTEGER NOT NULL,
    vector BYTEA NOT NULL
);
CREATE TABLE IF NOT EXISTS analysis_runs (
    id TEXT PRIMARY KEY,
    repository_id TEXT,
    source TEXT NOT NULL,
    status TEXT NOT NULL,
    stage TEXT NOT NULL,
    files_discovered BIGINT NOT NULL DEFAULT 0,
    files_parsed BIGINT NOT NULL DEFAULT 0,
    files_skipped BIGINT NOT NULL DEFAULT 0,
    symbols_extracted BIGINT NOT NULL DEFAULT 0,
    relationships_built BIGINT NOT NULL DEFAULT 0,
    chunks_embedded BIGINT NOT NULL DEFAULT 0,
    message TEXT NOT NULL DEFAULT '',
    error TEXT,
    started_at BIGINT NOT NULL DEFAULT 0,
    finished_at BIGINT
);
CREATE TABLE IF NOT EXISTS git_evolution (
    repository_id TEXT PRIMARY KEY REFERENCES repositories(id) ON DELETE CASCADE,
    payload TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_files_repo ON files(repository_id);
CREATE INDEX IF NOT EXISTS idx_symbols_file ON symbols(file_id);
CREATE INDEX IF NOT EXISTS idx_relationships_repo ON relationships(repository_id);
CREATE INDEX IF NOT EXISTS idx_chunks_symbol ON chunks(symbol_id);
CREATE INDEX IF NOT EXISTS idx_embeddings_repo ON embeddings(repository_id);
CREATE INDEX IF NOT EXISTS idx_runs_repo ON analysis_runs(repository_id);
";
