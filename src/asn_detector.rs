use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::{Result, anyhow};
use tracing::{warn, error};

/// ASN data retrieved from IP lookup services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ASNData {
    pub asn: u32,
    pub org: String,
    pub is_mobile: bool,
    pub is_residential: bool,
    pub carrier_name: Option<String>,
    pub isp_name: Option<String>,
}

/// ASN Detector - THE core differentiator for mobile/residential vs datacenter detection
pub struct ASNDetector {
    carrier_asns: HashMap<u32, &'static str>,
    residential_asns: HashMap<u32, &'static str>,
    client: reqwest::Client,
}

impl ASNDetector {
    pub fn new() -> Self {
        let mut carrier_asns = HashMap::new();
        let mut residential_asns = HashMap::new();

        // Major Mobile Carriers - USA
        carrier_asns.insert(7018, "AT&T");
        carrier_asns.insert(20057, "AT&T Mobility");
        carrier_asns.insert(701, "Verizon");
        carrier_asns.insert(22394, "Verizon Wireless");
        carrier_asns.insert(6167, "Verizon Business");
        carrier_asns.insert(21928, "T-Mobile USA");
        carrier_asns.insert(21929, "T-Mobile");
        carrier_asns.insert(23567, "Sprint");
        carrier_asns.insert(26492, "Sprint PCS");
        
        // Major Mobile Carriers - International
        carrier_asns.insert(45029, "China Mobile"); // CN
        carrier_asns.insert(9808, "China Mobile Guangdong");
        carrier_asns.insert(56046, "China Mobile");
        carrier_asns.insert(38266, "Vodafone"); // EU/UK
        carrier_asns.insert(12353, "Vodafone Italy");
        carrier_asns.insert(3209, "Vodafone Germany");
        carrier_asns.insert(10207, "Orange France");
        carrier_asns.insert(5410, "Bouygues Telecom"); // FR
        carrier_asns.insert(15557, "SFR"); // FR
        carrier_asns.insert(31334, "Vodafone Spain");
        carrier_asns.insert(6739, "ONO Spain");
        carrier_asns.insert(12430, "Vodafone UK");
        carrier_asns.insert(2856, "BT Group"); // UK
        carrier_asns.insert(5089, "Virgin Media"); // UK
        carrier_asns.insert(4713, "NTT DoCoMo"); // JP
        carrier_asns.insert(9605, "NTT Communications"); // JP
        carrier_asns.insert(17676, "SoftBank"); // JP
        carrier_asns.insert(23655, "KDDI"); // JP
        carrier_asns.insert(45727, "Reliance Jio"); // IN
        carrier_asns.insert(55836, "Reliance Jio Infocomm");
        carrier_asns.insert(24560, "Bharti Airtel"); // IN
        carrier_asns.insert(9498, "Bharti Airtel");
        carrier_asns.insert(38266, "Vodafone Idea"); // IN
        
        // Major Residential ISPs - USA
        residential_asns.insert(7922, "Comcast");
        residential_asns.insert(33650, "Comcast Cable");
        residential_asns.insert(33651, "Comcast Business");
        residential_asns.insert(33657, "Comcast Cable Communications");
        residential_asns.insert(33660, "Comcast Cable Communications");
        residential_asns.insert(20115, "Charter Communications"); // Spectrum
        residential_asns.insert(11426, "TWC (now Spectrum)");
        residential_asns.insert(12271, "Charter Fiberlink");
        residential_asns.insert(10796, "Charter");
        residential_asns.insert(22773, "Cox Communications");
        residential_asns.insert(22874, "Cox Communications");
        residential_asns.insert(7015, "Frontier Communications");
        residential_asns.insert(5650, "Frontier");
        residential_asns.insert(11351, "Charter");
        residential_asns.insert(12271, "Time Warner Cable");
        
        // Major Residential ISPs - International
        residential_asns.insert(5089, "Virgin Media"); // UK
        residential_asns.insert(6830, "Liberty Global"); // EU
        residential_asns.insert(3320, "Deutsche Telekom"); // DE
        residential_asns.insert(6805, "Telefonica Germany");
        residential_asns.insert(3352, "Telefonica Spain");
        residential_asns.insert(12479, "Orange France");
        residential_asns.insert(5410, "Bouygues Telecom");
        residential_asns.insert(8402, "Rostelecom"); // RU
        residential_asns.insert(12389, "Rostelecom");
        residential_asns.insert(4134, "China Telecom"); // CN
        residential_asns.insert(4837, "China Unicom");
        residential_asns.insert(23969, "TOT (Thailand)");

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            carrier_asns,
            residential_asns,
            client,
        }
    }

    /// Fetch ASN data for an IP address with dual fallback (ipinfo.io → ipapi.co)
    pub async fn fetch_asn_data(&self, ip: &str) -> Result<ASNData> {
        // Try ipinfo.io first (primary)
        match self.fetch_from_ipinfo(ip).await {
            Ok(data) => return Ok(data),
            Err(e) => {
                warn!("ipinfo.io failed for {}: {}, trying ipapi.co", ip, e);
            }
        }

        // Fallback to ipapi.co
        match self.fetch_from_ipapi(ip).await {
            Ok(data) => Ok(data),
            Err(e) => {
                error!("Both ASN providers failed for {}: {}", ip, e);
                Err(anyhow!("All ASN providers failed: {}", e))
            }
        }
    }

    /// Fetch from ipinfo.io (primary provider)
    async fn fetch_from_ipinfo(&self, ip: &str) -> Result<ASNData> {
        let url = format!("https://ipinfo.io/{}/json", ip);
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow!("ipinfo.io request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!("ipinfo.io returned status: {}", response.status()));
        }

        let json: serde_json::Value = response.json()
            .await
            .map_err(|e| anyhow!("Failed to parse ipinfo.io JSON: {}", e))?;

        // Parse ASN from "org" field (format: "AS7018 AT&T Services, Inc.")
        let org_str = json.get("org")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("No 'org' field in ipinfo.io response"))?;

        let asn = self.parse_asn_from_org(org_str)?;
        
        self.build_asn_data(asn, org_str)
    }

    /// Fetch from ipapi.co (fallback provider)
    async fn fetch_from_ipapi(&self, ip: &str) -> Result<ASNData> {
        let url = format!("https://ipapi.co/{}/json/", ip);
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow!("ipapi.co request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!("ipapi.co returned status: {}", response.status()));
        }

        let json: serde_json::Value = response.json()
            .await
            .map_err(|e| anyhow!("Failed to parse ipapi.co JSON: {}", e))?;

        let asn = json.get("asn")
            .and_then(|v| v.as_str())
            .and_then(|s| s.strip_prefix("AS"))
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("No valid 'asn' field in ipapi.co response"))?;

        let org = json.get("org")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");

        self.build_asn_data(asn, org)
    }

    /// Parse ASN number from org string (format: "AS7018 AT&T Services, Inc.")
    fn parse_asn_from_org(&self, org: &str) -> Result<u32> {
        if let Some(asn_part) = org.split_whitespace().next() {
            if let Some(asn_str) = asn_part.strip_prefix("AS") {
                if let Ok(asn) = asn_str.parse::<u32>() {
                    return Ok(asn);
                }
            }
        }
        Err(anyhow!("Failed to parse ASN from org: {}", org))
    }

    /// Build ASNData struct with mobile/residential classification
    fn build_asn_data(&self, asn: u32, org: &str) -> Result<ASNData> {
        let is_mobile = self.carrier_asns.contains_key(&asn);
        let is_residential = self.residential_asns.contains_key(&asn);
        
        let carrier_name = if is_mobile {
            self.carrier_asns.get(&asn).map(|s| s.to_string())
        } else {
            None
        };

        let isp_name = if is_residential {
            self.residential_asns.get(&asn).map(|s| s.to_string())
        } else {
            None
        };

        Ok(ASNData {
            asn,
            org: org.to_string(),
            is_mobile,
            is_residential,
            carrier_name,
            isp_name,
        })
    }

    /// Check if ASN is a known mobile carrier
    pub fn is_carrier_asn(&self, asn: u32) -> bool {
        self.carrier_asns.contains_key(&asn)
    }

    /// Check if ASN is a known residential ISP
    pub fn is_residential_asn(&self, asn: u32) -> bool {
        self.residential_asns.contains_key(&asn)
    }

    /// Get carrier name for ASN
    pub fn get_carrier_name(&self, asn: u32) -> Option<&str> {
        self.carrier_asns.get(&asn).copied()
    }

    /// Get ISP name for ASN
    pub fn get_isp_name(&self, asn: u32) -> Option<&str> {
        self.residential_asns.get(&asn).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_carrier_asn_detection() {
        let detector = ASNDetector::new();
        
        // USA carriers
        assert!(detector.is_carrier_asn(7018)); // AT&T
        assert!(detector.is_carrier_asn(701)); // Verizon
        assert!(detector.is_carrier_asn(21928)); // T-Mobile
        
        // International carriers
        assert!(detector.is_carrier_asn(45029)); // China Mobile
        assert!(detector.is_carrier_asn(38266)); // Vodafone
        
        // Not a carrier
        assert!(!detector.is_carrier_asn(99999));
    }

    #[test]
    fn test_residential_asn_detection() {
        let detector = ASNDetector::new();
        
        // USA ISPs
        assert!(detector.is_residential_asn(7922)); // Comcast
        assert!(detector.is_residential_asn(20115)); // Spectrum
        assert!(detector.is_residential_asn(22773)); // Cox
        
        // International ISPs
        assert!(detector.is_residential_asn(3320)); // Deutsche Telekom
        assert!(detector.is_residential_asn(4134)); // China Telecom
        
        // Not residential
        assert!(!detector.is_residential_asn(99999));
    }

    #[test]
    fn test_parse_asn_from_org() {
        let detector = ASNDetector::new();
        
        assert_eq!(detector.parse_asn_from_org("AS7018 AT&T Services, Inc.").unwrap(), 7018);
        assert_eq!(detector.parse_asn_from_org("AS701 Verizon").unwrap(), 701);
        assert!(detector.parse_asn_from_org("Invalid format").is_err());
    }

    #[tokio::test]
    async fn test_asn_data_classification() {
        let detector = ASNDetector::new();
        
        // Test mobile carrier data
        let data = detector.build_asn_data(7018, "AS7018 AT&T Services, Inc.").unwrap();
        assert!(data.is_mobile);
        assert!(!data.is_residential);
        assert_eq!(data.carrier_name, Some("AT&T".to_string()));
        
        // Test residential ISP data
        let data = detector.build_asn_data(7922, "AS7922 Comcast Cable").unwrap();
        assert!(!data.is_mobile);
        assert!(data.is_residential);
        assert_eq!(data.isp_name, Some("Comcast".to_string()));
        
        // Test datacenter (neither)
        let data = detector.build_asn_data(99999, "AS99999 Random Datacenter").unwrap();
        assert!(!data.is_mobile);
        assert!(!data.is_residential);
        assert_eq!(data.carrier_name, None);
        assert_eq!(data.isp_name, None);
    }
}
