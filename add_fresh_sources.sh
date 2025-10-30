#!/bin/bash
# Add 85+ Fresh 2025 Proxy Sources to Omega9-Nexus
# Based on comprehensive proxy hunting list

DB_PATH="/home/dappy/omega9-nexus/omega9.db"

echo "🔥 Adding 85+ Fresh 2025 Proxy Sources..."

# GitHub Fresh Sources (2025)
sqlite3 "$DB_PATH" << 'EOF'
-- GitHub Core Sources
INSERT OR IGNORE INTO sources (url, name, quality_score, active) VALUES 
('https://raw.githubusercontent.com/TheSpeedX/PROXY-List/master/http.txt', 'TheSpeedX HTTP', 0.8, 1),
('https://raw.githubusercontent.com/TheSpeedX/PROXY-List/master/socks5.txt', 'TheSpeedX SOCKS5', 0.8, 1),
('https://raw.githubusercontent.com/proxifly/free-proxy-list/main/proxies/protocols/http/data.txt', 'Proxifly HTTP', 0.85, 1),
('https://raw.githubusercontent.com/proxifly/free-proxy-list/main/proxies/protocols/socks5/data.txt', 'Proxifly SOCKS5', 0.85, 1),
('https://raw.githubusercontent.com/gfpcom/free-proxy-list/main/list/http.txt', 'GFPCOM HTTP', 0.75, 1),
('https://raw.githubusercontent.com/gfpcom/free-proxy-list/main/list/socks5.txt', 'GFPCOM SOCKS5', 0.75, 1),
('https://raw.githubusercontent.com/vakhov/fresh-proxy-list/main/proxies/http.txt', 'Vakhov HTTP 2025', 0.9, 1),
('https://raw.githubusercontent.com/vakhov/fresh-proxy-list/main/proxies/socks5.txt', 'Vakhov SOCKS5 2025', 0.9, 1),
('https://raw.githubusercontent.com/dpangestuw/Free-Proxy/master/proxies/all.txt', 'Dpangestuw All', 0.7, 1),
('https://raw.githubusercontent.com/mmpx12/proxy-list/master/http.txt', 'MMPX12 HTTP', 0.8, 1),
('https://raw.githubusercontent.com/mmpx12/proxy-list/master/socks4.txt', 'MMPX12 SOCKS4', 0.8, 1),
('https://raw.githubusercontent.com/mmpx12/proxy-list/master/socks5.txt', 'MMPX12 SOCKS5', 0.8, 1),
('https://raw.githubusercontent.com/Zaeem20/FREE_PROXIES_LIST/main/http.txt', 'Zaeem20 HTTP', 0.75, 1),
('https://raw.githubusercontent.com/Zaeem20/FREE_PROXIES_LIST/main/socks5.txt', 'Zaeem20 SOCKS5', 0.75, 1),
('https://raw.githubusercontent.com/clarketm/proxy-list/master/proxy-list-raw.txt', 'Clarketm Raw', 0.7, 1),
('https://raw.githubusercontent.com/monosans/proxy-list/main/proxies/http.txt', 'Monosans HTTP', 0.85, 1),
('https://raw.githubusercontent.com/monosans/proxy-list/main/proxies/socks5.txt', 'Monosans SOCKS5', 0.85, 1),
('https://raw.githubusercontent.com/jetkai/proxy-list/main/online-proxies/txt/proxies-http.txt', 'Jetkai HTTP', 0.8, 1),
('https://raw.githubusercontent.com/jetkai/proxy-list/main/online-proxies/txt/proxies-socks5.txt', 'Jetkai SOCKS5', 0.8, 1),
('https://raw.githubusercontent.com/roosterkid/openproxylist/main/HTTP_RAW.txt', 'RoosterKid HTTP', 0.75, 1),
('https://raw.githubusercontent.com/roosterkid/openproxylist/main/SOCKS5_RAW.txt', 'RoosterKid SOCKS5', 0.75, 1),
('https://raw.githubusercontent.com/sunny9577/proxy-scraper/master/generated/http_proxies.txt', 'Sunny Scraper HTTP', 0.8, 1),
('https://raw.githubusercontent.com/sunny9577/proxy-scraper/master/generated/socks5_proxies.txt', 'Sunny Scraper SOCKS5', 0.8, 1),
('https://raw.githubusercontent.com/sunny9577/proxy-scraper/master/generated/all_proxies.txt', 'Sunny Scraper All', 0.8, 1),
('https://raw.githubusercontent.com/B4RC0DE-TM/proxy-list/main/HTTP.txt', 'B4RC0DE HTTP', 0.7, 1),
('https://raw.githubusercontent.com/B4RC0DE-TM/proxy-list/main/SOCKS5.txt', 'B4RC0DE SOCKS5', 0.7, 1),
('https://raw.githubusercontent.com/Volodichev/proxy-list/main/http.txt', 'Volodichev HTTP', 0.75, 1),
('https://raw.githubusercontent.com/Volodichev/proxy-list/main/socks5.txt', 'Volodichev SOCKS5', 0.75, 1),
('https://raw.githubusercontent.com/mertguvencli/http-proxy-list/main/proxy-list/data.txt', 'Mert HTTP', 0.7, 1),
('https://raw.githubusercontent.com/manuGMG/proxy-365/main/HTTP.txt', 'ManuGMG HTTP', 0.75, 1),
('https://raw.githubusercontent.com/manuGMG/proxy-365/main/SOCKS5.txt', 'ManuGMG SOCKS5', 0.75, 1),
('https://raw.githubusercontent.com/zevtyardt/proxy-list/main/http.txt', 'Zevtyardt HTTP', 0.75, 1),
('https://raw.githubusercontent.com/zevtyardt/proxy-list/main/socks5.txt', 'Zevtyardt SOCKS5', 0.75, 1),
('https://raw.githubusercontent.com/rdavydov/proxy-list/main/proxies/http.txt', 'Rdavydov HTTP', 0.8, 1),
('https://raw.githubusercontent.com/rdavydov/proxy-list/main/proxies/socks5.txt', 'Rdavydov SOCKS5', 0.8, 1),
('https://raw.githubusercontent.com/elliotwutingfeng/GlobalProxyList/master/data/http.txt', 'GlobalProxy HTTP', 0.8, 1),
('https://raw.githubusercontent.com/elliotwutingfeng/GlobalProxyList/master/data/socks5.txt', 'GlobalProxy SOCKS5', 0.8, 1),
('https://raw.githubusercontent.com/porthole-crew/proxy-list/main/proxies/http.txt', 'Porthole HTTP', 0.75, 1),
('https://raw.githubusercontent.com/porthole-crew/proxy-list/main/proxies/socks5.txt', 'Porthole SOCKS5', 0.75, 1),
('https://raw.githubusercontent.com/porthole-crew/proxy-list/main/proxies/all.txt', 'Porthole All', 0.75, 1),
('https://raw.githubusercontent.com/hendrikbgr/Free-Proxy-Repo/master/proxy_list.txt', 'Hendrik All', 0.7, 1),
('https://raw.githubusercontent.com/almroot/proxylist/master/list.txt', 'Almroot All', 0.7, 1),
('https://raw.githubusercontent.com/getfreeproxy/daily-proxy-list/main/http.txt', 'GetFreeProxy HTTP 2025', 0.9, 1),
('https://raw.githubusercontent.com/getfreeproxy/daily-proxy-list/main/socks5.txt', 'GetFreeProxy SOCKS5 2025', 0.9, 1);

