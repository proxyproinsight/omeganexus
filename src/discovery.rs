use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error, info};
use urlencoding::encode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxySource {
    pub url: String,
    pub name: String,
    pub source_type: String, // github, reddit, bing, tor, static
}

/// Discover new proxy sources dynamically
pub async fn discover_new_sources(
    client: &Client,
    github_token: Option<&str>,
) -> Result<Vec<ProxySource>, Box<dyn std::error::Error + Send + Sync>> {
    let mut sources = Vec::new();

    // GitHub API search
    if let Some(token) = github_token {
        match discover_github_sources(client, token).await {
            Ok(mut gh_sources) => sources.append(&mut gh_sources),
            Err(e) => error!("GitHub discovery failed: {}", e),
        }
    }

    // Reddit search
    match discover_reddit_sources(client).await {
        Ok(mut reddit_sources) => sources.append(&mut reddit_sources),
        Err(e) => error!("Reddit discovery failed: {}", e),
    }

    // Bing search
    match discover_bing_sources(client).await {
        Ok(mut bing_sources) => sources.append(&mut bing_sources),
        Err(e) => error!("Bing discovery failed: {}", e),
    }

    // Tor darknet sources (if running via Tor proxy)
    match discover_tor_sources(client).await {
        Ok(mut tor_sources) => sources.append(&mut tor_sources),
        Err(e) => error!("Tor discovery failed: {}", e),
    }

    info!("Discovered {} new potential sources", sources.len());
    Ok(sources)
}

/// Search GitHub for proxy list repositories
async fn discover_github_sources(
    client: &Client,
    token: &str,
) -> Result<Vec<ProxySource>, Box<dyn std::error::Error + Send + Sync>> {
    let query = "proxy list OR socks5 list OR http proxy";
    let url = format!(
        "https://api.github.com/search/repositories?q={}&sort=updated&per_page=20",
        encode(query)
    );

    #[derive(Deserialize)]
    struct GhRepo {
        full_name: String,
        html_url: String,
        updated_at: String,
    }

    #[derive(Deserialize)]
    struct GhResponse {
        items: Vec<GhRepo>,
    }

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "Omega9-NEXUS/15.0")
        .timeout(Duration::from_secs(15))
        .send()
        .await?;

    let data: GhResponse = response.json().await?;
    let mut sources = Vec::new();

    for repo in data.items {
        // Look for common proxy list files
        for file in ["proxies.txt", "proxy.txt", "http.txt", "socks5.txt", "list.txt"] {
            let raw_url = format!(
                "https://raw.githubusercontent.com/{}/main/{}",
                repo.full_name, file
            );
            sources.push(ProxySource {
                url: raw_url,
                name: format!("GitHub: {}", repo.full_name),
                source_type: "github".to_string(),
            });
        }
    }

    debug!("Found {} GitHub sources", sources.len());
    Ok(sources)
}

/// Search Reddit for proxy lists
async fn discover_reddit_sources(client: &Client) -> Result<Vec<ProxySource>, Box<dyn std::error::Error + Send + Sync>> {
    let subreddits = ["ProxyLists", "FreeProxies", "proxies"];
    let mut sources = Vec::new();

    for subreddit in subreddits {
        let url = format!("https://www.reddit.com/r/{}/new.json?limit=25", subreddit);

        #[derive(Deserialize)]
        struct RedditPost {
            data: RedditPostData,
        }

        #[derive(Deserialize)]
        struct RedditPostData {
            title: String,
            url: String,
            selftext: String,
        }

        #[derive(Deserialize)]
        struct RedditResponse {
            data: RedditResponseData,
        }

        #[derive(Deserialize)]
        struct RedditResponseData {
            children: Vec<RedditPost>,
        }

        let response = client
            .get(&url)
            .header("User-Agent", "Omega9-NEXUS/15.0")
            .timeout(Duration::from_secs(15))
            .send()
            .await?;

        let data: RedditResponse = response.json().await?;

        for post in data.data.children {
            // Extract URLs from post content
            let text = format!("{} {}", post.data.title, post.data.selftext);
            for word in text.split_whitespace() {
                if word.starts_with("http") && (word.contains("pastebin") || word.contains("raw.githubusercontent") || word.contains(".txt")) {
                    sources.push(ProxySource {
                        url: word.to_string(),
                        name: format!("Reddit: r/{}", subreddit),
                        source_type: "reddit".to_string(),
                    });
                }
            }
        }
    }

    debug!("Found {} Reddit sources", sources.len());
    Ok(sources)
}

