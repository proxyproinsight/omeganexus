# 🚀 OMEGA9-NEXUS: GOD MODE IMPLEMENTATION GUIDE

> **Objective**: Transform Omega9-Nexus into the most aggressive, intelligent, and efficient proxy hunter capable of discovering 1,000+ validated mobile/residential proxies per hour.

**Status**: 🟡 In Progress  
**Current Stats**: 3 verified proxies (2 mobile, 1 residential)  
**Target Stats**: 50-200 mobile/residential proxies per session, 1,000+ total proxies/hour  
**Timeline**: 3-4 weeks (60-80 hours total)  

---

## 📊 CURRENT STATE ANALYSIS

### Existing Infrastructure
```
├── src/
│   ├── main.rs          # API server, Telegram bot, hunt orchestration
│   ├── crawler.rs       # Proxy fetching, validation, geo lookup
│   ├── discovery.rs     # Source discovery, Tor integration
│   └── ai.rs            # Basic heuristic scoring
├── static/
│   └── index.html       # Dashboard UI
└── migrations/
    └── init.sql         # PostgreSQL schema
```

### Current Capabilities
- ✅ Basic proxy fetching from ~20 sources
- ✅ IP geolocation via ipinfo.io
- ✅ Fraud detection via scamalytics.com
- ✅ Google.com browsing test for premium proxies
- ✅ Telegram bot with commands
- ✅ Web dashboard with real-time stats

### Current Limitations
- ❌ No ASN-based mobile/residential detection
- ❌ No rotation testing for mobile proxies
- ❌ Limited sources (~20 vs 100+ needed)
- ❌ No browser emulation (60% ban rate)
- ❌ Sequential validation (slow)
- ❌ No AI-powered discovery
- ❌ No self-healing source management

---

## 🎯 GOD MODE ARCHITECTURE

### System Overview
```mermaid
graph TB
    A[Hunt Orchestrator] --> B[Source Discovery Layer]
    A --> C[Validation Pipeline]
    A --> D[Storage Layer]
    
    B --> B1[Static Sources: 50+]
    B --> B2[Dynamic Scrapers: HTML/Browser]
    B --> B3[AI Discovery: LLM + Dorks]
    B --> B4[Reddit/GitHub Monitors]
    
    C --> C1[Stage 1: ASN Detection]
    C --> C2[Stage 2: Rotation Test]
    C --> C3[Stage 3: Browse Test]
    C --> C4[Stage 4: Fraud Check]
    C --> C5[Stage 5: Device Simulation]
    
    D --> D1[PostgreSQL + Indexes]
    D --> D2[Proxy Pool Cache]
    D --> D3[Source Health Tracker]
    
    A --> E[API Layer]
    E --> E1[/api/proxies]
    E --> E2[/api/export]
    E --> E3[/api/premium/verified]
    E --> E4[/metrics Prometheus]
```

### Data Flow
```
Source URL → Fetch (with proxy pool) → Parse → ASN Lookup → 
5-Stage Validation → Tag (mobile/residential) → Database → 
API/Dashboard → Rotation Testing (24h loop)
```

---

## 🗄️ DATABASE SCHEMA ENHANCEMENTS

