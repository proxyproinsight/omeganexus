-- Enhanced proxy validation with anonymity levels and stability scoring
ALTER TABLE proxies ADD COLUMN anonymity_level TEXT DEFAULT 'unknown';
ALTER TABLE proxies ADD COLUMN stability_score REAL DEFAULT 0.0;

-- Indexes for filtering by quality
CREATE INDEX idx_proxies_anonymity ON proxies(anonymity_level) WHERE anonymity_level != 'unknown';
CREATE INDEX idx_proxies_stability ON proxies(stability_score DESC) WHERE stability_score > 0.0;
