# Omega9-NEXUS v15.0 🔥

**Elite Proxy Hunter with AI-Driven Scoring & Dynamic Source Discovery**

A cutting-edge proxy hunting system built in Rust featuring:
- 🤖 AI-powered quality scoring with EMA learning
- 🔍 Dynamic source discovery (GitHub, Reddit, Bing, Tor)
- 🛡️ Advanced validation (DNS leak, fraud detection, elite anonymity)
- 📱 Telegram bot integration
- 🌐 Real-time web dashboard
- 🐋 Docker containerization
- 🔐 Security hardened for Pop!_OS

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Omega9-NEXUS v15.0                       │
├─────────────────────────────────────────────────────────────┤
│  Rust Backend (Axum + Tokio)                                 │
│  ├─ Hunt Loop: Fetch & validate proxies every 5min           │
│  ├─ Discovery Loop: Find new sources hourly                  │
│  ├─ AI Scoring: Quality prediction with fraud penalties      │
│  ├─ Validation: Latency, geo, fraud, DNS leak, elite check   │
│  └─ API: REST + WebSocket for real-time updates              │
├─────────────────────────────────────────────────────────────┤
│  Telegram Bot (Teloxide)                                     │
│  ├─ /stats - Current metrics                                 │
│  ├─ /top - Best quality proxies                              │
│  ├─ /fastest - Lowest latency proxies                        │
│  ├─ /sources - Active source list                            │
│  └─ /deactivate <id> - Disable poor sources                  │
├─────────────────────────────────────────────────────────────┤
│  Web Dashboard (Tailwind CSS)                                │
│  ├─ Real-time stats via WebSocket                            │
│  ├─ Proxy table with filtering                               │
│  └─ Source quality monitoring                                │
├─────────────────────────────────────────────────────────────┤
│  Storage (SQLite + WAL)                                       │
│  ├─ Proxies: host, port, protocol, quality_score, etc.       │
│  ├─ Sources: url, name, quality_score, success_rate          │
│  └─ Metrics: historical stats for analysis                   │
└─────────────────────────────────────────────────────────────┘
```

---

## Features

### 🎯 Core Capabilities
- **Multi-Protocol Support**: HTTP, HTTPS, SOCKS5
- **Concurrent Validation**: 50+ workers for fast proxy testing
- **Geolocation**: Country/city detection via ip-api.com
- **Fraud Detection**: Risk scoring via scamalytics.com
- **DNS Leak Testing**: Privacy check via bash.ws
- **Elite Anonymity**: Header analysis via httpbin.org

### 🧠 AI Scoring System
Predictive quality scoring using weighted components:
- **Latency** (40%): Lower is better
- **Source Quality** (25%): EMA-based source reputation
- **Uptime** (20%): Freshness/age factor
- **Country Diversity** (15%): Bonus for rare countries
- **Fraud Penalty** (50% reduction): High-risk IP detection
- **DNS Leak Penalty** (30% reduction): Privacy compromise
- **Elite Bonus** (15% boost): True anonymity

### 🔍 Dynamic Source Discovery
Automated hunting across:
1. **GitHub API**: Recent proxy list repositories
2. **Reddit**: r/ProxyLists, r/FreeProxies, r/proxies
3. **Bing Search**: Web scraping for public lists
4. **Tor Darknet**: .onion proxy sources (if Tor available)
5. **Static Sources**: 14+ high-quality curated URLs

### 🔐 Security Hardening
- UFW firewall with restrictive rules
- Non-root container execution
- Capability dropping (no-new-privileges)
- Tailscale for secure remote access
- Tor integration for anonymity

---

## Quick Start

### Prerequisites
- **Pop!_OS 22.04+** (or Ubuntu-based distro)
- **Docker** & **docker-compose**
- **Telegram Bot Token** (from @BotFather)

### 1. Environment Setup
```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install dependencies
sudo apt install -y curl wget git tor nmap arp-scan docker.io docker-compose

# Setup Tailscale (optional)
curl -fsSL https://tailscale.com/install.sh | sh
sudo tailscale up

# Configure firewall
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow 8080/tcp comment 'Omega9-NEXUS Dashboard'
sudo ufw allow 22/tcp comment 'SSH'
sudo ufw enable
```

### 2. Deploy Application
```bash
# Clone/navigate to project
cd /home/dappy/omega9-nexus

# Configure environment
cp .env.example .env
nano .env  # Set TELOXIDE_TOKEN and other vars

# Make deploy script executable
chmod +x deploy.sh

# Run deployment
./deploy.sh
```

### 3. Verify Installation
```bash
# Check containers
docker ps

# View logs
docker logs -f omega9-nexus

# Test dashboard
curl http://localhost:8080

# Test Telegram bot
# Send /start to your bot
```

---

## Configuration

### Environment Variables
```bash
# Required
TELOXIDE_TOKEN=7810974163:AAEcbS9EyM8UU-gwVX8L_QynVK7QZL62Cmk

