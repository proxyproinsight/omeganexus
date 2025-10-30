-- Phase 5: AI Predictive Source Scoring
-- Track historical success rates for intelligent source prioritization

ALTER TABLE sources ADD COLUMN total_fetches INTEGER DEFAULT 0;
ALTER TABLE sources ADD COLUMN successful_proxies INTEGER DEFAULT 0;
ALTER TABLE sources ADD COLUMN last_success_rate REAL DEFAULT 0.0;
ALTER TABLE sources ADD COLUMN consecutive_failures INTEGER DEFAULT 0;
ALTER TABLE sources ADD COLUMN last_fetch_timestamp INTEGER DEFAULT 0;

-- Index for prioritizing best sources
CREATE INDEX idx_sources_success_rate ON sources(last_success_rate DESC) WHERE active = 1;
CREATE INDEX idx_sources_priority ON sources(quality_score DESC, last_success_rate DESC) WHERE active = 1 AND consecutive_failures < 10;
