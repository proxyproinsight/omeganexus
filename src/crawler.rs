use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error, warn, info};
use crate::asn_detector::{ASNDetector, ASNData};
use fake_useragent::UserAgentsBuilder;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoInfo {
    pub country: String,
    pub city: String,
    pub isp: Option<String>,
    pub asn: Option<String>,
    pub proxy_type: String, // "datacenter", "residential", "mobile"
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
    pub asn_data: Option<ASNData>, // NEW: ASN-based mobile/residential detection
}

/// Fetch proxies from a given URL (supports TXT and JSON formats)
pub async fn fetch_proxies(client: &Client, url: &str) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    debug!("Fetching proxies from: {}", url);
    
    // GOD MODE: Rotate user agent
    let ua_pool = UserAgentsBuilder::new().cache(false).build();
    let random_ua = ua_pool.random();
    
    let response = client
        .get(url)
        .header("User-Agent", random_ua)
        .timeout(Duration::from_secs(30))
        .send()
        .await?;
    
    let body = response.text().await?;
    let mut proxies = Vec::new();

    // 🚀 GOD MODE: Handle fate0 JSON format (http://proxylist.fatezero.org/proxy.list)
    if url.contains("fatezero.org") || url.contains("fate0") {
        use crate::sources::parse_fate0_json;
        proxies = parse_fate0_json(&body);
        debug!("Fetched {} proxies from {} (JSON format)", proxies.len(), url);
        return Ok(proxies);
    }

    // Parse standard TXT format (IP:PORT per line)
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

    debug!("Fetched {} proxies from {} (TXT format)", proxies.len(), url);
    Ok(proxies)
}

/// Fetch geolocation info for a proxy
pub async fn fetch_geo(client: &Client, proxy: &str) -> Result<GeoInfo, Box<dyn std::error::Error + Send + Sync>> {
    let parts: Vec<&str> = proxy.split(':').collect();
    let ip = parts[0];
    
    // Using ip-api.com (free tier) with fields parameter for ISP/ASN data
    let url = format!("http://ip-api.com/json/{}?fields=status,country,city,isp,as,mobile", ip);
    
    #[derive(Deserialize)]
    struct IpApiResponse {
        #[serde(default)]
        country: String,
        #[serde(default)]
        city: String,
        #[serde(default)]
        isp: String,
        #[serde(default, rename = "as")]
        asn: String,
        #[serde(default)]
        mobile: bool,
        status: String,
    }
    
    let response = client
        .get(&url)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;
    
    let data: IpApiResponse = response.json().await?;
    
    if data.status == "success" {
        // Detect proxy type based on ISP name and mobile flag
        let proxy_type = detect_proxy_type(&data.isp, data.mobile);
        
        Ok(GeoInfo {
            country: data.country,
            city: data.city,
            isp: if !data.isp.is_empty() { Some(data.isp) } else { None },
            asn: if !data.asn.is_empty() { Some(data.asn) } else { None },
            proxy_type,
        })
    } else {
        Err("Geolocation lookup failed".into())
    }
}

