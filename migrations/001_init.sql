-- Proxy storage with comprehensive metrics
CREATE TABLE IF NOT EXISTS proxies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    host TEXT NOT NULL,
    port INTEGER NOT NULL,
    protocol TEXT NOT NULL DEFAULT 'http',
    country TEXT,
    city TEXT,
    latency_ms INTEGER,
    quality_score REAL DEFAULT 0.0,
    fraud_score REAL DEFAULT 0.0,
    dns_leak INTEGER DEFAULT 0,
    elite INTEGER DEFAULT 0,
    last_checked INTEGER NOT NULL,
    discovered_at INTEGER NOT NULL,
    source TEXT NOT NULL,
    active INTEGER DEFAULT 1,
    UNIQUE(host, port, protocol)
);

-- Source quality tracking for AI scoring
CREATE TABLE IF NOT EXISTS sources (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT UNIQUE NOT NULL,
    name TEXT,
    quality_score REAL DEFAULT 0.5,
    total_proxies INTEGER DEFAULT 0,
    working_proxies INTEGER DEFAULT 0,
    last_updated INTEGER,
    active INTEGER DEFAULT 1
);

-- Metrics for monitoring
CREATE TABLE IF NOT EXISTS metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER NOT NULL,
    total_proxies INTEGER DEFAULT 0,
    working_proxies INTEGER DEFAULT 0,
    avg_latency REAL DEFAULT 0.0,
    avg_quality REAL DEFAULT 0.0,
    sources_active INTEGER DEFAULT 0
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_proxies_active ON proxies(active);
CREATE INDEX IF NOT EXISTS idx_proxies_quality ON proxies(quality_score DESC);
CREATE INDEX IF NOT EXISTS idx_proxies_checked ON proxies(last_checked);
CREATE INDEX IF NOT EXISTS idx_sources_active ON sources(active);
CREATE INDEX IF NOT EXISTS idx_sources_quality ON sources(quality_score DESC);