# Optional
GITHUB_TOKEN=ghp_xxxxx              # For GitHub API discovery
REDDIT_CLIENT_ID=xxxxx              # For Reddit API
REDDIT_CLIENT_SECRET=xxxxx          # For Reddit API
DATABASE_URL=sqlite:omega9.db       # SQLite path
BIND_ADDRESS=0.0.0.0:8080          # Server address
HUNT_INTERVAL_SECS=300             # Hunt frequency (5min default)
VALIDATION_WORKERS=50               # Concurrent validators
MAX_LATENCY_MS=3000                # Reject slower proxies
MIN_QUALITY_SCORE=0.3              # Reject low-quality proxies
```

### Telegram Bot Setup
1. Message @BotFather on Telegram
2. Create new bot: `/newbot`
3. Copy token to `.env`
4. Commands will auto-register on first run

---

## API Endpoints

### REST API
- `GET /` - Web dashboard
- `GET /api/proxies` - List top 100 proxies
- `GET /api/stats` - Current statistics
- `GET /api/sources` - Active sources
- `POST /api/hunt` - Trigger manual hunt
- `GET /ws` - WebSocket for real-time stats

### Telegram Commands
- `/start` - Bot introduction
- `/stats` - Current metrics
- `/top` - Top 10 quality proxies
- `/fastest` - Top 10 fastest proxies
- `/hunt` - Trigger manual hunt cycle
- `/sources` - List active sources
- `/deactivate <id>` - Disable source by ID

---

## Database Schema

### Proxies Table
```sql
- id: INTEGER PRIMARY KEY
- host: TEXT (IP address)
- port: INTEGER
- protocol: TEXT (http/socks5)
- country: TEXT
- city: TEXT
- latency_ms: INTEGER
- quality_score: REAL (0.0-1.0)
- fraud_score: REAL (0.0-1.0)
- dns_leak: INTEGER (0/1)
- elite: INTEGER (0/1)
- last_checked: INTEGER (timestamp)
- discovered_at: INTEGER (timestamp)
- source: TEXT (source name)
- active: INTEGER (0/1)
```

### Sources Table
```sql
- id: INTEGER PRIMARY KEY
- url: TEXT UNIQUE
- name: TEXT
- quality_score: REAL (0.0-1.0, EMA updated)
- total_proxies: INTEGER
- working_proxies: INTEGER
- last_updated: INTEGER (timestamp)
- active: INTEGER (0/1)
```

---

## Development

### Build from Source
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build
cargo build --release

# Run locally
export DATABASE_URL=sqlite:omega9.db
export TELOXIDE_TOKEN=your_token
./target/release/omega9-nexus
```

### Run Tests
```bash
cargo test
```

### Add Custom Source
```sql
INSERT INTO sources (url, name, quality_score, active)
VALUES ('https://example.com/proxies.txt', 'Custom Source', 0.7, 1);
```

---

## Monitoring

### Logs
```bash
# Application logs
docker logs -f omega9-nexus

# Scanner logs
docker logs -f omega9-scanner
```

### Metrics
Access Prometheus-compatible metrics at `/metrics` (requires additional setup)

### Database Inspection
```bash
docker exec -it omega9-nexus sqlite3 /app/data/omega9.db
sqlite> SELECT COUNT(*) FROM proxies WHERE active = 1;
sqlite> SELECT name, quality_score FROM sources ORDER BY quality_score DESC LIMIT 10;
```

---

## Troubleshooting

### No Proxies Found
- Check hunt interval: `docker logs omega9-nexus | grep "hunt cycle"`
- Verify sources: `SELECT * FROM sources WHERE active = 1;`
- Test manually: `curl https://api.proxyscrape.com/v2/?request=get&protocol=http`

### Telegram Bot Not Responding
- Verify token: `echo $TELOXIDE_TOKEN`
- Check bot logs: `docker logs omega9-nexus | grep teloxide`
- Ensure bot is started: Send `/start` command

### High Memory Usage
- Reduce `VALIDATION_WORKERS` in `.env`
- Increase `HUNT_INTERVAL_SECS` to reduce frequency
- Clean old proxies: `DELETE FROM proxies WHERE last_checked < strftime('%s', 'now') - 86400;`

---

## Security Notes

⚠️ **Important:**
- Never expose port 8080 to public internet without authentication
- Use Tailscale or VPN for remote access
- Rotate Telegram bot token if compromised
- Keep system packages updated
- Review firewall rules regularly

---

## Roadmap

- [ ] Machine learning regression with ndarray
- [ ] IPv6 proxy support
- [ ] Proxy rotation API
- [ ] Grafana dashboard integration
- [ ] Export formats (JSON, CSV, TXT)
- [ ] Proxy chain support
- [ ] Auto-ban unreliable sources
- [ ] Rate limiting per source

---

## License

MIT License - See LICENSE file

---

## Credits

Built with ❤️ using:
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [Teloxide](https://github.com/teloxide/teloxide) - Telegram bot
- [SQLx](https://github.com/launchbadge/sqlx) - Async SQL
- [Reqwest](https://github.com/seanmonstar/reqwest) - HTTP client
- [Scraper](https://github.com/causal-agent/scraper) - HTML parsing

---

**Omega9-NEXUS v15.0** - Because mediocre proxies are not an option. 🔥
