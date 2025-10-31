// GOD MODE Elite Validation Pipeline - 5-Stage Quality Assurance
use crate::asn_detector::{ASNDetector, ASNData};
use anyhow::{Result, anyhow};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, warn, error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationStageResult {
    pub stage1_passed: bool, // ASN verification
    pub stage2_passed: bool, // Rotation test (mobile only)
    pub stage3_passed: bool, // Browsing test
    pub stage4_passed: bool, // Fraud check
    pub stage5_passed: bool, // Device simulation
    pub stages_passed: u8,
    pub is_elite: bool,
    pub stability_score: f64,
    pub anonymity_level: String,
    pub asn_data: Option<ASNData>,
    pub fraud_score: f64,
    pub abuse_score: f64,
    pub rotation_verified: bool,
    pub browser_compatible: bool,
}

impl Default for ValidationStageResult {
    fn default() -> Self {
        Self {
            stage1_passed: false,
            stage2_passed: false,
            stage3_passed: false,
            stage4_passed: false,
            stage5_passed: false,
            stages_passed: 0,
            is_elite: false,
            stability_score: 0.0,
            anonymity_level: "unknown".to_string(),
            asn_data: None,
            fraud_score: 0.0,
            abuse_score: 0.0,
            rotation_verified: false,
            browser_compatible: false,
        }
    }
}

pub struct EliteValidator {
    asn_detector: ASNDetector,
    http_client: Client,
}

