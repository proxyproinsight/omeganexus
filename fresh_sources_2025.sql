-- Fresh 2025 Proxy Sources
INSERT INTO sources (url, name, quality_score, total_proxies, working_proxies, active) VALUES
-- Zaeem20 repos (2025 fresh)
('https://raw.githubusercontent.com/Zaeem20/FREE_PROXIES_LIST/master/http.txt', 'Zaeem20 HTTP', 0.7, 0, 0, 1),
('https://raw.githubusercontent.com/Zaeem20/FREE_PROXIES_LIST/master/https.txt', 'Zaeem20 HTTPS', 0.7, 0, 0, 1),
('https://raw.githubusercontent.com/Zaeem20/FREE_PROXIES_LIST/master/socks4.txt', 'Zaeem20 SOCKS4', 0.6, 0, 0, 1),
('https://raw.githubusercontent.com/Zaeem20/FREE_PROXIES_LIST/master/socks5.txt', 'Zaeem20 SOCKS5', 0.7, 0, 0, 1),

-- Proxifly (2025 active)
('https://raw.githubusercontent.com/proxifly/free-proxy-list/main/proxies/protocols/http/data.txt', 'Proxifly HTTP', 0.75, 0, 0, 1),
('https://raw.githubusercontent.com/proxifly/free-proxy-list/main/proxies/protocols/socks4/data.txt', 'Proxifly SOCKS4', 0.7, 0, 0, 1),
('https://raw.githubusercontent.com/proxifly/free-proxy-list/main/proxies/protocols/socks5/data.txt', 'Proxifly SOCKS5', 0.75, 0, 0, 1),

-- Skillter/ProxyGather (2025)
('https://raw.githubusercontent.com/Skillter/proxy-list/main/http.txt', 'Skillter HTTP', 0.7, 0, 0, 1),
('https://raw.githubusercontent.com/Skillter/proxy-list/main/socks4.txt', 'Skillter SOCKS4', 0.65, 0, 0, 1),

-- Additional 2025 sources
('https://raw.githubusercontent.com/roosterkid/openproxylist/main/HTTPS_RAW.txt', 'RoosterKid HTTPS', 0.7, 0, 0, 1),
('https://raw.githubusercontent.com/sunny9577/proxy-scraper/master/proxies.txt', 'Sunny Scraper', 0.65, 0, 0, 1),
('https://raw.githubusercontent.com/officialputuid/KangProxy/main/http/http.txt', 'KangProxy HTTP', 0.7, 0, 0, 1),
('https://raw.githubusercontent.com/officialputuid/KangProxy/main/socks5/socks5.txt', 'KangProxy SOCKS5', 0.7, 0, 0, 1),

-- API-based sources
('https://api.proxyscrape.com/v3/free-proxy-list/get?request=displayproxies&protocol=http&timeout=10000&country=all&ssl=all&anonymity=all&proxy_format=ipport&format=text', 'ProxyScrape API HTTP', 0.75, 0, 0, 1),
('https://api.proxyscrape.com/v3/free-proxy-list/get?request=displayproxies&protocol=socks4&timeout=10000&country=all&proxy_format=ipport&format=text', 'ProxyScrape API SOCKS4', 0.7, 0, 0, 1),
('https://api.proxyscrape.com/v3/free-proxy-list/get?request=displayproxies&protocol=socks5&timeout=10000&country=all&proxy_format=ipport&format=text', 'ProxyScrape API SOCKS5', 0.75, 0, 0, 1)

ON CONFLICT(url) DO NOTHING;