### New Columns (Task #1)
```sql
-- Migration: migrations/add_premium_fields.sql
ALTER TABLE proxies ADD COLUMN IF NOT EXISTS is_mobile BOOLEAN DEFAULT FALSE;
ALTER TABLE proxies ADD COLUMN IF NOT EXISTS is_residential BOOLEAN DEFAULT FALSE;
ALTER TABLE proxies ADD COLUMN IF NOT EXISTS carrier_asn INTEGER;
ALTER TABLE proxies ADD COLUMN IF NOT EXISTS isp_name TEXT;
ALTER TABLE proxies ADD COLUMN IF NOT EXISTS rotation_verified BOOLEAN DEFAULT FALSE;
ALTER TABLE proxies ADD COLUMN IF NOT EXISTS last_rotation_test TIMESTAMP;
ALTER TABLE proxies ADD COLUMN IF NOT EXISTS fraud_score FLOAT DEFAULT 0.0;
ALTER TABLE proxies ADD COLUMN IF NOT EXISTS abuse_score FLOAT DEFAULT 0.0;
ALTER TABLE proxies ADD COLUMN IF NOT EXISTS anonymity_level TEXT DEFAULT 'unknown';
ALTER TABLE proxies ADD COLUMN IF NOT EXISTS browser_compatible BOOLEAN DEFAULT FALSE;

-- Indexes for fast filtering
CREATE INDEX IF NOT EXISTS idx_is_mobile ON proxies(is_mobile) WHERE is_mobile = TRUE;
CREATE INDEX IF NOT EXISTS idx_is_residential ON proxies(is_residential) WHERE is_residential = TRUE;
CREATE INDEX IF NOT EXISTS idx_carrier_asn ON proxies(carrier_asn);
CREATE INDEX IF NOT EXISTS idx_quality_mobile ON proxies(quality_score, is_mobile) WHERE is_mobile = TRUE;
CREATE INDEX IF NOT EXISTS idx_quality_residential ON proxies(quality_score, is_residential) WHERE is_residential = TRUE;

-- Source health tracking table
CREATE TABLE IF NOT EXISTS source_health (
    id SERIAL PRIMARY KEY,
    source_url TEXT UNIQUE NOT NULL,
    total_fetches INTEGER DEFAULT 0,
    successful_fetches INTEGER DEFAULT 0,
    last_success TIMESTAMP,
    last_failure TIMESTAMP,
    avg_yield FLOAT DEFAULT 0.0,
    fraud_rate FLOAT DEFAULT 0.0,
    is_active BOOLEAN DEFAULT TRUE,
    priority INTEGER DEFAULT 5,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_source_active ON source_health(is_active, priority DESC);
```

### Query Patterns
```sql
-- Get all verified mobile proxies
SELECT * FROM proxies 
WHERE is_mobile = TRUE 
  AND rotation_verified = TRUE 
  AND quality_score > 0.8 
  AND active = TRUE
ORDER BY quality_score DESC
LIMIT 100;

-- Get residential proxies by ISP
SELECT * FROM proxies 
WHERE is_residential = TRUE 
  AND isp_name IN ('Comcast', 'Spectrum', 'AT&T')
  AND fraud_score < 0.3
  AND browser_compatible = TRUE
ORDER BY stability_score DESC;

-- Source health report
SELECT 
    source_url,
    avg_yield,
    (successful_fetches::FLOAT / NULLIF(total_fetches, 0)) as success_rate,
    fraud_rate,
    last_success
FROM source_health
WHERE is_active = TRUE
ORDER BY priority DESC, avg_yield DESC;
```

---

## 🔍 ASN-BASED DETECTION (Task #2)