/// Detect if proxy is residential, mobile, or datacenter based on ISP name
fn detect_proxy_type(isp: &str, is_mobile: bool) -> String {
    let isp_lower = isp.to_lowercase();
    
    // Mobile carriers (high value!)
    let mobile_keywords = [
        "mobile", "wireless", "cellular", "t-mobile", "verizon", "at&t", "att",
        "sprint", "vodafone", "orange", "o2", "telefonica", "telekom", "rogers",
        "bell canada", "telus", "claro", "tim", "movistar", "airtel", "reliance",
        "jio", "idea", "mtn", "safaricom", "china mobile", "china unicom"
    ];
    
    // Residential ISPs (golden!)
    let residential_keywords = [
        "comcast", "xfinity", "charter", "spectrum", "cox", "optimum", "altice",
        "centurylink", "frontier", "windstream", "bt internet", "sky broadband",
        "virgin media", "talktalk", "plusnet", "ee", "vodafone broadband",
        "telstra", "optus", "tpg", "dodo", "telus", "shaw", "cogeco", "videotron",
        "oi", "vivo", "net", "telmex", "izzi", "megacable", "totalplay",
        "rostelecom", "beeline", "mts", "megafon", "ttnet", "turk telekom"
    ];
    
    // Datacenter/hosting (lower value)
    let datacenter_keywords = [
        "amazon", "aws", "google", "gcp", "microsoft", "azure", "digital ocean",
        "digitalocean", "linode", "vultr", "ovh", "hetzner", "choopa", "quadranet",
        "constant", "leaseweb", "online.net", "scaleway", "packet", "cloudflare"
    ];
    
    if is_mobile {
        return "mobile".to_string();
    }
    
    for keyword in mobile_keywords.iter() {
        if isp_lower.contains(keyword) {
            return "mobile".to_string();
        }
    }
    
    for keyword in residential_keywords.iter() {
        if isp_lower.contains(keyword) {
            return "residential".to_string();
        }
    }
    
    for keyword in datacenter_keywords.iter() {
        if isp_lower.contains(keyword) {
            return "datacenter".to_string();
        }
    }
    
    // Default to datacenter if unknown
    "datacenter".to_string()
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

/// Check for DNS leaks via bash.ws dnsleak test
pub async fn check_dns_leak(proxy_url: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let proxy = reqwest::Proxy::all(proxy_url)?;
    let proxy_client = Client::builder()
        .proxy(proxy)
        .timeout(Duration::from_secs(15))
        .build()?;
    
    // Fetch DNS leak test results
    let response = proxy_client
        .get("https://bash.ws/dnsleak/test/?json")
        .send()
        .await?;
    
    let json: serde_json::Value = response.json().await?;
    
    // Known public DNS servers (whitelist - these are OK)
    let public_dns = vec![
        "8.8.8.8", "8.8.4.4",           // Google DNS
        "1.1.1.1", "1.0.0.1",           // Cloudflare DNS
        "208.67.222.222", "208.67.220.220", // OpenDNS
        "9.9.9.9", "149.112.112.112",   // Quad9
        "64.6.64.6", "64.6.65.6",       // Verisign
    ];
    
    // Parse dns_servers array
    if let Some(dns_servers) = json.get("dns_servers").and_then(|v| v.as_array()) {
        for server in dns_servers {
            if let Some(ip) = server.get("ip").and_then(|v| v.as_str()) {
                // If DNS server is NOT in public whitelist, it's a leak
                if !public_dns.iter().any(|&public| ip.starts_with(public)) {
                    info!("🚨 DNS LEAK detected: Using DNS server {} (not public)", ip);
                    return Ok(true); // DNS leak detected
                }
            }
        }
    }
    
    Ok(false) // No leak - all DNS servers are public
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

/// Fast validation - only connectivity + geo (skip fraud/dns/stability for speed)
pub async fn validate_proxy_fast(
    client: &Client,
    proxy: &str,
    protocol: &str,
) -> Result<ValidationResult, Box<dyn std::error::Error + Send + Sync>> {
    let proxy_url = format!("{}://{}", protocol, proxy);
    
    // Basic connectivity test with reduced timeout
    let start = std::time::Instant::now();
    let proxy_obj = reqwest::Proxy::all(&proxy_url)?;
    let test_client = Client::builder()
        .proxy(proxy_obj)
        .timeout(Duration::from_secs(5)) // Reduced from 10s
        .pool_max_idle_per_host(0) // Disable keep-alive for faster cleanup
        .build()?;
    
    // Test basic connectivity
    match test_client.get("https://httpbin.org/ip").send().await {
        Ok(_) => {
            let latency_ms = start.elapsed().as_millis() as i64;
            
            // Only fetch geo info - skip fraud/dns/stability for speed
            let geo = fetch_geo(client, proxy).await.ok();
            
            // For premium proxies (mobile/residential), do extra web browsing test
            let mut stability_score = 0.7;
            if let Some(ref geo_data) = geo {
                if geo_data.proxy_type == "mobile" || geo_data.proxy_type == "residential" {
                    // Test if we can actually browse a real website
                    if test_client.get("https://www.google.com").send().await.is_ok() {
                        stability_score = 0.85; // Higher score for browsable premium proxies
                    } else {
                        stability_score = 0.4; // Lower score if can't browse
                    }
                }
            }
            
            Ok(ValidationResult {
                working: true,
                latency_ms,
                geo,
                fraud: None, // Skip fraud check
                dns_leak: false, // Skip DNS leak check
                elite: false, // Skip elite check
                anonymity_level: "unknown".to_string(),
                stability_score,
                asn_data: None, // Skip ASN check in fast mode
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
                asn_data: None,
            })
        }
    }
}

/// Validate a single proxy with comprehensive checks + ASN mobile/residential detection
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
            
            // Extract IP for ASN lookup
            let ip = proxy.split(':').next().unwrap_or("");
            
            // Fetch additional info (in parallel for speed)
            let geo = fetch_geo(client, proxy).await.ok();
            let fraud = fetch_fraud(client, proxy).await.ok();
            let dns_leak = check_dns_leak(&proxy_url).await.unwrap_or(false);
            let (elite, anon_level) = check_elite_anonymity(client, &proxy_url).await.unwrap_or((false, "unknown".to_string()));
            let stability = test_stability(client, &proxy_url, 3).await.unwrap_or(0.0);
            
            // 🎯 GOD MODE: ASN-based mobile/residential detection
            let asn_detector = ASNDetector::new();
            let asn_data = match asn_detector.fetch_asn_data(ip).await {
                Ok(data) => {
                    // Log mobile/residential discoveries
                    if data.is_mobile {
                        info!("🚀 MOBILE PROXY DISCOVERED: {} (Carrier: {} | ASN: {})", 
                              proxy, 
                              data.carrier_name.as_ref().unwrap_or(&"Unknown".to_string()),
                              data.asn);
                    } else if data.is_residential {
                        info!("🏠 RESIDENTIAL PROXY DISCOVERED: {} (ISP: {} | ASN: {})", 
                              proxy,
                              data.isp_name.as_ref().unwrap_or(&"Unknown".to_string()),
                              data.asn);
                    } else {
                        debug!("Datacenter proxy: {} (ASN: {} | Org: {})", proxy, data.asn, data.org);
                    }
                    Some(data)
                }
                Err(e) => {
                    warn!("ASN detection failed for {}: {}", proxy, e);
                    None
                }
            };
            
            Ok(ValidationResult {
                working: true,
                latency_ms,
                geo,
                fraud,
                dns_leak,
                elite,
                anonymity_level: anon_level,
                stability_score: stability,
                asn_data,
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
                asn_data: None,
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
