use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoInfo {
    pub country: String,
    pub city: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FraudInfo {
    pub score: f64,
    pub risky: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub working: bool,
    pub latency_ms: i64,
    pub geo: Option<GeoInfo>,
    pub fraud: Option<FraudInfo>,
    pub dns_leak: bool,
    pub elite: bool,
    pub anonymity_level: String, // "elite", "anonymous", "transparent"
    pub stability_score: f64, // 0.0-1.0 based on multi-ping test
}

/// Fetch proxies from a given URL
pub async fn fetch_proxies(client: &Client, url: &str) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    debug!("Fetching proxies from: {}", url);
    
    let response = client
        .get(url)
        .timeout(Duration::from_secs(30))
        .send()
        .await?;
    
    let body = response.text().await?;
    let mut proxies = Vec::new();

    // Parse common formats
    for line in body.lines() {
        let line = line.trim();
        
        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Match IP:PORT pattern
        if line.contains(':') {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 {
                if let Ok(_) = parts[0].parse::<std::net::IpAddr>() {
                    if let Ok(_) = parts[1].parse::<u16>() {
                        proxies.push(format!("{}:{}", parts[0], parts[1]));
                    }
                }
            }
        }
    }

    debug!("Fetched {} proxies from {}", proxies.len(), url);
    Ok(proxies)
}

/// Fetch geolocation info for a proxy
pub async fn fetch_geo(client: &Client, proxy: &str) -> Result<GeoInfo, Box<dyn std::error::Error + Send + Sync>> {
    let parts: Vec<&str> = proxy.split(':').collect();
    let ip = parts[0];
    
    // Using ip-api.com (free tier)
    let url = format!("http://ip-api.com/json/{}", ip);
    
    #[derive(Deserialize)]
    struct IpApiResponse {
        #[serde(default)]
        country: String,
        #[serde(default)]
        city: String,
        status: String,
    }
    
    let response = client
        .get(&url)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;
    
    let data: IpApiResponse = response.json().await?;
    
    if data.status == "success" {
        Ok(GeoInfo {
            country: data.country,
            city: data.city,
        })
    } else {
        Err("Geolocation lookup failed".into())
    }
}

/// Check fraud score via scamalytics.com
pub async fn fetch_fraud(client: &Client, proxy: &str) -> Result<FraudInfo, Box<dyn std::error::Error + Send + Sync>> {
    let parts: Vec<&str> = proxy.split(':').collect();
    let ip = parts[0];
    
    let url = format!("https://scamalytics.com/ip/{}", ip);
    
    let response = client
        .get(&url)
        .timeout(Duration::from_secs(15))
        .send()
        .await?;
    
    let body = response.text().await?;
    let document = Html::parse_document(&body);
    
    // Parse fraud score from page
    let score_selector = Selector::parse(".score").ok();
    let mut fraud_score = 0.0;
    
    if let Some(selector) = score_selector {
        if let Some(element) = document.select(&selector).next() {
            let text = element.text().collect::<String>();
            fraud_score = text.parse::<f64>().unwrap_or(0.0) / 100.0;
        }
    }
    
    Ok(FraudInfo {
        score: fraud_score,
        risky: fraud_score > 0.5,
    })
}

/// Check for DNS leaks via bash.ws
pub async fn check_dns_leak(client: &Client, proxy_url: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let proxy = reqwest::Proxy::all(proxy_url)?;
    let proxy_client = Client::builder()
        .proxy(proxy)
        .timeout(Duration::from_secs(10))
        .build()?;
    
    let response = proxy_client
        .get("https://bash.ws/json")
        .send()
        .await?;
    
    #[derive(Deserialize)]
    struct BashWsResponse {
        #[serde(default)]
        ip: String,
    }
    
    let data: BashWsResponse = response.json().await?;
    
    // If returned IP matches proxy IP, no leak
    let parts: Vec<&str> = proxy_url.split('@').last().unwrap_or("").split(':').collect();
    let proxy_ip = parts[0];
    
    Ok(data.ip != proxy_ip)
}