### Carrier ASN Mapping
```rust
// src/asn_detector.rs
use std::collections::HashMap;

pub struct ASNDetector {
    carrier_asns: HashMap<u32, &'static str>,
    residential_asns: HashMap<u32, &'static str>,
}

impl ASNDetector {
    pub fn new() -> Self {
        let mut carrier_asns = HashMap::new();
        
        // Major US Carriers
        carrier_asns.insert(7018, "AT&T");
        carrier_asns.insert(20057, "AT&T Mobility");
        carrier_asns.insert(701, "Verizon");
        carrier_asns.insert(22394, "Verizon Wireless");
        carrier_asns.insert(6167, "Verizon Business");
        carrier_asns.insert(21928, "T-Mobile USA");
        carrier_asns.insert(21929, "T-Mobile");
        carrier_asns.insert(23567, "Sprint");
        carrier_asns.insert(26492, "Sprint PCS");
        
        // International Carriers
        carrier_asns.insert(45029, "China Mobile");
        carrier_asns.insert(9808, "China Mobile Guangdong");
        carrier_asns.insert(56046, "China Mobile Communications");
        carrier_asns.insert(38266, "Vodafone India");
        carrier_asns.insert(55410, "Telefonica Brasil");
        carrier_asns.insert(12353, "Vodafone Italy");
        carrier_asns.insert(3209, "Vodafone Germany");
        carrier_asns.insert(25173, "Telecom Italia Mobile");
        
        let mut residential_asns = HashMap::new();
        
        // Major US ISPs
        residential_asns.insert(7922, "Comcast");
        residential_asns.insert(33650, "Comcast Business");
        residential_asns.insert(33651, "Comcast Cable");
        residential_asns.insert(20115, "Charter/Spectrum");
        residential_asns.insert(11426, "Charter Communications");
        residential_asns.insert(12271, "Charter/Spectrum");
        residential_asns.insert(22773, "Cox Communications");
        residential_asns.insert(7015, "Frontier Communications");
        residential_asns.insert(11351, "Charter Comm");
        residential_asns.insert(20001, "Optimum/Altice");
        
        // International Residential
        residential_asns.insert(6805, "Telefonica Germany");
        residential_asns.insert(3320, "Deutsche Telekom");
        residential_asns.insert(5089, "Virgin Media (UK)");
        residential_asns.insert(13285, "TalkTalk (UK)");
        residential_asns.insert(4713, "NTT Communications (Japan)");
        residential_asns.insert(9318, "SK Broadband (Korea)");
        
        Self {
            carrier_asns,
            residential_asns,
        }
    }
    
    pub fn is_mobile(&self, asn: u32) -> Option<&'static str> {
        self.carrier_asns.get(&asn).copied()
    }
    
    pub fn is_residential(&self, asn: u32) -> Option<&'static str> {
        self.residential_asns.get(&asn).copied()
    }
    
    pub async fn fetch_asn_data(&self, ip: &str) -> Result<ASNData, Error> {
        // Try ipinfo.io first
        let url = format!("https://ipinfo.io/{}/json", ip);
        let resp: serde_json::Value = reqwest::get(&url)
            .await?
            .json()
            .await?;
        
        if let Some(org) = resp.get("org").and_then(|v| v.as_str()) {
            // Parse "AS7018 AT&T Services, Inc."
            let asn: u32 = org.split_whitespace()
                .next()
                .and_then(|s| s.trim_start_matches("AS").parse().ok())
                .unwrap_or(0);
            
            let org_name = org.split_whitespace().skip(1).collect::<Vec<_>>().join(" ");
            
            return Ok(ASNData {
                asn,
                org: org_name,
                is_mobile: self.is_mobile(asn).is_some(),
                is_residential: self.is_residential(asn).is_some(),
                carrier_name: self.is_mobile(asn).map(|s| s.to_string()),
                isp_name: self.is_residential(asn).map(|s| s.to_string()),
            });
        }
        
        // Fallback to ipapi.co
        let fallback_url = format!("https://ipapi.co/{}/json/", ip);
        let fb_resp: serde_json::Value = reqwest::get(&fallback_url)
            .await?
            .json()
            .await?;
        
        let asn = fb_resp.get("asn")
            .and_then(|v| v.as_str())
            .and_then(|s| s.trim_start_matches("AS").parse().ok())
            .unwrap_or(0);
        
        let org = fb_resp.get("org").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
        
        Ok(ASNData {
            asn,
            org,
            is_mobile: self.is_mobile(asn).is_some(),
            is_residential: self.is_residential(asn).is_some(),
            carrier_name: self.is_mobile(asn).map(|s| s.to_string()),
            isp_name: self.is_residential(asn).map(|s| s.to_string()),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ASNData {
    pub asn: u32,
    pub org: String,
    pub is_mobile: bool,
    pub is_residential: bool,
    pub carrier_name: Option<String>,
    pub isp_name: Option<String>,
}
```

### Integration in crawler.rs
```rust
// In validate_proxy function, after geo lookup:
let asn_detector = ASNDetector::new();
let asn_data = asn_detector.fetch_asn_data(&proxy.host).await?;

// Update proxy struct
proxy.carrier_asn = Some(asn_data.asn as i32);
proxy.is_mobile = asn_data.is_mobile;
proxy.is_residential = asn_data.is_residential;
proxy.isp_name = asn_data.isp_name.or(Some(asn_data.org.clone()));

// If mobile, tag for rotation testing
if proxy.is_mobile {
    info!("🚀 MOBILE PROXY DETECTED: {} ({} - ASN {})", 
          proxy.full_address(), 
          asn_data.carrier_name.unwrap_or_default(),
          asn_data.asn);
}

// If residential, tag for premium status
if proxy.is_residential {
    info!("🏠 RESIDENTIAL PROXY DETECTED: {} ({} - ASN {})", 
          proxy.full_address(),
          asn_data.isp_name.clone().unwrap_or_default(),
          asn_data.asn);
}
```