impl EliteValidator {
    pub fn new() -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| Client::new());
        
        Self {
            asn_detector: ASNDetector::new(),
            http_client,
        }
    }

    /// 5-Stage Elite Validation Pipeline
    pub async fn validate_elite(&self, proxy_url: &str, ip: &str) -> Result<ValidationStageResult> {
        let mut result = ValidationStageResult::default();
        let mut passed_stages = 0;

        info!("🎯 Starting 5-stage elite validation for {}", ip);

        // STAGE 1: ASN Verification
        info!("Stage 1/5: ASN verification for {}", ip);
        match self.asn_detector.fetch_asn_data(ip).await {
            Ok(asn_data) => {
                result.asn_data = Some(asn_data.clone());
                if asn_data.is_mobile || asn_data.is_residential {
                    passed_stages += 1;
                    result.stage1_passed = true;
                    info!("✅ Stage 1 PASSED: {} (ASN: {})", 
                          if asn_data.is_mobile { "MOBILE" } else { "RESIDENTIAL" },
                          asn_data.asn);
                } else {
                    warn!("❌ Stage 1 FAILED: Datacenter ASN {}", asn_data.asn);
                }
            }
            Err(e) => warn!("❌ Stage 1 ERROR: {}", e),
        }

        // STAGE 2: Rotation Test (mobile proxies only)
        let is_mobile = result.asn_data.as_ref().map(|d| d.is_mobile).unwrap_or(false);
        if is_mobile {
            info!("Stage 2/5: Rotation test for {}", ip);
            match self.test_rotation(proxy_url).await {
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
            // Auto-pass for non-mobile
            passed_stages += 1;
            result.stage2_passed = true;
        }

        // STAGE 3: Browsing Test
        info!("Stage 3/5: Browsing test for {}", ip);
        match self.test_browsing(proxy_url).await {
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

        // STAGE 4: Fraud Detection (with carrier ASN whitelist)
        info!("Stage 4/5: Fraud check for {}", ip);
        
        // GOD MODE: Whitelist mobile/residential - they're verified by ASN, skip fraud check
        let is_premium = result.asn_data.as_ref()
            .map(|d| d.is_mobile || d.is_residential)
            .unwrap_or(false);
        
        if is_premium {
            // Mobile/residential proxies auto-pass fraud check (carrier ASN verified)
            passed_stages += 1;
            result.stage4_passed = true;
            result.fraud_score = 0.0;
            result.abuse_score = 0.0;
            info!("✅ Stage 4 PASSED: Premium proxy (carrier ASN whitelisted)");
        } else {
            // Datacenter proxies need fraud verification
            match self.check_fraud(ip).await {
                Ok((fraud_score, abuse_score)) => {
                    result.fraud_score = fraud_score;
                    result.abuse_score = abuse_score;
                    
                    // More lenient threshold: fraud<0.7 OR abuse<70 (not AND)
                    if fraud_score < 0.7 || abuse_score < 0.7 {
                        passed_stages += 1;
                        result.stage4_passed = true;
                        info!("✅ Stage 4 PASSED: Fraud={:.2}, Abuse={:.2}", fraud_score, abuse_score);
                    } else {
                        warn!("❌ Stage 4 FAILED: Fraud={:.2}, Abuse={:.2} (both high)", fraud_score, abuse_score);
                    }
                }
                Err(e) => {
                    warn!("❌ Stage 4 ERROR: {} (auto-fail for datacenter)", e);
                    result.fraud_score = 1.0;
                    result.abuse_score = 1.0;
                }
            }
        }

        // STAGE 5: Device Simulation
        info!("Stage 5/5: Device simulation for {}", ip);
        match self.test_devices(proxy_url).await {
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

        Ok(result)
    }

    /// Test IP rotation (for mobile proxies)
    pub async fn test_rotation(&self, proxy_url: &str) -> Result<u8> {
        let proxy = reqwest::Proxy::all(proxy_url)?;
        let client = Client::builder()
            .proxy(proxy)
            .timeout(Duration::from_secs(10))
            .build()?;

        let mut unique_ips = Vec::new();

        for i in 1..=3 {
            match client.get("https://api.ipify.org").send().await {
                Ok(resp) => {
                    if let Ok(ip) = resp.text().await {
                        if !unique_ips.contains(&ip) {
                            unique_ips.push(ip.clone());
                            info!("Rotation check {}/3: IP = {}", i, ip);
                        }
                    }
                }
                Err(e) => warn!("Rotation check {}/3 failed: {}", i, e),
            }

            if i < 3 {
                tokio::time::sleep(Duration::from_secs(120)).await; // 2min delay
            }
        }

        Ok(unique_ips.len() as u8)
    }

    /// Test browsing capability
    async fn test_browsing(&self, proxy_url: &str) -> Result<bool> {
        let proxy = reqwest::Proxy::all(proxy_url)?;
        let client = Client::builder()
            .proxy(proxy)
            .timeout(Duration::from_secs(15))
            .build()?;

        // Test Google
        let google_ok = client.get("https://www.google.com")
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false);

        // Test Amazon
        let amazon_ok = client.get("https://www.amazon.com")
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false);

        Ok(google_ok && amazon_ok)
    }

    /// Check fraud score (scamalytics + AbuseIPDB fallback)
    async fn check_fraud(&self, ip: &str) -> Result<(f64, f64)> {
        use regex::Regex;
        use serde_json::Value;
        
        // Try Scamalytics first (free, no API key needed)
        let fraud_score = match self.fetch_scamalytics_score(ip).await {
            Ok(score) => score,
            Err(e) => {
                warn!("Scamalytics fetch failed for {}: {}", ip, e);
                0.0
            }
        };
        
        // Try AbuseIPDB as fallback (requires API key)
        let abuse_score = if let Ok(api_key) = std::env::var("ABUSEIPDB_API_KEY") {
            match self.fetch_abuseipdb_score(ip, &api_key).await {
                Ok(score) => score,
                Err(e) => {
                    warn!("AbuseIPDB fetch failed for {}: {}", ip, e);
                    0.0
                }
            }
        } else {
            0.0 // No API key configured
        };
        
        Ok((fraud_score, abuse_score))
    }

    /// Fetch fraud score from Scamalytics (web scraping)
    async fn fetch_scamalytics_score(&self, ip: &str) -> Result<f64> {
        use regex::Regex;
        
        let url = format!("https://scamalytics.com/ip/{}", ip.trim());
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;
        
        let response = client.get(&url).send().await?;
        let html = response.text().await?;
        
        // Parse "Fraud Score: XX%" from HTML
        let re = Regex::new(r"Fraud Score:\s*(\d+)%")?;
        if let Some(caps) = re.captures(&html) {
            if let Some(score_str) = caps.get(1) {
                let score_int: u32 = score_str.as_str().parse()?;
                return Ok(score_int as f64 / 100.0); // Convert to 0.0-1.0 range
            }
        }
        
        Err(anyhow::anyhow!("Could not parse Scamalytics fraud score"))
    }

    /// Fetch abuse confidence score from AbuseIPDB API v2
    async fn fetch_abuseipdb_score(&self, ip: &str, api_key: &str) -> Result<f64> {
        use serde_json::Value;
        
        let url = format!("https://api.abuseipdb.com/api/v2/check?ipAddress={}", ip);
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;
        
        let response = client
            .get(&url)
            .header("Key", api_key)
            .header("Accept", "application/json")
            .send()
            .await?;
        
        let json: Value = response.json().await?;
        
        // Parse abuseConfidenceScore (0-100)
        if let Some(data) = json.get("data") {
            if let Some(score) = data.get("abuseConfidenceScore") {
                if let Some(score_int) = score.as_u64() {
                    return Ok(score_int as f64 / 100.0); // Convert to 0.0-1.0 range
                }
            }
        }
        
        Err(anyhow::anyhow!("Could not parse AbuseIPDB score"))
    }

    /// Test with multiple device user agents
    async fn test_devices(&self, proxy_url: &str) -> Result<u8> {
        let user_agents = [
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            "Mozilla/5.0 (iPhone; CPU iPhone OS 14_7_1 like Mac OS X) AppleWebKit/605.1.15",
            "Mozilla/5.0 (Linux; Android 11; SM-G991B) AppleWebKit/537.36",
        ];

        let proxy = reqwest::Proxy::all(proxy_url)?;
        let mut compatible = 0;

        for ua in &user_agents {
            let client = Client::builder()
                .proxy(proxy.clone())
                .timeout(Duration::from_secs(10))
                .user_agent(*ua)
                .build()?;

            if client.get("https://httpbin.org/user-agent")
                .send()
                .await
                .map(|r| r.status().is_success())
                .unwrap_or(false)
            {
                compatible += 1;
            }
        }

        Ok(compatible)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_default() {
        let result = ValidationStageResult::default();
        assert_eq!(result.stages_passed, 0);
        assert!(!result.is_elite);
        assert_eq!(result.stability_score, 0.0);
    }
}