/// Check for elite anonymity via httpbin.org
pub async fn check_elite_anonymity(client: &Client, proxy_url: &str) -> Result<(bool, String), Box<dyn std::error::Error + Send + Sync>> {
    let proxy = reqwest::Proxy::all(proxy_url)?;
    let proxy_client = Client::builder()
        .proxy(proxy)
        .timeout(Duration::from_secs(10))
        .build()?;
    
    let response = proxy_client
        .get("https://httpbin.org/headers")
        .send()
        .await?;
    
    #[derive(Deserialize)]
    struct HeadersResponse {
        headers: std::collections::HashMap<String, String>,
    }
    
    let data: HeadersResponse = response.json().await?;
    
    // Check anonymity level
    let has_via = data.headers.contains_key("Via");
    let has_forwarded = data.headers.contains_key("X-Forwarded-For");
    let has_proxy_id = data.headers.contains_key("X-Proxy-Id");
    let has_real_ip = data.headers.contains_key("X-Real-Ip");
    
    let (elite, level) = if !has_via && !has_forwarded && !has_proxy_id && !has_real_ip {
        (true, "elite".to_string())
    } else if has_via || has_forwarded {
        (false, "transparent".to_string())
    } else {
        (false, "anonymous".to_string())
    };
    
    Ok((elite, level))
}

/// Test proxy stability with multiple pings
pub async fn test_stability(client: &Client, proxy_url: &str, pings: usize) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
    let proxy = reqwest::Proxy::all(proxy_url)?;
    let proxy_client = Client::builder()
        .proxy(proxy)
        .timeout(Duration::from_secs(5))
        .build()?;
    
    let mut successful_pings = 0;
    
    for _ in 0..pings {
        if proxy_client.get("https://httpbin.org/ip").send().await.is_ok() {
            successful_pings += 1;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    Ok(successful_pings as f64 / pings as f64)
}

/// Validate a single proxy with comprehensive checks
pub async fn validate_proxy(
    client: &Client,
    proxy: &str,
    protocol: &str,
) -> Result<ValidationResult, Box<dyn std::error::Error + Send + Sync>> {
    let proxy_url = format!("{}://{}", protocol, proxy);
    
    // Basic connectivity test
    let start = std::time::Instant::now();
    let proxy_obj = reqwest::Proxy::all(&proxy_url)?;
    let test_client = Client::builder()
        .proxy(proxy_obj)
        .timeout(Duration::from_secs(10))
        .build()?;
    
    match test_client.get("https://httpbin.org/ip").send().await {
        Ok(_) => {
            let latency_ms = start.elapsed().as_millis() as i64;
            
            // Fetch additional info (in parallel for speed)
            let geo = fetch_geo(client, proxy).await.ok();
            let fraud = fetch_fraud(client, proxy).await.ok();
            let dns_leak = check_dns_leak(client, &proxy_url).await.unwrap_or(false);
            let (elite, anon_level) = check_elite_anonymity(client, &proxy_url).await.unwrap_or((false, "unknown".to_string()));
            let stability = test_stability(client, &proxy_url, 3).await.unwrap_or(0.0);
            
            Ok(ValidationResult {
                working: true,
                latency_ms,
                geo,
                fraud,
                dns_leak,
                elite,
                anonymity_level: anon_level,
                stability_score: stability,
            })
        }
        Err(e) => {
            debug!("Proxy {} failed: {}", proxy, e);
            Ok(ValidationResult {
                working: false,
                latency_ms: 0,
                geo: None,
                fraud: None,
                dns_leak: false,
                elite: false,
                anonymity_level: "unknown".to_string(),
                stability_score: 0.0,
            })
        }
    }
}

/// Fetch proxies from local network using arp-scan results
pub async fn fetch_local(client: &Client) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    // This would typically read from a shared volume where scanner sidecar writes results
    let scan_results_path = "/tmp/arp-scan-results.txt";
    
    match tokio::fs::read_to_string(scan_results_path).await {
        Ok(content) => {
            let mut proxies = Vec::new();
            
            for line in content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    // Try common proxy ports
                    for port in [3128, 8080, 1080, 8888, 9050] {
                        proxies.push(format!("{}:{}", parts[0], port));
                    }
                }
            }
            
            debug!("Found {} potential local proxies", proxies.len());
            Ok(proxies)
        }
        Err(_) => {
            warn!("No local scan results found");
            Ok(Vec::new())
        }
    }
}