---

## 📚 PROXY SOURCES (Task #3)

### TheSpeedX Integration
```rust
// Add to main.rs PROXY_SOURCES
const SPEEDX_SOURCES: &[(&str, &str)] = &[
    ("https://raw.githubusercontent.com/TheSpeedX/SOCKS-List/master/socks5.txt", "socks5"),
    ("https://raw.githubusercontent.com/TheSpeedX/SOCKS-List/master/socks4.txt", "socks4"),
    ("https://raw.githubusercontent.com/TheSpeedX/SOCKS-List/master/http.txt", "http"),
];

// Clarketm with metadata parsing
async fn fetch_clarketm() -> Result<Vec<RawProxy>, Error> {
    let url = "https://raw.githubusercontent.com/clarketm/proxy-list/master/proxy-list-raw.txt";
    let text = reqwest::get(url).await?.text().await?;
    
    let mut proxies = Vec::new();
    for line in text.lines() {
        // Format: IP:PORT or just IP:PORT
        if let Some((host_port, _rest)) = line.split_once(' ') {
            if let Some((host, port)) = host_port.split_once(':') {
                proxies.push(RawProxy {
                    host: host.to_string(),
                    port: port.parse().unwrap_or(8080),
                    protocol: "http".to_string(),
                });
            }
        } else if let Some((host, port)) = line.split_once(':') {
            proxies.push(RawProxy {
                host: host.to_string(),
                port: port.parse().unwrap_or(8080),
                protocol: "http".to_string(),
            });
        }
    }
    
    Ok(proxies)
}
```

### fate0/proxylist (JSON format)
```rust
async fn fetch_fate0() -> Result<Vec<RawProxy>, Error> {
    let url = "http://proxylist.fatezero.org/proxy.list";
    let text = reqwest::get(url).await?.text().await?;
    
    let mut proxies = Vec::new();
    for line in text.lines() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            if let (Some(host), Some(port), Some(ptype)) = (
                json.get("host").and_then(|v| v.as_str()),
                json.get("port").and_then(|v| v.as_u64()),
                json.get("type").and_then(|v| v.as_str()),
            ) {
                proxies.push(RawProxy {
                    host: host.to_string(),
                    port: port as u16,
                    protocol: ptype.to_lowercase(),
                });
            }
        }
    }
    
    Ok(proxies)
}
```