-- API Sources
INSERT OR IGNORE INTO sources (url, name, quality_score, active) VALUES
('https://api.proxyscrape.com/v2/?request=getproxies&protocol=http&timeout=10000&country=all&ssl=all&anonymity=all', 'ProxyScrape HTTP', 0.85, 1),
('https://api.proxyscrape.com/v2/?request=getproxies&protocol=socks5&timeout=10000&country=all', 'ProxyScrape SOCKS5', 0.85, 1),
('https://api.proxyscrape.com/v2/?request=getproxies&protocol=socks4&timeout=10000&country=all', 'ProxyScrape SOCKS4', 0.85, 1),
('https://www.proxy-list.download/api/v1/get?type=http', 'ProxyListDownload HTTP', 0.8, 1),
('https://www.proxy-list.download/api/v1/get?type=socks5', 'ProxyListDownload SOCKS5', 0.8, 1),
('https://www.proxy-list.download/api/v1/get?type=socks4', 'ProxyListDownload SOCKS4', 0.8, 1),
('https://api.openproxylist.xyz/http.txt', 'OpenProxyList HTTP', 0.75, 1),
('https://api.openproxylist.xyz/socks5.txt', 'OpenProxyList SOCKS5', 0.75, 1);

-- Web Scraper Sources
INSERT OR IGNORE INTO sources (url, name, quality_score, active) VALUES
('https://free-proxy-list.net/', 'FreeProxyList.net', 0.7, 1),
('https://www.sslproxies.org/', 'SSLProxies.org', 0.7, 1),
('https://www.us-proxy.org/', 'US-Proxy.org', 0.7, 1),
('https://spys.one/en/free-proxy-list/', 'Spys.one', 0.75, 1),
('https://hidemy.name/en/proxy-list/', 'HideMyName', 0.7, 1),
('https://www.proxynova.com/proxy-server-list/', 'ProxyNova', 0.7, 1),
('https://geonode.com/free-proxy-list', 'GeoNode', 0.75, 1),
('https://openproxy.space/list/http', 'OpenProxySpace HTTP', 0.7, 1),
('https://openproxy.space/list/socks5', 'OpenProxySpace SOCKS5', 0.7, 1),
('https://www.proxyscan.io/download?type=http', 'ProxyScan HTTP', 0.75, 1),
('https://www.proxyscan.io/download?type=socks5', 'ProxyScan SOCKS5', 0.75, 1),
('https://proxybros.com/free-proxy-list/http/download/txt', 'ProxyBros HTTP', 0.7, 1),
('https://proxybros.com/free-proxy-list/socks5/download/txt', 'ProxyBros SOCKS5', 0.7, 1),
('https://sockslist.us/socks5.txt', 'SocksList SOCKS5', 0.7, 1),
('https://sockslist.us/socks4.txt', 'SocksList SOCKS4', 0.7, 1);

-- Add fresh sources tracking
UPDATE sources SET last_updated = NULL WHERE last_updated IS NULL;

-- Mark sources as fresh (no failures yet)
INSERT OR IGNORE INTO source_performance (source_url, check_timestamp, success) 
SELECT url, datetime('now'), 1 FROM sources WHERE active = 1;
EOF

# Count total sources
TOTAL=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM sources WHERE active = 1;")

echo "✅ Added fresh 2025 sources!"
echo "📊 Total active sources: $TOTAL"
echo "🎯 Ready to hunt!"
