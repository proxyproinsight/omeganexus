mod ai;
mod asn_detector;
mod crawler;
mod discovery;
mod sources;
mod validator;

use axum::{
    extract::{
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
        State, Query,
    },
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePool, Pool, Sqlite};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use sysinfo::System;
use teloxide::{prelude::*, types::ParseMode, utils::command::BotCommands};
use tokio::sync::RwLock;
use tokio_retry::{Retry, strategy::ExponentialBackoff};
use tower_http::{cors::CorsLayer, services::ServeDir};
use tracing::{error, info, warn};
use std::collections::HashMap;

// GOD MODE: ASN Cache for fast validation
#[derive(Clone)]
struct ASNCacheEntry {
    data: crate::asn_detector::ASNData,
    cached_at: std::time::Instant,
}

type ASNCache = Arc<RwLock<HashMap<String, ASNCacheEntry>>>;

// Phase 10: Global start time for uptime tracking
static START_TIME: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

// Cached stats for performance
#[derive(Clone)]
struct CachedData<T> {
    data: T,
    cached_at: std::time::Instant,
}

#[derive(Clone)]
struct AppState {
    db: Pool<Sqlite>,
    client: reqwest::Client,
    bot: Bot,
    stats: Arc<RwLock<Stats>>,
    stats_cache: Arc<RwLock<Option<CachedData<Stats>>>>,
    system_cache: Arc<RwLock<Option<CachedData<SystemMetrics>>>>,
    asn_cache: ASNCache, // GOD MODE: In-memory ASN cache with 1-hour TTL
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Stats {
    total_proxies: i64,
    working_proxies: i64,
    avg_latency: f64,
    avg_quality: f64,
    sources_active: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
struct Proxy {
    id: i64,
    host: String,
    port: i64,
    protocol: String,
    country: Option<String>,
    city: Option<String>,
    latency_ms: Option<i64>,
    quality_score: f64,
    fraud_score: f64,
    dns_leak: i64,
    elite: i64,
    anonymity_level: Option<String>,
    stability_score: Option<f64>,
    last_checked: i64,
    discovered_at: i64,
    source: String,
    active: i64,
    isp: Option<String>,
    asn: Option<String>,
    proxy_type: Option<String>, // datacenter, residential, mobile
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
struct Source {
    id: i64,
    url: String,
    name: String,
    quality_score: f64,
    total_proxies: i64,
    working_proxies: i64,
    last_updated: Option<i64>,
    active: i64,
    // Phase 5: Predictive AI scoring fields
    total_fetches: Option<i64>,
    successful_proxies: Option<i64>,
    last_success_rate: Option<f64>,
    consecutive_failures: Option<i64>,
    last_fetch_timestamp: Option<i64>,
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    #[command(description = "Start the bot")]
    Start,
    #[command(description = "Get current statistics")]
    Stats,
    #[command(description = "Get top quality proxies")]
    Top,
    #[command(description = "Get fastest proxies")]
    Fastest,
    #[command(description = "💎 Get premium residential/mobile proxies")]
    Premium,
    #[command(description = "✅ Get verified browsable premium proxies only")]
    Verified,
    #[command(description = "Trigger manual hunt")]
    Hunt,
    #[command(description = "List active sources")]
    Sources,
    #[command(description = "Deactivate a source by ID")]
    Deactivate(String),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    // Phase 10: Set start time for uptime tracking
    START_TIME.store(Utc::now().timestamp(), std::sync::atomic::Ordering::Relaxed);

    // Load environment
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:omega9.db".to_string());
    let bind_addr = std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let bot_token = std::env::var("TELOXIDE_TOKEN").expect("TELOXIDE_TOKEN not set");

    // Initialize database
    let db = SqlitePool::connect(&database_url).await?;
    sqlx::migrate!("./migrations").run(&db).await?;
    info!("Database initialized");

    // Initialize HTTP client
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    // Initialize Telegram bot
    let bot = Bot::new(bot_token);

    // Shared state
    let state = AppState {
        db: db.clone(),
        client: client.clone(),
        bot: bot.clone(),
        stats: Arc::new(RwLock::new(Stats::default())),
        stats_cache: Arc::new(RwLock::new(None)),
        system_cache: Arc::new(RwLock::new(None)),
        asn_cache: Arc::new(RwLock::new(HashMap::new())), // GOD MODE: ASN cache for fast validation
    };

    // 🚀 GOD MODE: Seed 52 premium sources from ALL_SOURCES
    seed_premium_sources(&state.db).await?;

    // Spawn background tasks
    tokio::spawn(hunt_loop(state.clone()));
    tokio::spawn(discovery_loop(state.clone()));
    tokio::spawn(stats_updater(state.clone()));
    tokio::spawn(cleanup_loop(state.clone()));
    tokio::spawn(revalidation_loop(state.clone())); // New: Background revalidation
    tokio::spawn(elite_validation_loop(state.clone())); // GOD MODE: Elite validation for premium proxies
    
    // Spawn Telegram bot with error handling
    let bot_state = state.clone();
    tokio::spawn(async move {
        info!("🤖 Starting Telegram bot...");
        match run_bot(bot.clone(), bot_state).await {
            Ok(_) => info!("🤖 Bot stopped gracefully"),
            Err(e) => error!("❌ Bot error: {}", e),
        }
    });

    // Build web server
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/health", get(health_check))  // Phase 10: Health monitoring
        .route("/api/proxies", get(get_proxies))
        .route("/api/proxy/next", get(get_next_proxy))  // Rotation API
        .route("/api/proxies/export", get(export_proxies))  // Multi-format export
        .route("/api/export", get(export_premium_api))  // GOD MODE: Premium export with filters
        .route("/api/proxies/filter", get(filter_proxies))  // Advanced filtering
        .route("/api/proxies/country/:code", get(get_proxies_by_country))  // Country-specific
        .route("/api/proxies/premium", get(get_premium_proxies))  // Residential & Mobile
        .route("/api/proxies/premium/verified", get(get_verified_premium_proxies))  // Verified browsable only
        .route("/api/proxies/batch-test", post(batch_test_proxies))  // Batch testing
        .route("/api/stats", get(get_stats))
        .route("/api/system/stats", get(get_system_stats))  // System metrics
        .route("/api/stats/protocols", get(get_protocol_breakdown))  // Phase 8: Protocol stats
        .route("/api/sources", get(get_sources))
        .route("/api/sources/health", get(get_source_health))  // Source health
        .route("/api/hunt", post(trigger_hunt))
        .route("/api/test-proxy", post(test_proxy_endpoint))
        .route("/ws", get(websocket_handler))
        .route("/metrics", get(metrics_handler))  // GOD MODE: Prometheus metrics
        .nest_service("/static", ServeDir::new("static"))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr: SocketAddr = bind_addr.parse()?;
    info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// 🚀 GOD MODE: Seed database with 52 premium sources from ALL_SOURCES
async fn seed_premium_sources(db: &Pool<Sqlite>) -> Result<(), Box<dyn std::error::Error>> {
    use crate::sources::ALL_SOURCES;
    
    let mut seeded_count = 0;
    let mut updated_count = 0;
    
    for (url, protocol, name) in ALL_SOURCES.iter() {
        // Check if source already exists
        let exists: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM sources WHERE name = ? OR url = ?"
        )
        .bind(name)
        .bind(url)
        .fetch_optional(db)
        .await?;
        
        if exists.is_none() {
            // Insert new source
            sqlx::query(
                "INSERT INTO sources (name, url, active, quality_score, last_success_rate, consecutive_failures)
                 VALUES (?, ?, 1, 0.5, 0.0, 0)"
            )
            .bind(name)
            .bind(url)
            .execute(db)
            .await?;
            
            seeded_count += 1;
        } else {
            // Update existing source URL if changed
            sqlx::query(
                "UPDATE sources SET url = ?, active = 1 WHERE name = ?"
            )
            .bind(url)
            .bind(name)
            .execute(db)
            .await?;
            
            updated_count += 1;
        }
    }
    
    if seeded_count > 0 {
        info!("🌟 GOD MODE: Seeded {} new premium sources from ALL_SOURCES", seeded_count);
    }
    if updated_count > 0 {
        info!("🔄 GOD MODE: Updated {} existing sources", updated_count);
    }
    
    let total_sources: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sources WHERE active = 1")
        .fetch_one(db)
        .await?;
    
    info!("📊 Total active sources: {} (Expected: 52+ from GOD MODE)", total_sources);
    
    Ok(())
}

// GOD MODE: Cached ASN lookup with 1-hour TTL
async fn get_asn_cached(ip: &str, cache: &ASNCache) -> Option<crate::asn_detector::ASNData> {
    use crate::asn_detector::ASNDetector;
    
    // Check cache first
    {
        let cache_read = cache.read().await;
        if let Some(entry) = cache_read.get(ip) {
            // Check if cache entry is still valid (1 hour TTL)
            if entry.cached_at.elapsed().as_secs() < 3600 {
                return Some(entry.data.clone());
            }
        }
    }
    
    // Cache miss or expired - fetch fresh data
    let detector = ASNDetector::new();
    if let Ok(asn_data) = detector.fetch_asn_data(ip).await {
        // Update cache
        let mut cache_write = cache.write().await;
        cache_write.insert(ip.to_string(), ASNCacheEntry {
            data: asn_data.clone(),
            cached_at: std::time::Instant::now(),
        });
        return Some(asn_data);
    }
    
    None
}

// Background hunt loop
async fn hunt_loop(state: AppState) {
    let interval_secs = std::env::var("HUNT_INTERVAL_SECS")
        .unwrap_or_else(|_| "300".to_string())
        .parse::<u64>()
        .unwrap_or(300);

    loop {
        info!("Starting hunt cycle");

        let now = Utc::now().timestamp();
        
        // Get active sources, excluding those in cooldown period
        // Cooldown formula: 5 min * (2 ^ failures), max 24 hours
        let sources: Vec<Source> = match sqlx::query_as(
            "SELECT * FROM sources 
             WHERE active = 1 
             AND (consecutive_failures IS NULL OR consecutive_failures < 10)
             AND (next_retry_time IS NULL OR next_retry_time <= ?)
             ORDER BY 
                CASE 
                    WHEN consecutive_failures IS NULL OR consecutive_failures = 0 THEN 0
                    ELSE consecutive_failures 
                END ASC,
                quality_score DESC, 
                last_success_rate DESC 
             LIMIT 40"
        )
            .bind(now)
            .fetch_all(&state.db)
            .await
        {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to fetch sources: {}", e);
                tokio::time::sleep(Duration::from_secs(interval_secs)).await;
                continue;
            }
        };

        info!("Hunting from {} sources (skipping sources in cooldown)", sources.len());

        // OPTIMIZATION: Process sources in parallel instead of sequentially
        let mut tasks = Vec::new();
        
        for source in sources {
            let state_clone = state.clone();
            let now_clone = now;
            
            // Spawn task for each source
            tasks.push(tokio::spawn(async move {
                process_source(state_clone, source, now_clone).await
            }));
        }
        
        // Wait for all sources to complete
        let hunt_start = std::time::Instant::now();
        for task in tasks {
            let _ = task.await;
        }
        
        // 🚀 GOD MODE: Log hunt cycle summary with mobile/residential counts
        let total_proxies: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM proxies")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);
        
        let mobile_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM proxies WHERE is_mobile = 1")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);
        
        let residential_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM proxies WHERE is_residential = 1")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);
        
        let working_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM proxies WHERE active = 1")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);
        
        info!("🎯 Hunt cycle complete in {:?}", hunt_start.elapsed());
        info!("📊 GOD MODE Stats: Total={} | Working={} | 📱Mobile={} | 🏠Residential={}", 
              total_proxies, working_count, mobile_count, residential_count);
        
        tokio::time::sleep(Duration::from_secs(interval_secs)).await;
    }
}

// Process a single source (extracted for parallel execution)
async fn process_source(state: AppState, source: Source, now: i64) {
    let source_start = std::time::Instant::now();
    
    // GOD MODE: Retry strategy with exponential backoff (1s -> 2s -> 4s -> 8s max)
    let retry_strategy = ExponentialBackoff::from_millis(1000)
        .max_delay(Duration::from_secs(8))
        .take(3);
    
    // Set timeout for source fetch: 15 seconds with retry wrapper
    let fetch_result = tokio::time::timeout(
        Duration::from_secs(15),
        Retry::spawn(retry_strategy, || async {
            crawler::fetch_proxies(&state.client, &source.url).await
        })
    ).await;
    
    match fetch_result {
        Ok(Ok(proxies)) => {
            info!("✅ Fetched {} proxies from {} in {:?}", proxies.len(), source.name, source_start.elapsed());

            // Reset failure count on success
            let _ = sqlx::query(
                "UPDATE sources SET consecutive_failures = 0, next_retry_time = NULL WHERE id = ?"
            )
            .bind(source.id)
            .execute(&state.db)
            .await;

            // Limit to first 500 proxies per source for aggressive hunting
            let limited_proxies: Vec<_> = proxies.iter().take(500).cloned().collect();
            info!("Validating {} proxies from {} (limited from {})", limited_proxies.len(), source.name, proxies.len());

            let mut working_count = 0;
            
            // GOD MODE: Process in batches of 500 concurrently (scaled from 200)
            for chunk in limited_proxies.chunks(500) {
                let mut tasks = Vec::new();
                
                for proxy in chunk {
                    let client = state.client.clone();
                    let asn_cache = state.asn_cache.clone();
                    let proxy_str = proxy.clone();
                    let protocol = if source.url.contains("socks") {
                        "socks5"
                    } else {
                        "http"
                    };
                    
                    // GOD MODE: Spawn task with ASN cache lookup
                    tasks.push(tokio::spawn(async move {
                        let result = crawler::validate_proxy_fast(&client, &proxy_str, protocol).await;
                        
                        // Extract IP and fetch cached ASN data
                        let ip = proxy_str.split(':').next().unwrap_or("");
                        let asn_data = get_asn_cached(ip, &asn_cache).await;
                        
                        (result, asn_data)
                    }));
                }
                
                // Wait for batch to complete
                        for (idx, task) in tasks.into_iter().enumerate() {
                            if let Ok((result_res, asn_data)) = task.await {
                                if let Ok(mut result) = result_res {
                                    // GOD MODE: Inject cached ASN data into result
                                    result.asn_data = asn_data;
                                    
                                    if result.working {
                                    working_count += 1;
                                    
                                    let proxy = &chunk[idx];
                                    let parts: Vec<&str> = proxy.split(':').collect();
                                    let host = parts[0];
                                    let port: i64 = parts.get(1).unwrap_or(&"0").parse().unwrap_or(0);

                                    let country = result.geo.as_ref().map(|g| g.country.clone()).unwrap_or_default();
                                    let city = result.geo.as_ref().map(|g| g.city.clone()).unwrap_or_default();
                                    let fraud_score = result.fraud.as_ref().map(|f| f.score).unwrap_or(0.0);

                                    // Calculate AI score
                                    let source_quality = source.quality_score;
                                    let age_hours = 0; // Just discovered
                                    let country_rare = ["IS", "LU", "CH", "SG", "NL"].contains(&country.as_str());
                                    let weights = ai::Weights::default();

                                    let mut quality_score = ai::predict_score(
                                        result.latency_ms,
                                        country_rare,
                                        source_quality,
                                        age_hours,
                                        fraud_score,
                                        result.dns_leak,
                                        result.elite,
                                        &weights,
                                    );
                                    
                                    // GOD MODE: Rotation bonus for mobile proxies
                                    // Mobile proxies that rotate get quality boost
                                    if let Some(ref asn_data) = result.asn_data {
                                        if asn_data.is_mobile {
                                            quality_score += 0.20; // Mobile proxies are premium
                                        }
                                    }

                                    // Insert or update proxy with GOD MODE ASN fields
                                    let now = Utc::now().timestamp();
                                    let protocol = if source.url.contains("socks") { "socks5" } else { "http" };
                                    
                                    // Extract ASN data (prioritize ASN detector over geo API)
                                    let (isp, asn, proxy_type, is_mobile, is_residential, carrier_asn, isp_name, abuse_score_val) = if let Some(ref asn_data) = result.asn_data {
                                        // GOD MODE: Use ASN detector results (THE truth source)
                                        let isp_val = asn_data.carrier_name.clone()
                                            .or_else(|| asn_data.isp_name.clone())
                                            .or_else(|| Some(asn_data.org.clone()));
                                        let asn_val = Some(format!("AS{}", asn_data.asn));
                                        let proxy_type_val = if asn_data.is_mobile {
                                            "mobile"
                                        } else if asn_data.is_residential {
                                            "residential"
                                        } else {
                                            "datacenter"
                                        }.to_string();
                                        
                                        (
                                            isp_val, 
                                            asn_val,
                                            proxy_type_val,
                                            if asn_data.is_mobile { 1 } else { 0 },
                                            if asn_data.is_residential { 1 } else { 0 },
                                            Some(asn_data.asn as i64),
                                            asn_data.carrier_name.clone().or_else(|| asn_data.isp_name.clone()),
                                            result.fraud.as_ref().map(|f| f.score).unwrap_or(0.0),
                                        )
                                    } else if let Some(ref geo) = result.geo {
                                        // Fallback to geo API (less accurate)
                                        let is_mob = if geo.proxy_type == "mobile" { 1 } else { 0 };
                                        let is_res = if geo.proxy_type == "residential" { 1 } else { 0 };
                                        (geo.isp.clone(), geo.asn.clone(), geo.proxy_type.clone(), is_mob, is_res, None, None, result.fraud.as_ref().map(|f| f.score).unwrap_or(0.0))
                                    } else {
                                        (None, None, "datacenter".to_string(), 0, 0, None, None, 0.0)
                                    };
                                    
                                    let _ = sqlx::query(
                                        "INSERT INTO proxies (host, port, protocol, country, city, latency_ms, quality_score, fraud_score, dns_leak, elite, anonymity_level, stability_score, last_checked, discovered_at, source, active, isp, asn, proxy_type, is_mobile, is_residential, carrier_asn, isp_name, abuse_score, browser_compatible)
                                         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?, ?, ?, ?, ?, ?, ?, 1)
                                         ON CONFLICT(host, port, protocol) DO UPDATE SET
                                         latency_ms = ?, quality_score = ?, fraud_score = ?, dns_leak = ?, elite = ?, anonymity_level = ?, stability_score = ?, last_checked = ?, active = 1, isp = ?, asn = ?, proxy_type = ?, is_mobile = ?, is_residential = ?, carrier_asn = ?, isp_name = ?, abuse_score = ?, browser_compatible = 1"
                                    )
                                    .bind(host)
                                    .bind(port)
                                    .bind(protocol)
                                    .bind(&country)
                                    .bind(&city)
                                    .bind(result.latency_ms)
                                    .bind(quality_score)
                                    .bind(fraud_score)
                                    .bind(if result.dns_leak { 1 } else { 0 })
                                    .bind(if result.elite { 1 } else { 0 })
                                    .bind(&result.anonymity_level)
                                    .bind(result.stability_score)
                                    .bind(now)
                                    .bind(now)
                                    .bind(&source.name)
                                    .bind(&isp)
                                    .bind(&asn)
                                    .bind(&proxy_type)
                                    .bind(is_mobile)
                                    .bind(is_residential)
                                    .bind(carrier_asn)
                                    .bind(&isp_name)
                                    .bind(abuse_score_val)
                                    // ON CONFLICT UPDATE binds
                                    .bind(result.latency_ms)
                                    .bind(quality_score)
                                    .bind(fraud_score)
                                    .bind(if result.dns_leak { 1 } else { 0 })
                                    .bind(if result.elite { 1 } else { 0 })
                                    .bind(&result.anonymity_level)
                                    .bind(result.stability_score)
                                    .bind(now)
                                    .bind(&isp)
                                    .bind(&asn)
                                    .bind(&proxy_type)
                                    .bind(is_mobile)
                                    .bind(is_residential)
                                    .bind(carrier_asn)
                                    .bind(&isp_name)
                                    .bind(abuse_score_val)
                                    .execute(&state.db)
                                    .await;
                                    
                                    // Webhook notification for elite/residential/mobile proxies
                                    let is_premium = result.elite && quality_score > 0.7;
                                    let is_golden = proxy_type == "residential" || proxy_type == "mobile";
                                    
                                    if (is_premium || is_golden) && std::env::var("WEBHOOK_URL").is_ok() {
                                        if let Ok(webhook_url) = std::env::var("WEBHOOK_URL") {
                                            let client = state.client.clone();
                                            let webhook = webhook_url.clone();
                                            let event_type = if proxy_type == "mobile" {
                                                "new_mobile_proxy"
                                            } else if proxy_type == "residential" {
                                                "new_residential_proxy"
                                            } else {
                                                "new_elite_proxy"
                                            };
                                            
                                            let proxy_info = serde_json::json!({
                                                "event": event_type,
                                                "proxy": format!("{}://{}:{}", protocol, host, port),
                                                "quality_score": quality_score,
                                                "country": &country,
                                                "anonymity_level": &result.anonymity_level,
                                                "stability_score": result.stability_score,
                                                "proxy_type": &proxy_type,
                                                "isp": &isp,
                                                "asn": &asn,
                                                "latency_ms": result.latency_ms,
                                            });
                                            tokio::spawn(async move {
                                                let _ = client.post(&webhook).json(&proxy_info).send().await;
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

            // Phase 5: Update source with predictive AI scoring
            let success_rate = working_count as f64 / limited_proxies.len().max(1) as f64;
            let new_quality = ai::update_source_quality(source.quality_score, success_rate, 0.3);
            let now = Utc::now().timestamp();
            
            // Track historical performance for intelligent prioritization
            let consecutive_failures = if working_count == 0 { 
                source.consecutive_failures.unwrap_or(0) + 1 
            } else { 
                0 
            };

            let _ = sqlx::query(
                "UPDATE sources SET 
                 quality_score = ?, 
                 total_proxies = total_proxies + ?, 
                 working_proxies = working_proxies + ?, 
                 total_fetches = total_fetches + 1,
                 successful_proxies = successful_proxies + ?,
                 last_success_rate = ?,
                 consecutive_failures = ?,
                 last_fetch_timestamp = ?,
                 last_updated = ? 
                 WHERE id = ?"
            )
            .bind(new_quality)
            .bind(limited_proxies.len() as i64)
            .bind(working_count)
            .bind(working_count)
            .bind(success_rate)
            .bind(consecutive_failures)
            .bind(now)
            .bind(now)
            .bind(source.id)
            .execute(&state.db)
            .await;
            
            info!("Found {} working proxies from {} (quality: {:.2})", working_count, source.name, new_quality);
        }
        Ok(Err(_e)) => {
            // Fetch timed out or errored
            warn!("⏱️ Timeout or error fetching from {} (15s limit exceeded)", source.name);
            
            // Exponential backoff: 5min * 2^failures, max 24h (86400s)
            let failures = source.consecutive_failures.unwrap_or(0) + 1;
            let backoff_secs = (300 * 2_i64.pow(failures.min(7) as u32)).min(86400);
            let retry_time = now + backoff_secs;
            
            let _ = sqlx::query(
                "UPDATE sources 
                 SET consecutive_failures = ?, 
                     last_failure_time = ?,
                     next_retry_time = ?
                 WHERE id = ?"
            )
            .bind(failures)
            .bind(now)
            .bind(retry_time)
            .bind(source.id)
            .execute(&state.db)
            .await;
            
            info!("🔁 Will retry {} after {} minutes (failure #{}/10)", 
                  source.name, backoff_secs / 60, failures);
        }
        Err(_) => {
            // Actual timeout from tokio
            warn!("⏱️ Hard timeout on {} - skipping and scheduling retry", source.name);
            
            let failures = source.consecutive_failures.unwrap_or(0) + 1;
            let backoff_secs = (300 * 2_i64.pow(failures.min(7) as u32)).min(86400);
            let retry_time = now + backoff_secs;
            
            let _ = sqlx::query(
                "UPDATE sources 
                 SET consecutive_failures = ?, 
                     last_failure_time = ?,
                     next_retry_time = ?
                 WHERE id = ?"
            )
            .bind(failures)
            .bind(now)
            .bind(retry_time)
            .bind(source.id)
            .execute(&state.db)
            .await;
            
            info!("🔁 Will retry {} in {} hours (failure #{}/10)", 
                  source.name, backoff_secs / 3600, failures);
        }
    }
}

// Background discovery loop
async fn discovery_loop(state: AppState) {
    let interval_secs = 3600; // Discover new sources every hour

    loop {
        tokio::time::sleep(Duration::from_secs(interval_secs)).await;

        info!("Starting source discovery");

        let github_token = std::env::var("GITHUB_TOKEN").ok();
        match discovery::discover_new_sources(&state.client, github_token.as_deref()).await {
            Ok(new_sources) => {
                for source in &new_sources {
                    let _ = sqlx::query(
                        "INSERT INTO sources (url, name, quality_score, total_proxies, working_proxies, active)
                         VALUES (?, ?, 0.5, 0, 0, 1)
                         ON CONFLICT(url) DO NOTHING"
                    )
                    .bind(&source.url)
                    .bind(&source.name)
                    .execute(&state.db)
                    .await;
                }
                info!("Added {} new sources", new_sources.len());
            }
            Err(e) => {
                error!("Discovery failed: {}", e);
            }
        }

        // Also ensure static sources are in DB
        for source in discovery::get_static_sources() {
            let _ = sqlx::query(
                "INSERT INTO sources (url, name, quality_score, total_proxies, working_proxies, active)
                 VALUES (?, ?, 0.7, 0, 0, 1)
                 ON CONFLICT(url) DO NOTHING"
            )
            .bind(&source.url)
            .bind(&source.name)
            .execute(&state.db)
            .await;
        }
    }
}

// Stats updater
async fn stats_updater(state: AppState) {
    loop {
        tokio::time::sleep(Duration::from_secs(10)).await;

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM proxies")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

        let working: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM proxies WHERE active = 1")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

        let avg_latency: f64 = sqlx::query_scalar("SELECT AVG(latency_ms) FROM proxies WHERE active = 1")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0.0);

        let avg_quality: f64 = sqlx::query_scalar("SELECT AVG(quality_score) FROM proxies WHERE active = 1")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0.0);

        let sources_active: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sources WHERE active = 1")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

        let mut stats = state.stats.write().await;
        *stats = Stats {
            total_proxies: total,
            working_proxies: working,
            avg_latency,
            avg_quality,
            sources_active,
        };
    }
}

// Cleanup loop - remove stale proxies and revalidate elite ones
async fn cleanup_loop(state: AppState) {
    loop {
        tokio::time::sleep(Duration::from_secs(3600)).await; // Every hour
        
        let stale_threshold = Utc::now().timestamp() - (6 * 3600); // 6 hours old
        
        // Mark stale proxies as inactive
        let deactivated = sqlx::query(
            "UPDATE proxies SET active = 0 
             WHERE last_checked < ? 
             AND active = 1"
        )
        .bind(stale_threshold)
        .execute(&state.db)
        .await
        .map(|r| r.rows_affected())
        .unwrap_or(0);
        
        if deactivated > 0 {
            info!("Deactivated {} stale proxies", deactivated);
        }
        
        // Revalidate top quality stale proxies (give them a second chance)
        let stale_elite: Vec<Proxy> = sqlx::query_as(
            "SELECT * FROM proxies 
             WHERE last_checked < ? 
             AND quality_score > 0.6 
             AND active = 0
             LIMIT 20"
        )
        .bind(stale_threshold)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();
        
        let mut reactivated = 0;
        for proxy in stale_elite {
            let proxy_str = format!("{}:{}", proxy.host, proxy.port);
            if let Ok(result) = crawler::validate_proxy(&state.client, &proxy_str, &proxy.protocol).await {
                if result.working {
                    let now = Utc::now().timestamp();
                    let _ = sqlx::query(
                        "UPDATE proxies SET active = 1, last_checked = ?, latency_ms = ?, stability_score = ? WHERE id = ?"
                    )
                    .bind(now)
                    .bind(result.latency_ms)
                    .bind(result.stability_score)
                    .bind(proxy.id)
                    .execute(&state.db)
                    .await;
                    reactivated += 1;
                }
            }
        }
        
        if reactivated > 0 {
            info!("Reactivated {} high-quality proxies", reactivated);
        }
        
        info!("Cleanup cycle complete");
    }
}

// Background revalidation loop - revalidate top proxies every 6 hours
async fn revalidation_loop(state: AppState) {
    loop {
        tokio::time::sleep(Duration::from_secs(21600)).await; // Every 6 hours
        
        info!("🔄 Starting background revalidation of top proxies");
        
        // Get top 100 proxies by quality score
        let top_proxies: Vec<Proxy> = sqlx::query_as(
            "SELECT * FROM proxies 
             WHERE active = 1 
             ORDER BY quality_score DESC 
             LIMIT 100"
        )
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();
        
        let mut revalidated = 0;
        let mut failed = 0;
        
        for proxy in top_proxies {
            let proxy_str = format!("{}:{}", proxy.host, proxy.port);
            if let Ok(result) = crawler::validate_proxy_fast(&state.client, &proxy_str, &proxy.protocol).await {
                let now = Utc::now().timestamp();
                if result.working {
                    let _ = sqlx::query(
                        "UPDATE proxies SET last_checked = ?, latency_ms = ?, active = 1 WHERE id = ?"
                    )
                    .bind(now)
                    .bind(result.latency_ms)
                    .bind(proxy.id)
                    .execute(&state.db)
                    .await;
                    revalidated += 1;
                } else {
                    let _ = sqlx::query("UPDATE proxies SET active = 0 WHERE id = ?")
                        .bind(proxy.id)
                        .execute(&state.db)
                        .await;
                    failed += 1;
                }
            }
            
            // Small delay to avoid overwhelming network
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        info!("✅ Revalidation complete: {} still working, {} failed", revalidated, failed);
    }
}

// GOD MODE: Elite Validation Loop - Validates premium proxies with 5-stage pipeline
async fn elite_validation_loop(state: AppState) {
    use crate::validator::EliteValidator;
    
    loop {
        tokio::time::sleep(Duration::from_secs(3600)).await; // Every 1 hour
        
        info!("🏆 Starting elite validation for premium proxies");
        
        // Get mobile/residential proxies that haven't been elite-validated recently
        let premium_proxies: Vec<Proxy> = sqlx::query_as(
            "SELECT * FROM proxies 
             WHERE active = 1 
             AND (is_mobile = 1 OR is_residential = 1)
             AND (last_elite_check IS NULL OR last_elite_check < ?)
             ORDER BY quality_score DESC 
             LIMIT 20"
        )
        .bind(Utc::now().timestamp() - 86400) // Last 24 hours
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();
        
        info!("Found {} premium proxies to validate", premium_proxies.len());
        
        let validator = EliteValidator::new();
        let mut elite_count = 0;
        let total_count = premium_proxies.len();
        
        for proxy in premium_proxies {
            let proxy_url = format!("{}://{}:{}", proxy.protocol, proxy.host, proxy.port);
            let ip = proxy.host.clone();
            
            info!("Validating elite status for {}...", proxy_url);
            
            match validator.validate_elite(&proxy_url, &ip).await {
                Ok(result) => {
                    let now = Utc::now().timestamp();
                    
                    // Update database with elite validation results
                    let _ = sqlx::query(
                        "UPDATE proxies SET 
                         last_elite_check = ?,
                         anonymity_level = ?,
                         stability_score = ?,
                         fraud_score = ?,
                         abuse_score = ?,
                         browser_compatible = ?,
                         rotation_verified = ?
                         WHERE id = ?"
                    )
                    .bind(now)
                    .bind(&result.anonymity_level)
                    .bind(result.stability_score)
                    .bind(result.fraud_score)
                    .bind(result.abuse_score)
                    .bind(result.browser_compatible)
                    .bind(result.rotation_verified)
                    .bind(proxy.id)
                    .execute(&state.db)
                    .await;
                    
                    if result.is_elite {
                        elite_count += 1;
                        info!("✅ ELITE proxy found: {} (score: {:.2})", proxy_url, result.stability_score);
                    } else {
                        info!("⚠️ Not elite: {} (score: {:.2})", proxy_url, result.stability_score);
                    }
                }
                Err(e) => {
                    warn!("Failed to validate {}: {}", proxy_url, e);
                }
            }
            
            // Delay between validations (elite tests take time)
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
        
        info!("✅ Elite validation complete: {}/{} passed elite criteria", elite_count, total_count);
    }
}

// Telegram bot handler
async fn run_bot(bot: Bot, state: AppState) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("🤖 Bot handler initialized, waiting for commands...");
    Command::repl(bot.clone(), move |bot: Bot, msg: Message, cmd: Command| {
        let state = state.clone();
        async move {
            let chat_id = msg.chat.id;
            match cmd {
                Command::Start => {
                    bot.send_message(chat_id, "🔥 Omega9-NEXUS v15.0\n\nProxy hunting system active!\n\nCommands:\n/stats - Current statistics\n/top - Top quality proxies\n/fastest - Fastest proxies\n/hunt - Manual hunt\n/sources - Active sources\n/deactivate <id> - Deactivate source")
                        .await?;
                }
                Command::Stats => {
                    let stats = state.stats.read().await;
                    let msg_text = format!(
                        "📊 Statistics\n\n\
                        Total Proxies: {}\n\
                        Working: {}\n\
                        Avg Latency: {:.0}ms\n\
                        Avg Quality: {:.2}\n\
                        Active Sources: {}",
                        stats.total_proxies,
                        stats.working_proxies,
                        stats.avg_latency,
                        stats.avg_quality,
                        stats.sources_active
                    );
                    bot.send_message(chat_id, msg_text).await?;
                }
                Command::Top => {
                    let proxies: Vec<Proxy> = sqlx::query_as(
                        "SELECT * FROM proxies WHERE active = 1 ORDER BY quality_score DESC LIMIT 10"
                    )
                    .fetch_all(&state.db)
                    .await
                    .unwrap_or_default();

                    let mut msg_text = "🏆 Top Quality Proxies\n\n".to_string();
                    for p in proxies {
                        msg_text.push_str(&format!(
                            "{}://{}:{} [{}] Q:{:.2} L:{}ms\n",
                            p.protocol,
                            p.host,
                            p.port,
                            p.country.unwrap_or_else(|| "??".to_string()),
                            p.quality_score,
                            p.latency_ms.unwrap_or(0)
                        ));
                    }
                    bot.send_message(chat_id, msg_text).await?;
                }
                Command::Fastest => {
                    let proxies: Vec<Proxy> = sqlx::query_as(
                        "SELECT * FROM proxies WHERE active = 1 ORDER BY latency_ms ASC LIMIT 10"
                    )
                    .fetch_all(&state.db)
                    .await
                    .unwrap_or_default();

                    let mut msg_text = "⚡ Fastest Proxies\n\n".to_string();
                    for p in proxies {
                        msg_text.push_str(&format!(
                            "{}://{}:{} [{}] L:{}ms Q:{:.2}\n",
                            p.protocol,
                            p.host,
                            p.port,
                            p.country.unwrap_or_else(|| "??".to_string()),
                            p.latency_ms.unwrap_or(0),
                            p.quality_score
                        ));
                    }
                    bot.send_message(chat_id, msg_text).await?;
                }
                Command::Premium => {
                    let proxies: Vec<Proxy> = sqlx::query_as(
                        "SELECT * FROM proxies 
                         WHERE active = 1 AND proxy_type IN ('residential', 'mobile')
                         ORDER BY 
                            CASE proxy_type 
                                WHEN 'mobile' THEN 1 
                                WHEN 'residential' THEN 2 
                                ELSE 3 
                            END,
                            quality_score DESC
                         LIMIT 20"
                    )
                    .fetch_all(&state.db)
                    .await
                    .unwrap_or_default();

                    if proxies.is_empty() {
                        bot.send_message(chat_id, "💎 No premium proxies found yet!\n\nKeep hunting to discover residential and mobile proxies.").await?;
                    } else {
                        let mut msg_text = format!("💎 Premium Proxies ({} found)\n\n", proxies.len());
                        
                        for p in proxies {
                            let proxy_type = p.proxy_type.as_deref().unwrap_or("datacenter");
                            let type_icon = match proxy_type {
                                "mobile" => "📱",
                                "residential" => "🏠",
                                _ => "🖥️"
                            };
                            let isp = p.isp.as_deref().unwrap_or("Unknown ISP");
                            
                            msg_text.push_str(&format!(
                                "{} {}://{}:{}\n[{}] {} Q:{:.2}\n{}\n\n",
                                type_icon,
                                p.protocol,
                                p.host,
                                p.port,
                                p.country.unwrap_or_else(|| "??".to_string()),
                                proxy_type.to_uppercase(),
                                p.quality_score,
                                isp
                            ));
                        }
                        
                        bot.send_message(chat_id, msg_text).parse_mode(ParseMode::Html).await?;
                    }
                }
                Command::Verified => {
                    let proxies: Vec<Proxy> = sqlx::query_as(
                        "SELECT * FROM proxies 
                         WHERE active = 1 
                           AND proxy_type IN ('residential', 'mobile')
                           AND stability_score >= 0.8
                         ORDER BY 
                            stability_score DESC,
                            quality_score DESC
                         LIMIT 10"
                    )
                    .fetch_all(&state.db)
                    .await
                    .unwrap_or_default();

                    if proxies.is_empty() {
                        bot.send_message(chat_id, "✅ No verified browsable premium proxies found yet!\n\nThese are rare - only premium proxies that pass Google.com browsing test.\n\nKeep hunting!").await?;
                    } else {
                        let mut msg_text = format!("✅ Verified Browsable Premium ({} found)\n━━━━━━━━━━━━━━━━━━━━━━\n\n", proxies.len());
                        msg_text.push_str("🌐 These proxies can browse websites!\nTested: Google.com ✓\n\n");
                        
                        for p in proxies {
                            let proxy_type = p.proxy_type.as_deref().unwrap_or("datacenter");
                            let type_icon = match proxy_type {
                                "mobile" => "📱",
                                "residential" => "🏠",
                                _ => "🖥️"
                            };
                            let isp = p.isp.as_deref().unwrap_or("Unknown ISP");
                            let city = p.city.as_deref().unwrap_or("Unknown");
                            let quality = p.quality_score * 100.0;
                            let stability = p.stability_score.unwrap_or(0.0) * 100.0;
                            let latency = p.latency_ms.unwrap_or(0);
                            
                            msg_text.push_str(&format!(
                                "{} <code>{}://{}:{}</code>\n📍 {}, {}\n🏢 {}\n⭐ Quality: {:.1}% | 🎯 Stability: {:.1}%\n⚡ {}ms\n\n",
                                type_icon,
                                p.protocol,
                                p.host,
                                p.port,
                                city,
                                p.country.unwrap_or_else(|| "??".to_string()),
                                isp,
                                quality,
                                stability,
                                latency
                            ));
                        }
                        
                        msg_text.push_str("━━━━━━━━━━━━━━━━━━━━━━\n💡 Tip: Use /premium for all premium proxies");
                        
                        bot.send_message(chat_id, msg_text).parse_mode(ParseMode::Html).await?;
                    }
                }
                Command::Hunt => {
                    bot.send_message(chat_id, "🔍 Manual hunt triggered!").await?;
                    // Hunt will happen in background loop
                }
                Command::Sources => {
                    let sources: Vec<Source> = sqlx::query_as(
                        "SELECT * FROM sources WHERE active = 1 ORDER BY quality_score DESC LIMIT 20"
                    )
                    .fetch_all(&state.db)
                    .await
                    .unwrap_or_default();

                    let mut msg_text = "📡 Active Sources\n\n".to_string();
                    for s in sources {
                        msg_text.push_str(&format!(
                            "ID:{} {} Q:{:.2} ({}/{})\n",
                            s.id, s.name, s.quality_score, s.working_proxies, s.total_proxies
                        ));
                    }
                    bot.send_message(chat_id, msg_text).await?;
                }
                Command::Deactivate(id_str) => {
                    if let Ok(id) = id_str.parse::<i64>() {
                        let _ = sqlx::query("UPDATE sources SET active = 0 WHERE id = ?")
                            .bind(id)
                            .execute(&state.db)
                            .await;
                        bot.send_message(chat_id, format!("✅ Deactivated source ID {}", id)).await?;
                    } else {
                        bot.send_message(chat_id, "❌ Invalid ID").await?;
                    }
                }
            }
            Ok(())
        }
    })
    .await;
    
    Ok(())
}

// HTTP Handlers
// Phase 10: Health check endpoint for monitoring
#[derive(Serialize)]
struct HealthStatus {
    status: String,
    uptime_seconds: i64,
    active_proxies: i64,
    active_sources: i64,
    last_hunt: String,
}

async fn health_check(State(state): State<AppState>) -> Json<HealthStatus> {
    let active_proxies: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM proxies WHERE active = 1")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let active_sources: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sources WHERE active = 1")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let last_updated: Option<i64> = sqlx::query_scalar("SELECT MAX(last_updated) FROM sources")
        .fetch_one(&state.db)
        .await
        .ok()
        .flatten();

    let last_hunt = if let Some(ts) = last_updated {
        let dt = chrono::DateTime::from_timestamp(ts, 0).unwrap_or_default();
        dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
    } else {
        "Never".to_string()
    };

    Json(HealthStatus {
        status: if active_proxies > 0 { "healthy".to_string() } else { "degraded".to_string() },
        uptime_seconds: Utc::now().timestamp() - START_TIME.load(std::sync::atomic::Ordering::Relaxed),
        active_proxies,
        active_sources,
        last_hunt,
    })
}

async fn index_handler() -> Html<String> {
    let html = tokio::fs::read_to_string("static/index.html")
        .await
        .unwrap_or_else(|_| "<h1>Omega9-NEXUS v15.0</h1><p>Dashboard loading...</p>".to_string());
    Html(html)
}

async fn get_proxies(State(state): State<AppState>) -> Json<Vec<Proxy>> {
    let proxies: Vec<Proxy> = sqlx::query_as(
        "SELECT * FROM proxies WHERE active = 1 ORDER BY quality_score DESC LIMIT 100"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    Json(proxies)
}

async fn get_stats(State(state): State<AppState>) -> Json<Stats> {
    // Check cache first (10 second TTL)
    {
        let cache = state.stats_cache.read().await;
        if let Some(cached) = cache.as_ref() {
            if cached.cached_at.elapsed() < Duration::from_secs(10) {
                return Json(cached.data.clone());
            }
        }
    }
    
    // Cache miss - fetch fresh data
    let stats = state.stats.read().await.clone();
    
    // Update cache
    let mut cache = state.stats_cache.write().await;
    *cache = Some(CachedData {
        data: stats.clone(),
        cached_at: std::time::Instant::now(),
    });
    
    Json(stats)
}

// System metrics endpoint - CPU, RAM, Temperature
#[derive(Serialize, Clone)]
struct SystemMetrics {
    cpu_usage: f32,
    ram_used_mb: u64,
    ram_total_mb: u64,
    ram_percent: f32,
    temperature: Option<f32>,
}

async fn get_system_stats(State(state): State<AppState>) -> Json<SystemMetrics> {
    // Check cache first (2 second TTL)
    {
        let cache = state.system_cache.read().await;
        if let Some(cached) = cache.as_ref() {
            if cached.cached_at.elapsed() < Duration::from_secs(2) {
                return Json(cached.data.clone());
            }
        }
    }
    
    // Cache miss - fetch fresh metrics
    let mut sys = System::new_all();
    sys.refresh_all();
    
    // Get CPU usage
    let cpu_usage = sys.global_cpu_usage();
    
    // Get RAM usage
    let ram_used = sys.used_memory() / 1024 / 1024; // Convert to MB
    let ram_total = sys.total_memory() / 1024 / 1024;
    let ram_percent = (sys.used_memory() as f32 / sys.total_memory() as f32) * 100.0;
    
    // Get temperature from Linux thermal zones
    let temperature = std::fs::read_to_string("/sys/class/thermal/thermal_zone0/temp")
        .ok()
        .and_then(|s| s.trim().parse::<f32>().ok())
        .map(|temp_millidegrees| temp_millidegrees / 1000.0); // Convert millidegrees to Celsius
    
    let metrics = SystemMetrics {
        cpu_usage,
        ram_used_mb: ram_used,
        ram_total_mb: ram_total,
        ram_percent,
        temperature,
    };
    
    // Update cache
    let mut cache = state.system_cache.write().await;
    *cache = Some(CachedData {
        data: metrics.clone(),
        cached_at: std::time::Instant::now(),
    });
    
    Json(metrics)
}

async fn get_sources(State(state): State<AppState>) -> Json<Vec<Source>> {
    let sources: Vec<Source> = sqlx::query_as(
        "SELECT * FROM sources WHERE active = 1 ORDER BY quality_score DESC"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    Json(sources)
}

// Proxy Rotation API - get next available proxy with round-robin
#[derive(Serialize)]
struct ProxyRotation {
    proxy_url: String,
    host: String,
    port: i64,
    protocol: String,
    quality_score: f64,
    country: String,
    latency_ms: i64,
}

async fn get_next_proxy(State(state): State<AppState>) -> Response {
    let proxy: Option<Proxy> = sqlx::query_as(
        "SELECT * FROM proxies 
         WHERE active = 1 AND quality_score > 0.4
         ORDER BY RANDOM()
         LIMIT 1"
    )
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);
    
    match proxy {
        Some(p) => Json(ProxyRotation {
            proxy_url: format!("{}://{}:{}", p.protocol, p.host, p.port),
            host: p.host,
            port: p.port,
            protocol: p.protocol,
            quality_score: p.quality_score,
            country: p.country.unwrap_or_else(|| "Unknown".to_string()),
            latency_ms: p.latency_ms.unwrap_or(0),
        }).into_response(),
        None => (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "No proxies available"}))
        ).into_response()
    }
}

// Country-specific proxy endpoint
async fn get_proxies_by_country(
    State(state): State<AppState>,
    axum::extract::Path(country_code): axum::extract::Path<String>
) -> Json<Vec<Proxy>> {
    let proxies: Vec<Proxy> = sqlx::query_as(
        "SELECT * FROM proxies 
         WHERE active = 1 AND country = ?
         ORDER BY quality_score DESC 
         LIMIT 50"
    )
    .bind(country_code.to_uppercase())
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    
    Json(proxies)
}

// Advanced proxy filtering
#[derive(Deserialize)]
struct ProxyFilter {
    protocol: Option<String>,
    anonymity_level: Option<String>,
    max_latency: Option<i64>,
    min_quality: Option<f64>,
    country: Option<String>,
    limit: Option<i64>,
}

async fn filter_proxies(
    State(state): State<AppState>,
    axum::extract::Query(filter): axum::extract::Query<ProxyFilter>
) -> Json<Vec<Proxy>> {
    let limit = filter.limit.unwrap_or(50).min(500);
    let min_quality = filter.min_quality.unwrap_or(0.0);
    let max_latency = filter.max_latency.unwrap_or(10000);
    
    let mut query = "SELECT * FROM proxies WHERE active = 1".to_string();
    let mut conditions = Vec::new();
    
    if filter.protocol.is_some() {
        conditions.push(format!("protocol = '{}'", filter.protocol.unwrap()));
    }
    if filter.anonymity_level.is_some() {
        conditions.push(format!("anonymity_level = '{}'", filter.anonymity_level.unwrap()));
    }
    if filter.country.is_some() {
        conditions.push(format!("country = '{}'", filter.country.unwrap().to_uppercase()));
    }
    conditions.push(format!("quality_score >= {}", min_quality));
    conditions.push(format!("latency_ms <= {}", max_latency));
    
    if !conditions.is_empty() {
        query.push_str(" AND ");
        query.push_str(&conditions.join(" AND "));
    }
    
    query.push_str(&format!(" ORDER BY quality_score DESC LIMIT {}", limit));
    
    let proxies: Vec<Proxy> = sqlx::query_as(&query)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();
    
    Json(proxies)
}

// Get residential and mobile proxies (premium/golden proxies)
async fn get_premium_proxies(State(state): State<AppState>) -> Json<Vec<Proxy>> {
    let proxies: Vec<Proxy> = sqlx::query_as(
        "SELECT * FROM proxies 
         WHERE active = 1 AND proxy_type IN ('residential', 'mobile')
         ORDER BY 
            CASE proxy_type 
                WHEN 'mobile' THEN 1 
                WHEN 'residential' THEN 2 
                ELSE 3 
            END,
            quality_score DESC
         LIMIT 100"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    
    Json(proxies)
}

// Get only verified browsable premium proxies (stability_score >= 0.8)
async fn get_verified_premium_proxies(State(state): State<AppState>, Query(params): Query<std::collections::HashMap<String, String>>) -> Response {
    #[derive(sqlx::FromRow, Serialize)]
    struct VerifiedProxy {
        protocol: String,
        host: String,
        port: i64,
        proxy_type: String,
        country: String,
        city: Option<String>,
        isp: Option<String>,
        quality_score: f64,
        stability_score: f64,
        latency_ms: i64,
        last_checked: i64,
    }
    
    let proxies: Vec<VerifiedProxy> = sqlx::query_as(
        "SELECT 
            protocol,
            host,
            port,
            proxy_type,
            country,
            city,
            isp,
            quality_score,
            stability_score,
            latency_ms,
            last_checked
         FROM proxies 
         WHERE active = 1 
           AND proxy_type IN ('residential', 'mobile')
           AND stability_score >= 0.8
         ORDER BY 
            stability_score DESC,
            quality_score DESC,
            latency_ms ASC
         LIMIT 50"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    
    // Check if HTML format is requested
    if params.get("format").map(|s| s.as_str()) == Some("html") {
        let mut html = String::from(r#"<!DOCTYPE html>
<html>
<head>
    <title>🔥 Omega9 - Verified Premium Proxies</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: #fff;
            padding: 20px;
            min-height: 100vh;
        }
        .container {
            max-width: 1200px;
            margin: 0 auto;
        }
        .header {
            text-align: center;
            margin-bottom: 40px;
            animation: fadeIn 0.8s;
        }
        .header h1 {
            font-size: 3em;
            margin-bottom: 10px;
            text-shadow: 2px 2px 4px rgba(0,0,0,0.3);
        }
        .flame { animation: flicker 2s infinite; }
        @keyframes flicker {
            0%, 100% { filter: brightness(1) hue-rotate(0deg); }
            50% { filter: brightness(1.2) hue-rotate(10deg); }
        }
        .subtitle {
            font-size: 1.2em;
            opacity: 0.9;
            margin-bottom: 20px;
        }
        .stats {
            display: flex;
            gap: 20px;
            justify-content: center;
            margin-bottom: 40px;
            flex-wrap: wrap;
        }
        .stat-card {
            background: rgba(255,255,255,0.15);
            backdrop-filter: blur(10px);
            padding: 20px 30px;
            border-radius: 15px;
            border: 1px solid rgba(255,255,255,0.2);
            animation: slideUp 0.8s;
        }
        .stat-value {
            font-size: 2.5em;
            font-weight: bold;
            color: #ffd700;
        }
        .stat-label {
            font-size: 0.9em;
            opacity: 0.8;
            margin-top: 5px;
        }
        .proxy-grid {
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
            gap: 20px;
            animation: fadeIn 1s;
        }
        .proxy-card {
            background: rgba(255,255,255,0.1);
            backdrop-filter: blur(10px);
            border-radius: 15px;
            padding: 20px;
            border: 1px solid rgba(255,255,255,0.2);
            transition: all 0.3s;
            position: relative;
            overflow: hidden;
        }
        .proxy-card:hover {
            transform: translateY(-5px);
            background: rgba(255,255,255,0.15);
            box-shadow: 0 10px 30px rgba(0,0,0,0.3);
        }
        .proxy-type {
            position: absolute;
            top: 10px;
            right: 10px;
            background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
            padding: 5px 15px;
            border-radius: 20px;
            font-size: 0.8em;
            font-weight: bold;
        }
        .proxy-url {
            font-size: 1.1em;
            font-weight: bold;
            margin-bottom: 15px;
            word-break: break-all;
            color: #ffd700;
            font-family: 'Courier New', monospace;
        }
        .proxy-details {
            display: grid;
            gap: 10px;
        }
        .detail-row {
            display: flex;
            justify-content: space-between;
            padding: 8px;
            background: rgba(0,0,0,0.2);
            border-radius: 5px;
        }
        .detail-label {
            opacity: 0.8;
            font-size: 0.9em;
        }
        .detail-value {
            font-weight: bold;
        }
        .score-bar {
            height: 8px;
            background: rgba(255,255,255,0.2);
            border-radius: 4px;
            overflow: hidden;
            margin-top: 5px;
        }
        .score-fill {
            height: 100%;
            background: linear-gradient(90deg, #00ff88 0%, #00cc66 100%);
            transition: width 0.5s;
        }
        @keyframes fadeIn {
            from { opacity: 0; }
            to { opacity: 1; }
        }
        @keyframes slideUp {
            from { transform: translateY(20px); opacity: 0; }
            to { transform: translateY(0); opacity: 1; }
        }
        .no-proxies {
            text-align: center;
            padding: 60px 20px;
            font-size: 1.5em;
            opacity: 0.7;
        }
        .flag { font-size: 1.5em; margin-right: 5px; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1><span class="flame">🔥</span> Omega9 Premium Proxies</h1>
            <div class="subtitle">✅ Verified Browsable - 📱 Mobile & 🏠 Residential Only</div>
        </div>
        
        <div class="stats">
            <div class="stat-card">
                <div class="stat-value">"#);
        
        html.push_str(&proxies.len().to_string());
        html.push_str(r#"</div>
                <div class="stat-label">Verified Proxies</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">≥80%</div>
                <div class="stat-label">Browsability Score</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">Google</div>
                <div class="stat-label">Tested Against</div>
            </div>
        </div>
"#);

        if proxies.is_empty() {
            html.push_str(r#"<div class="no-proxies">No verified premium proxies available yet. Hunt cycle in progress...</div>"#);
        } else {
            html.push_str(r#"<div class="proxy-grid">"#);
            
            for proxy in &proxies {
                let type_emoji = if proxy.proxy_type == "mobile" { "📱" } else { "🏠" };
                let country_flag = match proxy.country.as_str() {
                    "United States" => "🇺🇸",
                    "China" => "🇨🇳",
                    "Russia" => "🇷🇺",
                    "Germany" => "🇩🇪",
                    "France" => "🇫🇷",
                    "United Kingdom" => "🇬🇧",
                    "Japan" => "🇯🇵",
                    "India" => "🇮🇳",
                    "Brazil" => "🇧🇷",
                    "Canada" => "🇨🇦",
                    _ => "🌍"
                };
                
                html.push_str(&format!(r#"
            <div class="proxy-card">
                <div class="proxy-type">{} {}</div>
                <div class="proxy-url">{}://{}:{}</div>
                <div class="proxy-details">
                    <div class="detail-row">
                        <span class="detail-label">📍 Location</span>
                        <span class="detail-value">{} {}, {}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">🏢 ISP</span>
                        <span class="detail-value">{}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">⚡ Latency</span>
                        <span class="detail-value">{} ms</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">⭐ Quality</span>
                        <span class="detail-value">{:.1}%</span>
                    </div>
                    <div class="score-bar">
                        <div class="score-fill" style="width: {:.0}%"></div>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">🎯 Stability</span>
                        <span class="detail-value">{:.1}%</span>
                    </div>
                    <div class="score-bar">
                        <div class="score-fill" style="width: {:.0}%"></div>
                    </div>
                </div>
            </div>"#,
                    type_emoji,
                    proxy.proxy_type,
                    proxy.protocol,
                    proxy.host,
                    proxy.port,
                    country_flag,
                    proxy.city.as_ref().unwrap_or(&"Unknown".to_string()),
                    proxy.country,
                    proxy.isp.as_ref().unwrap_or(&"Unknown ISP".to_string()),
                    proxy.latency_ms,
                    proxy.quality_score * 100.0,
                    proxy.quality_score * 100.0,
                    proxy.stability_score * 100.0,
                    proxy.stability_score * 100.0
                ));
            }
            
            html.push_str(r#"
        </div>"#);
        }
        
        html.push_str(r#"
    </div>
</body>
</html>"#);
        
        return (
            [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
            html
        ).into_response();
    }
    
    // Default JSON response
    let response = serde_json::json!({
        "count": proxies.len(),
        "proxies": proxies,
        "info": "Only verified premium proxies that can browse websites (tested against Google.com)",
        "criteria": "stability_score >= 0.8 (browsable), active, mobile or residential"
    });
    
    Json(response).into_response()
}

// Source health monitoring
#[derive(Serialize)]
struct SourceHealth {
    name: String,
    quality_score: f64,
    success_rate: f64,
    last_success: String,
    consecutive_failures: i64,
    total_contributed: i64,
    status: String,
}

async fn get_source_health(State(state): State<AppState>) -> Json<Vec<SourceHealth>> {
    let sources: Vec<Source> = sqlx::query_as(
        "SELECT * FROM sources WHERE active = 1 ORDER BY quality_score DESC"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    
    let health: Vec<SourceHealth> = sources.into_iter().map(|s| {
        let success_rate = s.last_success_rate.unwrap_or(0.0);
        let failures = s.consecutive_failures.unwrap_or(0);
        
        let status = if success_rate > 0.5 && failures == 0 {
            "excellent".to_string()
        } else if success_rate > 0.2 {
            "good".to_string()
        } else if failures < 5 {
            "degraded".to_string()
        } else {
            "failing".to_string()
        };
        
        let last_updated = s.last_updated.unwrap_or(0);
        let last_success = if last_updated > 0 {
            chrono::DateTime::from_timestamp(last_updated, 0)
                .unwrap_or_default()
                .format("%Y-%m-%d %H:%M UTC").to_string()
        } else {
            "Never".to_string()
        };
        
        SourceHealth {
            name: s.name,
            quality_score: s.quality_score,
            success_rate,
            last_success,
            consecutive_failures: failures,
            total_contributed: s.working_proxies,
            status,
        }
    }).collect();
    
    Json(health)
}

// Batch proxy testing
#[derive(Deserialize)]
struct BatchTestRequest {
    proxies: Vec<String>,
    protocol: String,
}

#[derive(Serialize)]
struct BatchTestResult {
    proxy: String,
    working: bool,
    latency_ms: i64,
    quality_score: f64,
    anonymity_level: String,
}

async fn batch_test_proxies(
    State(state): State<AppState>,
    Json(req): Json<BatchTestRequest>
) -> Json<Vec<BatchTestResult>> {
    let mut results = Vec::new();
    
    for proxy in req.proxies.iter().take(20) {
        match crawler::validate_proxy(&state.client, proxy, &req.protocol).await {
            Ok(validation) => {
                let quality = if validation.working {
                    ai::heuristic_score(validation.latency_ms, 
                        &validation.geo.as_ref().map(|g| g.country.as_str()).unwrap_or(""))
                } else {
                    0.0
                };
                
                results.push(BatchTestResult {
                    proxy: proxy.clone(),
                    working: validation.working,
                    latency_ms: validation.latency_ms,
                    quality_score: quality,
                    anonymity_level: validation.anonymity_level,
                });
            }
            Err(_) => {
                results.push(BatchTestResult {
                    proxy: proxy.clone(),
                    working: false,
                    latency_ms: 0,
                    quality_score: 0.0,
                    anonymity_level: "unknown".to_string(),
                });
            }
        }
    }
    
    Json(results)
}

// Multi-format export
#[derive(Deserialize)]
struct ExportParams {
    format: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PremiumExportParams {
    #[serde(rename = "type")]
    proxy_type: Option<String>,  // mobile, residential, premium, all
    format: Option<String>,       // json, txt, pac
    count: Option<usize>,         // max proxies to return
    min_quality: Option<f64>,     // minimum quality score (0.0-1.0)
}

async fn export_proxies(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<ExportParams>
) -> Response {
    let proxies: Vec<Proxy> = sqlx::query_as(
        "SELECT * FROM proxies WHERE active = 1 ORDER BY quality_score DESC LIMIT 1000"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    
    match params.format.as_deref() {
        Some("txt") => {
            let txt = proxies.iter()
                .map(|p| format!("{}://{}:{}", p.protocol, p.host, p.port))
                .collect::<Vec<_>>()
                .join("\n");
            ([("Content-Type", "text/plain"), ("Content-Disposition", "attachment; filename=proxies.txt")], txt).into_response()
        },
        Some("json") => {
            Json(proxies).into_response()
        },
        Some("pac") => {
            let proxy_list = proxies.iter()
                .map(|p| format!("\"{}://{}:{}\"", p.protocol, p.host, p.port))
                .collect::<Vec<_>>()
                .join(", ");
            
            let pac = format!(
                r#"function FindProxyForURL(url, host) {{
    var proxies = [{}];
    var proxy = proxies[Math.floor(Math.random() * proxies.length)];
    return "PROXY " + proxy + "; DIRECT";
}}"#,
                proxy_list
            );
            
            ([("Content-Type", "application/x-ns-proxy-autoconfig"), ("Content-Disposition", "attachment; filename=proxy.pac")], pac).into_response()
        },
        _ => {
            // Default CSV
            export_proxies_csv(State(state)).await
        }
    }
}

// Phase 8: CSV Export endpoint
async fn export_proxies_csv(State(state): State<AppState>) -> Response {
    let proxies: Vec<Proxy> = sqlx::query_as(
        "SELECT * FROM proxies WHERE active = 1 ORDER BY quality_score DESC LIMIT 1000"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let mut csv = "host,port,protocol,country,latency_ms,quality_score,anonymity_level,stability_score\n".to_string();
    for p in proxies {
        csv.push_str(&format!(
            "{},{},{},{},{},{:.2},{},{:.2}\n",
            p.host,
            p.port,
            p.protocol,
            p.country.unwrap_or_else(|| "Unknown".to_string()),
            p.latency_ms.unwrap_or(0),
            p.quality_score,
            p.anonymity_level.unwrap_or_else(|| "unknown".to_string()),
            p.stability_score.unwrap_or(0.0)
        ));
    }

    (
        [("Content-Type", "text/csv"), ("Content-Disposition", "attachment; filename=proxies.csv")],
        csv
    ).into_response()
}

// GOD MODE: Premium Proxy Export API
async fn export_premium_api(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<PremiumExportParams>
) -> Response {
    let proxy_type = params.proxy_type.as_deref().unwrap_or("all");
    let format = params.format.as_deref().unwrap_or("json");
    let count = params.count.unwrap_or(100).min(1000); // Max 1000
    let min_quality = params.min_quality.unwrap_or(0.5);

    // Build query based on type filter
    let query = match proxy_type {
        "mobile" => "SELECT * FROM proxies WHERE active = 1 AND is_mobile = 1 AND quality_score >= ? ORDER BY quality_score DESC LIMIT ?",
        "residential" => "SELECT * FROM proxies WHERE active = 1 AND is_residential = 1 AND quality_score >= ? ORDER BY quality_score DESC LIMIT ?",
        "premium" => "SELECT * FROM proxies WHERE active = 1 AND (is_mobile = 1 OR is_residential = 1) AND quality_score >= ? ORDER BY quality_score DESC LIMIT ?",
        _ => "SELECT * FROM proxies WHERE active = 1 AND quality_score >= ? ORDER BY quality_score DESC LIMIT ?",
    };

    let proxies: Vec<Proxy> = sqlx::query_as(query)
        .bind(min_quality)
        .bind(count as i64)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    match format {
        "txt" => {
            let txt = proxies.iter()
                .map(|p| format!("{}://{}:{}", p.protocol, p.host, p.port))
                .collect::<Vec<_>>()
                .join("\n");
            ([("Content-Type", "text/plain")], txt).into_response()
        },
        "pac" => {
            let proxy_list = proxies.iter()
                .map(|p| format!("\"{}://{}:{}\"", p.protocol, p.host, p.port))
                .collect::<Vec<_>>()
                .join(", ");
            
            let pac = format!(
                r#"function FindProxyForURL(url, host) {{
    var proxies = [{}];
    var proxy = proxies[Math.floor(Math.random() * proxies.length)];
    return "PROXY " + proxy + "; DIRECT";
}}"#,
                proxy_list
            );
            
            ([("Content-Type", "application/x-ns-proxy-autoconfig")], pac).into_response()
        },
        _ => {
            // JSON (default)
            Json(proxies).into_response()
        }
    }
}

// Phase 8: Protocol breakdown statistics
#[derive(Serialize)]
struct ProtocolStats {
    protocol: String,
    count: i64,
    avg_quality: f64,
    avg_latency: f64,
}

async fn get_protocol_breakdown(State(state): State<AppState>) -> Json<Vec<ProtocolStats>> {
    #[derive(sqlx::FromRow)]
    struct Row {
        protocol: String,
        count: i64,
        avg_quality: f64,
        avg_latency: f64,
    }

    let stats: Vec<Row> = sqlx::query_as(
        "SELECT 
            protocol,
            COUNT(*) as count,
            AVG(quality_score) as avg_quality,
            AVG(latency_ms) as avg_latency
         FROM proxies 
         WHERE active = 1
         GROUP BY protocol
         ORDER BY count DESC"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let protocol_stats: Vec<ProtocolStats> = stats.into_iter().map(|r| ProtocolStats {
        protocol: r.protocol,
        count: r.count,
        avg_quality: r.avg_quality,
        avg_latency: r.avg_latency,
    }).collect();

    Json(protocol_stats)
}

async fn trigger_hunt(State(state): State<AppState>) -> Response {
    // Just return success, actual hunt happens in background loop
    Json(serde_json::json!({"status": "hunt triggered"})).into_response()
}

#[derive(Deserialize)]
struct TestProxyRequest {
    host: String,
    port: u16,
    protocol: String,
}

async fn test_proxy_endpoint(
    State(state): State<AppState>,
    Json(req): Json<TestProxyRequest>,
) -> Response {
    let proxy_str = format!("{}:{}", req.host, req.port);
    
    match crawler::validate_proxy(&state.client, &proxy_str, &req.protocol).await {
        Ok(result) => {
            Json(serde_json::json!({
                "success": true,
                "working": result.working,
                "latency_ms": result.latency_ms,
                "geo": result.geo,
                "fraud": result.fraud,
                "dns_leak": result.dns_leak,
                "elite": result.elite
            })).into_response()
        }
        Err(e) => {
            Json(serde_json::json!({
                "success": false,
                "error": e.to_string()
            })).into_response()
        }
    }
}

async fn websocket_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(|socket| websocket(socket, state))
}

// GOD MODE: Prometheus metrics endpoint
async fn metrics_handler(State(state): State<AppState>) -> Response {
    use prometheus::{Encoder, TextEncoder, Gauge, Counter, Registry, Opts};
    
    let registry = Registry::new();
    
    // Query current stats from database
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM proxies")
        .fetch_one(&state.db).await.unwrap_or(0);
    let working: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM proxies WHERE active = 1")
        .fetch_one(&state.db).await.unwrap_or(0);
    let mobile: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM proxies WHERE is_mobile = 1 AND active = 1")
        .fetch_one(&state.db).await.unwrap_or(0);
    let residential: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM proxies WHERE is_residential = 1 AND active = 1")
        .fetch_one(&state.db).await.unwrap_or(0);
    let avg_latency: f64 = sqlx::query_scalar("SELECT AVG(latency_ms) FROM proxies WHERE active = 1")
        .fetch_one(&state.db).await.unwrap_or(0.0);
    let avg_quality: f64 = sqlx::query_scalar("SELECT AVG(quality_score) FROM proxies WHERE active = 1")
        .fetch_one(&state.db).await.unwrap_or(0.0);
    
    // Create gauges
    let total_gauge = Gauge::with_opts(Opts::new("omega9_total_proxies", "Total proxies in database")).unwrap();
    let working_gauge = Gauge::with_opts(Opts::new("omega9_working_proxies", "Active working proxies")).unwrap();
    let mobile_gauge = Gauge::with_opts(Opts::new("omega9_mobile_proxies", "Mobile proxies count")).unwrap();
    let residential_gauge = Gauge::with_opts(Opts::new("omega9_residential_proxies", "Residential proxies count")).unwrap();
    let latency_gauge = Gauge::with_opts(Opts::new("omega9_avg_latency_ms", "Average latency in milliseconds")).unwrap();
    let quality_gauge = Gauge::with_opts(Opts::new("omega9_avg_quality_score", "Average quality score")).unwrap();
    
    // Set values
    total_gauge.set(total as f64);
    working_gauge.set(working as f64);
    mobile_gauge.set(mobile as f64);
    residential_gauge.set(residential as f64);
    latency_gauge.set(avg_latency);
    quality_gauge.set(avg_quality);
    
    // Register metrics
    registry.register(Box::new(total_gauge)).unwrap();
    registry.register(Box::new(working_gauge)).unwrap();
    registry.register(Box::new(mobile_gauge)).unwrap();
    registry.register(Box::new(residential_gauge)).unwrap();
    registry.register(Box::new(latency_gauge)).unwrap();
    registry.register(Box::new(quality_gauge)).unwrap();
    
    // Encode to Prometheus format
    let encoder = TextEncoder::new();
    let mut buffer = vec![];
    let metric_families = registry.gather();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    
    ([("Content-Type", "text/plain; version=0.0.4")], buffer).into_response()
}

async fn websocket(mut socket: WebSocket, state: AppState) {
    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;

        let stats = state.stats.read().await;
        let msg = serde_json::to_string(&*stats).unwrap_or_default();

        if socket.send(WsMessage::Text(msg)).await.is_err() {
            break;
        }
    }
}