### Complete Source List (50+)
```rust
// src/sources.rs - Centralized source management
pub const ALL_SOURCES: &[(&str, &str, &str)] = &[
    // TheSpeedX (45K proxies, hourly updates)
    ("https://raw.githubusercontent.com/TheSpeedX/SOCKS-List/master/socks5.txt", "socks5", "speedx"),
    ("https://raw.githubusercontent.com/TheSpeedX/SOCKS-List/master/socks4.txt", "socks4", "speedx"),
    ("https://raw.githubusercontent.com/TheSpeedX/SOCKS-List/master/http.txt", "http", "speedx"),
    
    // Clarketm (quality filtered)
    ("https://raw.githubusercontent.com/clarketm/proxy-list/master/proxy-list-raw.txt", "http", "clarketm"),
    
    // fate0 (JSON, 15min updates)
    ("http://proxylist.fatezero.org/proxy.list", "mixed", "fate0"),
    
    // Zaeem20
    ("https://raw.githubusercontent.com/Zaeem20/FREE_PROXIES_LIST/master/http.txt", "http", "zaeem20"),
    ("https://raw.githubusercontent.com/Zaeem20/FREE_PROXIES_LIST/master/https.txt", "https", "zaeem20"),
    ("https://raw.githubusercontent.com/Zaeem20/FREE_PROXIES_LIST/master/socks4.txt", "socks4", "zaeem20"),
    ("https://raw.githubusercontent.com/Zaeem20/FREE_PROXIES_LIST/master/socks5.txt", "socks5", "zaeem20"),
    
    // monosans
    ("https://raw.githubusercontent.com/monosans/proxy-list/main/proxies/http.txt", "http", "monosans"),
    ("https://raw.githubusercontent.com/monosans/proxy-list/main/proxies/socks4.txt", "socks4", "monosans"),
    ("https://raw.githubusercontent.com/monosans/proxy-list/main/proxies/socks5.txt", "socks5", "monosans"),
    
    // sunny9577
    ("https://raw.githubusercontent.com/sunny9577/proxy-scraper/master/proxies.txt", "http", "sunny9577"),
    
    // iw4p
    ("https://raw.githubusercontent.com/iw4p/proxy-list/main/socks5.txt", "socks5", "iw4p"),
    ("https://raw.githubusercontent.com/iw4p/proxy-list/main/http.txt", "http", "iw4p"),
    
    // ProxyScrape API
    ("https://api.proxyscrape.com/v2/?request=get&protocol=http&timeout=10000&country=all", "http", "proxyscrape"),
    ("https://api.proxyscrape.com/v2/?request=get&protocol=socks5&timeout=10000&country=all", "socks5", "proxyscrape"),
    ("https://api.proxyscrape.com/v2/?request=get&protocol=socks4&timeout=10000&country=all", "socks4", "proxyscrape"),
    
    // Additional high-quality sources
    ("https://raw.githubusercontent.com/ShiftyTR/Proxy-List/master/http.txt", "http", "shiftytr"),
    ("https://raw.githubusercontent.com/ShiftyTR/Proxy-List/master/socks5.txt", "socks5", "shiftytr"),
    ("https://raw.githubusercontent.com/roosterkid/openproxylist/main/HTTPS_RAW.txt", "https", "roosterkid"),
    ("https://raw.githubusercontent.com/roosterkid/openproxylist/main/SOCKS4_RAW.txt", "socks4", "roosterkid"),
    ("https://raw.githubusercontent.com/roosterkid/openproxylist/main/SOCKS5_RAW.txt", "socks5", "roosterkid"),
    ("https://raw.githubusercontent.com/jetkai/proxy-list/main/online-proxies/txt/proxies-http.txt", "http", "jetkai"),
    ("https://raw.githubusercontent.com/jetkai/proxy-list/main/online-proxies/txt/proxies-https.txt", "https", "jetkai"),
    ("https://raw.githubusercontent.com/jetkai/proxy-list/main/online-proxies/txt/proxies-socks4.txt", "socks4", "jetkai"),
    ("https://raw.githubusercontent.com/jetkai/proxy-list/main/online-proxies/txt/proxies-socks5.txt", "socks5", "jetkai"),
    ("https://raw.githubusercontent.com/mmpx12/proxy-list/master/http.txt", "http", "mmpx12"),
    ("https://raw.githubusercontent.com/mmpx12/proxy-list/master/https.txt", "https", "mmpx12"),
    ("https://raw.githubusercontent.com/mmpx12/proxy-list/master/socks4.txt", "socks4", "mmpx12"),
    ("https://raw.githubusercontent.com/mmpx12/proxy-list/master/socks5.txt", "socks5", "mmpx12"),
];
```

---

## ✅ 5-STAGE ELITE VALIDATION (Task #5)