/// Search Bing for public proxy lists
async fn discover_bing_sources(client: &Client) -> Result<Vec<ProxySource>, Box<dyn std::error::Error + Send + Sync>> {
    let query = "free proxy list http socks5";
    let url = format!(
        "https://www.bing.com/search?q={}",
        encode(query)
    );

    let response = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .timeout(Duration::from_secs(15))
        .send()
        .await?;

    let body = response.text().await?;
    let document = Html::parse_document(&body);
    
    let link_selector = Selector::parse("a").ok();
    let mut sources = Vec::new();

    if let Some(selector) = link_selector {
        for element in document.select(&selector) {
            if let Some(href) = element.value().attr("href") {
                if href.contains(".txt") || href.contains("proxy") {
                    // Clean up Bing redirect URLs
                    let clean_url = href.split('&').next().unwrap_or(href);
                    if clean_url.starts_with("http") {
                        sources.push(ProxySource {
                            url: clean_url.to_string(),
                            name: "Bing Search Result".to_string(),
                            source_type: "bing".to_string(),
                        });
                    }
                }
            }
        }
    }

    debug!("Found {} Bing sources", sources.len());
    Ok(sources)
}

/// Discover Tor .onion proxy sources
async fn discover_tor_sources(client: &Client) -> Result<Vec<ProxySource>, Box<dyn std::error::Error + Send + Sync>> {
    // Configure client to use Tor SOCKS proxy (typically localhost:9050)
    let tor_proxy = reqwest::Proxy::all("socks5://127.0.0.1:9050")?;
    let tor_client = Client::builder()
        .proxy(tor_proxy)
        .timeout(Duration::from_secs(30))
        .build()?;

    let onion_sites = vec![
        "http://proxylist.onion",
        "http://socks5list.onion",
        // Add known darknet proxy list sites
    ];

    let mut sources = Vec::new();

    for site in onion_sites {
        match tor_client.get(site).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    sources.push(ProxySource {
                        url: site.to_string(),
                        name: format!("Tor: {}", site),
                        source_type: "tor".to_string(),
                    });
                }
            }
            Err(e) => {
                debug!("Tor site {} unreachable: {}", site, e);
            }
        }
    }

    debug!("Found {} Tor sources", sources.len());
    Ok(sources)
}

/// Get static high-quality sources
pub fn get_static_sources() -> Vec<ProxySource> {
    vec![
        ProxySource {
            url: "https://api.proxyscrape.com/v2/?request=get&protocol=http&timeout=10000&country=all".to_string(),
            name: "ProxyScrape HTTP".to_string(),
            source_type: "static".to_string(),
        },
        ProxySource {
            url: "https://api.proxyscrape.com/v2/?request=get&protocol=socks5&timeout=10000&country=all".to_string(),
            name: "ProxyScrape SOCKS5".to_string(),
            source_type: "static".to_string(),
        },
        ProxySource {
            url: "https://raw.githubusercontent.com/TheSpeedX/PROXY-List/master/http.txt".to_string(),
            name: "TheSpeedX HTTP".to_string(),
            source_type: "static".to_string(),
        },
        ProxySource {
            url: "https://raw.githubusercontent.com/TheSpeedX/PROXY-List/master/socks5.txt".to_string(),
            name: "TheSpeedX SOCKS5".to_string(),
            source_type: "static".to_string(),
        },
        ProxySource {
            url: "https://raw.githubusercontent.com/monosans/proxy-list/main/proxies/http.txt".to_string(),
            name: "monosans HTTP".to_string(),
            source_type: "static".to_string(),
        },
        ProxySource {
            url: "https://raw.githubusercontent.com/monosans/proxy-list/main/proxies/socks5.txt".to_string(),
            name: "monosans SOCKS5".to_string(),
            source_type: "static".to_string(),
        },
        ProxySource {
            url: "https://www.proxy-list.download/api/v1/get?type=http".to_string(),
            name: "proxy-list.download HTTP".to_string(),
            source_type: "static".to_string(),
        },
        ProxySource {
            url: "https://www.proxy-list.download/api/v1/get?type=socks5".to_string(),
            name: "proxy-list.download SOCKS5".to_string(),
            source_type: "static".to_string(),
        },
        ProxySource {
            url: "https://proxylist.geonode.com/api/proxy-list?limit=500&page=1&sort_by=lastChecked&sort_type=desc".to_string(),
            name: "GeoNode".to_string(),
            source_type: "static".to_string(),
        },
        ProxySource {
            url: "https://raw.githubusercontent.com/proxifly/free-proxy-list/main/proxies/all/data.txt".to_string(),
            name: "Proxifly All".to_string(),
            source_type: "static".to_string(),
        },
        ProxySource {
            url: "https://raw.githubusercontent.com/ALIILAPRO/Proxy/main/http.txt".to_string(),
            name: "ALIILAPRO HTTP".to_string(),
            source_type: "static".to_string(),
        },
        ProxySource {
            url: "https://api.openproxylist.xyz/http.txt".to_string(),
            name: "OpenProxyList HTTP".to_string(),
            source_type: "static".to_string(),
        },
        ProxySource {
            url: "https://api.openproxylist.xyz/socks5.txt".to_string(),
            name: "OpenProxyList SOCKS5".to_string(),
            source_type: "static".to_string(),
        },
        ProxySource {
            url: "http://www.gatherproxy.com/".to_string(),
            name: "GatherProxy (HTTP)".to_string(),
            source_type: "static".to_string(),
        },
    ]
}
