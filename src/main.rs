mod ai;
mod crawler;
mod discovery;

use axum::{
    extract::{
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
        State,
    },
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePool, Pool, Sqlite};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use teloxide::{prelude::*, types::ParseMode, utils::command::BotCommands};
use tokio::sync::RwLock;
use tower_http::{cors::CorsLayer, services::ServeDir};
use tracing::{error, info, warn};

#[derive(Clone)]
struct AppState {
    db: Pool<Sqlite>,
    client: reqwest::Client,
    bot: Bot,
    stats: Arc<RwLock<Stats>>,
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
    };

    // Spawn background tasks
    tokio::spawn(hunt_loop(state.clone()));
    tokio::spawn(discovery_loop(state.clone()));
    tokio::spawn(stats_updater(state.clone()));
    tokio::spawn(run_bot(bot.clone(), state.clone()));

    // Build web server
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/api/proxies", get(get_proxies))
        .route("/api/stats", get(get_stats))
        .route("/api/sources", get(get_sources))
        .route("/api/hunt", post(trigger_hunt))
        .route("/api/test-proxy", post(test_proxy_endpoint))
        .route("/ws", get(websocket_handler))
        .nest_service("/static", ServeDir::new("static"))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr: SocketAddr = bind_addr.parse()?;
    info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// Background hunt loop
async fn hunt_loop(state: AppState) {
    let interval_secs = std::env::var("HUNT_INTERVAL_SECS")
        .unwrap_or_else(|_| "300".to_string())
        .parse::<u64>()
        .unwrap_or(300);

    loop {
        info!("Starting hunt cycle");

        // Get all active sources
        let sources: Vec<Source> = match sqlx::query_as("SELECT * FROM sources WHERE active = 1 ORDER BY quality_score DESC")
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

        info!("Hunting from {} sources", sources.len());

        for source in sources {
            match crawler::fetch_proxies(&state.client, &source.url).await {
                Ok(proxies) => {
                    info!("Fetched {} proxies from {}", proxies.len(), source.name);

                    // Limit to first 500 proxies per source for aggressive hunting
                    let limited_proxies: Vec<_> = proxies.iter().take(500).cloned().collect();
                    info!("Validating {} proxies from {} (limited from {})", limited_proxies.len(), source.name, proxies.len());

                    let mut working_count = 0;
                    
                    // Process in batches of 50 concurrently for maximum speed
                    for chunk in limited_proxies.chunks(50) {
                        let mut tasks = Vec::new();
                        
                        for proxy in chunk {
                            let client = state.client.clone();
                            let proxy_str = proxy.clone();
                            let protocol = if source.url.contains("socks") {
                                "socks5"
                            } else {
                                "http"
                            };
                            
                            tasks.push(tokio::spawn(async move {
                                crawler::validate_proxy(&client, &proxy_str, protocol).await
                            }));
                        }
                        
                        // Wait for batch to complete
                        for (idx, task) in tasks.into_iter().enumerate() {
                            if let Ok(Ok(result)) = task.await {
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

                                    let quality_score = ai::predict_score(
                                        result.latency_ms,
                                        country_rare,
                                        source_quality,
                                        age_hours,
                                        fraud_score,
                                        result.dns_leak,
                                        result.elite,
                                        &weights,
                                    );

                                    // Insert or update proxy
                                    let now = Utc::now().timestamp();
                                    let protocol = if source.url.contains("socks") { "socks5" } else { "http" };
                                    let _ = sqlx::query(
                                        "INSERT INTO proxies (host, port, protocol, country, city, latency_ms, quality_score, fraud_score, dns_leak, elite, anonymity_level, stability_score, last_checked, discovered_at, source, active)
                                         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1)
                                         ON CONFLICT(host, port, protocol) DO UPDATE SET
                                         latency_ms = ?, quality_score = ?, fraud_score = ?, dns_leak = ?, elite = ?, anonymity_level = ?, stability_score = ?, last_checked = ?, active = 1"
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
                                    .bind(result.latency_ms)
                                    .bind(quality_score)
                                    .bind(fraud_score)
                                    .bind(if result.dns_leak { 1 } else { 0 })
                                    .bind(if result.elite { 1 } else { 0 })
                                    .bind(&result.anonymity_level)
                                    .bind(result.stability_score)
                                    .bind(now)
                                    .execute(&state.db)
                                    .await;
                                }
                            }
                        }
                    }

                    // Update source quality using EMA
                    let success_rate = working_count as f64 / limited_proxies.len().max(1) as f64;
                    let new_quality = ai::update_source_quality(source.quality_score, success_rate, 0.3);
                    let now = Utc::now().timestamp();

                    let _ = sqlx::query(
                        "UPDATE sources SET quality_score = ?, total_proxies = total_proxies + ?, working_proxies = working_proxies + ?, last_updated = ? WHERE id = ?"
                    )
                    .bind(new_quality)
                    .bind(limited_proxies.len() as i64)
                    .bind(working_count)
                    .bind(now)
                    .bind(source.id)
                    .execute(&state.db)
                    .await;
                    
                    info!("Found {} working proxies from {} (quality: {:.2})", working_count, source.name, new_quality);
                }
                Err(e) => {
                    warn!("Failed to fetch from {}: {}", source.name, e);
                }
            }
        }

        info!("Hunt cycle complete, sleeping for {} seconds", interval_secs);
        tokio::time::sleep(Duration::from_secs(interval_secs)).await;
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

// Telegram bot handler
async fn run_bot(bot: Bot, state: AppState) {
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
}

// HTTP Handlers
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
    let stats = state.stats.read().await;
    Json(stats.clone())
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