### Complete Pipeline
```rust
// src/validator.rs
pub struct EliteValidator {
    asn_detector: ASNDetector,
    fraud_checker: FraudChecker,
    http_client: reqwest::Client,
}

impl EliteValidator {
    pub async fn validate_elite(&self, proxy: &Proxy) -> ValidationResult {
        let mut result = ValidationResult::default();
        let mut passed_stages = 0;
        
        // STAGE 1: ASN Verification
        info!("Stage 1/5: ASN verification for {}", proxy.full_address());
        match self.verify_asn(proxy).await {
            Ok(asn_data) => {
                result.asn_data = Some(asn_data.clone());
                if asn_data.is_mobile || asn_data.is_residential {
                    passed_stages += 1;
                    result.stage1_passed = true;
                    info!("✅ Stage 1 PASSED: {} (ASN: {})", 
                          if asn_data.is_mobile { "Mobile" } else { "Residential" },
                          asn_data.asn);
                } else {
                    warn!("❌ Stage 1 FAILED: Datacenter ASN {}", asn_data.asn);
                }
            }
            Err(e) => warn!("❌ Stage 1 ERROR: {}", e),
        }
        
        // STAGE 2: Rotation Test (mobile only)
        if result.asn_data.as_ref().map(|a| a.is_mobile).unwrap_or(false) {
            info!("Stage 2/5: Rotation test (mobile) for {}", proxy.full_address());
            match self.test_rotation(proxy).await {
                Ok(rotation_count) => {
                    if rotation_count >= 2 {
                        passed_stages += 1;
                        result.stage2_passed = true;
                        result.rotation_verified = true;
                        info!("✅ Stage 2 PASSED: IP changed {} times", rotation_count);
                    } else {
                        warn!("❌ Stage 2 FAILED: Only {} rotations (need 2+)", rotation_count);
                    }
                }
                Err(e) => warn!("❌ Stage 2 ERROR: {}", e),
            }
        } else {
            // Skip for non-mobile, auto-pass
            passed_stages += 1;
            result.stage2_passed = true;
        }
        
        // STAGE 3: Browsing Test
        info!("Stage 3/5: Browsing test for {}", proxy.full_address());
        match self.test_browsing(proxy).await {
            Ok(browse_ok) => {
                if browse_ok {
                    passed_stages += 1;
                    result.stage3_passed = true;
                    result.browser_compatible = true;
                    info!("✅ Stage 3 PASSED: Google & Amazon accessible");
                } else {
                    warn!("❌ Stage 3 FAILED: Blocked or captcha detected");
                }
            }
            Err(e) => warn!("❌ Stage 3 ERROR: {}", e),
        }
        
        // STAGE 4: Fraud Detection
        info!("Stage 4/5: Fraud check for {}", proxy.full_address());
        match self.check_fraud(proxy).await {
            Ok((fraud_score, abuse_score)) => {
                result.fraud_score = fraud_score;
                result.abuse_score = abuse_score;
                if fraud_score < 0.5 && abuse_score < 50.0 {
                    passed_stages += 1;
                    result.stage4_passed = true;
                    info!("✅ Stage 4 PASSED: Fraud={:.2}, Abuse={:.0}%", fraud_score, abuse_score);
                } else {
                    warn!("❌ Stage 4 FAILED: Fraud={:.2}, Abuse={:.0}%", fraud_score, abuse_score);
                }
            }
            Err(e) => warn!("❌ Stage 4 ERROR: {}", e),
        }
        
        // STAGE 5: Device Simulation
        info!("Stage 5/5: Device simulation for {}", proxy.full_address());
        match self.test_devices(proxy).await {
            Ok(compatible_count) => {
                if compatible_count >= 2 {
                    passed_stages += 1;
                    result.stage5_passed = true;
                    info!("✅ Stage 5 PASSED: {}/3 devices compatible", compatible_count);
                } else {
                    warn!("❌ Stage 5 FAILED: Only {}/3 devices", compatible_count);
                }
            }
            Err(e) => warn!("❌ Stage 5 ERROR: {}", e),
        }
        
        // Final scoring
        result.stages_passed = passed_stages;
        result.is_elite = passed_stages >= 4; // Need 4/5 to pass
        
        if result.is_elite {
            result.stability_score = 0.95;
            result.anonymity_level = "elite".to_string();
            info!("🌟 ELITE PROXY: {}/{} stages passed", passed_stages, 5);
        } else if passed_stages >= 3 {
            result.stability_score = 0.7;
            result.anonymity_level = "good".to_string();
            info!("👍 GOOD PROXY: {}/{} stages passed", passed_stages, 5);
        } else {
            result.stability_score = 0.3;
            result.anonymity_level = "poor".to_string();
            warn!("👎 POOR PROXY: {}/{} stages passed", passed_stages, 5);
        }
        
        result
    }
    
    async fn test_rotation(&self, proxy: &Proxy) -> Result<u32, Error> {
        let mut ips = Vec::new();
        
        for i in 1..=3 {
            info!("  Rotation check {}/3...", i);
            
            let client = self.build_proxy_client(proxy)?;
            let resp = client.get("https://ipinfo.io/json")
                .timeout(Duration::from_secs(10))
                .send()
                .await?;
            
            let json: serde_json::Value = resp.json().await?;
            if let Some(ip) = json.get("ip").and_then(|v| v.as_str()) {
                ips.push(ip.to_string());
            }
            
            if i < 3 {
                tokio::time::sleep(Duration::from_secs(120)).await; // 2min intervals
            }
        }
        
        let unique_ips = ips.iter().collect::<std::collections::HashSet<_>>().len();
        Ok(unique_ips as u32)
    }
    
    async fn test_browsing(&self, proxy: &Proxy) -> Result<bool, Error> {
        let client = self.build_proxy_client(proxy)?;
        
        // Test Google
        let google_resp = client.get("https://www.google.com")
            .timeout(Duration::from_secs(10))
            .send()
            .await?;
        
        if google_resp.status() != 200 {
            return Ok(false);
        }
        
        let body = google_resp.text().await?;
        if body.contains("captcha") || body.contains("unusual traffic") {
            return Ok(false);
        }
        
        // Test Amazon
        let amazon_resp = client.get("https://www.amazon.com")
            .timeout(Duration::from_secs(10))
            .send()
            .await?;
        
        Ok(amazon_resp.status() == 200)
    }
    
    async fn check_fraud(&self, proxy: &Proxy) -> Result<(f64, f64), Error> {
        // Primary: scamalytics (existing)
        let fraud_score = self.fraud_checker.check_scamalytics(&proxy.host).await
            .unwrap_or(0.5);
        
        // Fallback: AbuseIPDB
        let abuse_score = self.check_abuseipdb(&proxy.host).await
            .unwrap_or(0.0);
        
        Ok((fraud_score, abuse_score))
    }
    
    async fn check_abuseipdb(&self, ip: &str) -> Result<f64, Error> {
        // Requires API key (free tier: 1000 req/day)
        let api_key = std::env::var("ABUSEIPDB_API_KEY").ok();
        
        if let Some(key) = api_key {
            let url = format!("https://api.abuseipdb.com/api/v2/check?ipAddress={}", ip);
            let resp = self.http_client
                .get(&url)
                .header("Key", key)
                .send()
                .await?;
            
            let json: serde_json::Value = resp.json().await?;
            if let Some(score) = json.get("data")
                .and_then(|d| d.get("abuseConfidenceScore"))
                .and_then(|s| s.as_f64()) 
            {
                return Ok(score);
            }
        }
        
        Ok(0.0)
    }
    
    async fn test_devices(&self, proxy: &Proxy) -> Result<u32, Error> {
        let user_agents = vec![
            "Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X) AppleWebKit/605.1.15", // iOS
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/118.0.0.0", // Desktop
            "curl/7.68.0", // CLI
        ];
        
        let mut compatible = 0;
        
        for (idx, ua) in user_agents.iter().enumerate() {
            let client = reqwest::Client::builder()
                .proxy(reqwest::Proxy::all(&proxy.full_address())?)
                .user_agent(*ua)
                .timeout(Duration::from_secs(8))
                .build()?;
            
            match client.get("https://httpbin.org/user-agent").send().await {
                Ok(resp) if resp.status() == 200 => {
                    compatible += 1;
                    info!("  Device {}/3: OK", idx + 1);
                }
                _ => info!("  Device {}/3: FAILED", idx + 1),
            }
        }
        
        Ok(compatible)
    }
}

#[derive(Debug, Default)]
pub struct ValidationResult {
    pub stage1_passed: bool,
    pub stage2_passed: bool,
    pub stage3_passed: bool,
    pub stage4_passed: bool,
    pub stage5_passed: bool,
    pub stages_passed: u32,
    pub is_elite: bool,
    pub stability_score: f64,
    pub anonymity_level: String,
    pub asn_data: Option<ASNData>,
    pub rotation_verified: bool,
    pub browser_compatible: bool,
    pub fraud_score: f64,
    pub abuse_score: f64,
}
```

---

## 🚀 EXECUTION PHASES

### Phase 1: Foundation (Week 1)
**Tasks**: #1 (DB), #2 (ASN), #3 (Sources)  
**Time**: 8-10 hours  
**Commands**:
```bash
# Stop service
sudo systemctl stop omega9-nexus

# Run migration
psql -U omega9 -d omega9_proxies -f migrations/add_premium_fields.sql

# Update code
git checkout -b godmode-phase1
# Implement ASN detector, add sources, update validation
cargo build --release
cargo test

# Deploy
docker build -t omega9-nexus:godmode .
sudo systemctl start omega9-nexus
```

**Validation**:
```sql
-- Verify schema
\d proxies;

-- Check for mobile/residential tags
SELECT 
    COUNT(*) FILTER (WHERE is_mobile) as mobile_count,
    COUNT(*) FILTER (WHERE is_residential) as residential_count,
    COUNT(*) as total
FROM proxies;
```

### Phase 2: Validation Enhancement (Week 2)
**Tasks**: #5 (Elite validation), #13 (Fraud), #14 (Anomaly), #15 (Rotation)  
**Time**: 10-12 hours  

### Phase 3: Scale & Discovery (Week 2-3)
**Tasks**: #8 (Concurrency), #9 (Scraping), #10 (20+ sources), #17-19 (Discovery)  
**Time**: 12-15 hours  

### Phase 4: Production Ready (Week 3-4)
**Tasks**: #21-22 (Metrics), #25-26 (API/Dashboard), #28 (CI/CD), #30 (Benchmark)  
**Time**: 10-12 hours  

---

## 📈 SUCCESS METRICS

### Before GOD MODE
```
Total Proxies: ~500/day
Mobile Proxies: 2-3 total
Residential Proxies: 1 total
Validation Throughput: ~50/min
Source Success Rate: 60%
Ban Rate: 40%
```

### After GOD MODE (Target)
```
Total Proxies: 1,000-5,000/hour
Mobile Proxies: 50-200/session
Residential Proxies: 100-500/session
Validation Throughput: 500-1000/min
Source Success Rate: 95%+
Ban Rate: <5%
Elite Accuracy: 95%+
Uptime: 99%+
```

---

## 🔧 CRITICAL DEPENDENCIES

### Cargo.toml Additions
```toml
[dependencies]
# Existing...
thirtyfour = "0.31"           # Browser automation
scraper = "0.17"              # HTML parsing
select = "0.6"                # CSS selectors
fake-useragent = "1.4"        # UA rotation
tokio-retry = "0.3"           # Exponential backoff
rayon = "1.7"                 # Parallel processing
roux = "2.2"                  # Reddit API
prometheus = "0.13"           # Metrics
ollama-rs = "0.1"             # Optional: Local LLM
rusoto_ec2 = "0.48"           # Optional: AWS
```

### Environment Variables
```bash
# .env additions
ABUSEIPDB_API_KEY=your_key_here
IPQUALITYSCORE_API_KEY=your_key_here
REDDIT_CLIENT_ID=your_id
REDDIT_CLIENT_SECRET=your_secret
GITHUB_TOKEN=your_token  # For API rate limit: 5000/hour
MAX_CONCURRENT=1000
ENABLE_BROWSER_EMULATION=true
ENABLE_AI_DISCOVERY=false  # Set true when ready
```

---

## 🎯 QUICK START CHECKLIST

- [ ] Backup database: `pg_dump omega9_proxies > backup.sql`
- [ ] Run migration: `psql -f migrations/add_premium_fields.sql`
- [ ] Implement Task #1: Database schema ✅
- [ ] Implement Task #2: ASN detection ✅
- [ ] Implement Task #3: Add 50+ sources ✅
- [ ] Test ASN tagging: Should see mobile/residential flags
- [ ] Run hunt: Expect 10x volume from new sources
- [ ] Implement Task #5: Elite validation pipeline
- [ ] Benchmark: Compare before/after metrics
- [ ] Document results in benchmark.md
- [ ] Continue with Phase 2-4 tasks

---

## 📝 NOTES & TIPS

1. **Always work in feature branches**: `git checkout -b feature-name`
2. **Test each task independently**: Don't stack untested changes
3. **Monitor resource usage**: `htop`, `docker stats`
4. **Check logs frequently**: `journalctl -u omega9-nexus -f`
5. **Validate DB changes**: Run test queries after migrations
6. **Commit frequently**: Small, atomic commits with clear messages
7. **Update this doc**: Add learnings, gotchas, optimizations

---

**Last Updated**: 2025-10-30  
**Status**: Ready to implement Phase 1  
**Next Step**: Task #1 - Database Schema Migration
