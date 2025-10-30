# Omega9-NEXUS v15.0 - Aggressive Hunting Improvements

## What We Improved

### 1. **Visual Indicators Added** ✨
- **Blue Pulsing Dot**: Shows system is online (top right corner)
- **Lightning Bolt**: Pulses yellow to indicate active hunting
- Both animate continuously so you know the system is alive

### 2. **More Aggressive Hunting** 🚀

#### Before:
- 4 proxy sources (mostly HTTP)
- 100 proxies validated per source
- 10 concurrent validations
- Hunt every 5 minutes

#### After:
- **16 proxy sources** (HTTP, SOCKS4, SOCKS5, HTTPS)
- **500 proxies validated per source** (5x more)
- **20 concurrent validations** (2x faster)
- **Hunt every 2 minutes** (2.5x more frequent)

### 3. **New Proxy Sources Added**
```
HTTP Sources:
- TheSpeedX, monosans, ProxyScrape, MMPX12, ClarkeTM, ShiftyTR, JetKai

SOCKS5 Sources:
- TheSpeedX SOCKS5, Hookzof, MMPX12, ShiftyTR, JetKai

SOCKS4 Sources:
- proxy-list.download, OpenProxyList, JetKai

HTTPS Sources:
- JetKai HTTPS
```

### 4. **Proxy Testing Endpoint** 🧪
New API endpoint to test if a proxy actually works:

```bash
# Test a specific proxy
curl -X POST http://localhost:8081/api/test-proxy \
  -H "Content-Type: application/json" \
  -d '{"host":"134.209.29.120","port":8080,"protocol":"http"}'
```

Response shows:
- ✅ If proxy is working
- ⚡ Latency in milliseconds
- 🌍 Geographic location
- 🔒 Elite anonymity status
- 🛡️ DNS leak status
- ⚠️ Fraud score

## Understanding Proxy Quality

### Current Stats (from your database):
- **17 working HTTP proxies**
- Average latency: **~5 seconds** (slower proxies, but they work)
- Average quality score: **0.47** (moderate quality)
- **1 elite proxy** (anonymous, no headers leaked)

### Can devices connect?
**YES!** Example working proxy:
```
134.209.29.120:8080 (UK) - 1.4s latency
```

To test manually:
```bash
# HTTP proxy test
curl -x http://134.209.29.120:8080 https://api.ipify.org
```

### Why Only HTTP So Far?
1. **SOCKS sources just added** - Need time to hunt
2. **Many SOCKS proxies are dead** - Public SOCKS lists have low success rates
3. **SOCKS validation is slower** - Requires different connection method

## What About Port Scanning?

### Current Behavior:
We're **NOT** scanning ports - we're fetching from public lists of known proxies.

### To Add Aggressive Port Scanning:
The code already has `fetch_local()` function in `crawler.rs` that can:
1. Scan local network with `arp-scan`
2. Test common proxy ports: 3128, 8080, 1080, 8888, 9050
3. Find hidden proxies on your network

**Note**: Port scanning external IPs is:
- ⚠️ Slow (would take hours for meaningful results)
- ⚠️ May trigger ISP alerts
- ⚠️ Often blocked by firewalls
- ⚠️ Better suited for local network discovery

## Performance Metrics

### Hunt Speed:
- Validates 500 proxies per source × 16 sources = **8,000 proxies per cycle**
- At 20 concurrent validations = ~**400 proxies/minute**
- Full cycle completes in **~20 minutes**
- New cycle starts every **2 minutes** (overlapping hunts)

### Expected Results:
With 16 sources and aggressive settings, you should see:
- **50-200 working proxies** within first hour
- Mix of HTTP, SOCKS4, SOCKS5
- Quality scores ranging 0.3-0.8
- Some elite proxies (no header leakage)

## API Endpoints Summary

```bash
# View stats
curl http://localhost:8081/api/stats | jq .

# List working proxies
curl http://localhost:8081/api/proxies | jq '.[0:5]'

# List all sources
curl http://localhost:8081/api/sources | jq .

# Trigger manual hunt
curl -X POST http://localhost:8081/api/hunt

# Test specific proxy
curl -X POST http://localhost:8081/api/test-proxy \
  -H "Content-Type: application/json" \
  -d '{"host":"IP","port":PORT,"protocol":"http"}'
```

## Dashboard Features

Visit: **http://localhost:8081**

Features:
- 📊 Real-time stats (total, working, latency, quality)
- ⚡ Live hunt indicator
- 💙 System online pulse
- 📋 Proxy list with quality scores
- 🌍 Country flags
- 🔒 Elite/leak indicators
- 🔄 Auto-refresh every 2 seconds via WebSocket

## Proxy Usage Example

### Using HTTP Proxy:
```bash
# Basic curl test
curl -x http://134.209.29.120:8080 https://api.ipify.org

# Python requests
import requests
proxies = {
    'http': 'http://134.209.29.120:8080',
    'https': 'http://134.209.29.120:8080'
}
response = requests.get('https://api.ipify.org', proxies=proxies)
print(response.text)
```

### Using SOCKS5 Proxy:
```bash
# curl with SOCKS5
curl --socks5 IP:PORT https://api.ipify.org

# Python with SOCKS5
pip install pysocks
proxies = {
    'http': 'socks5://IP:PORT',
    'https': 'socks5://IP:PORT'
}
```

## Telegram Bot Usage

The bot responds to:
- `/stats` - See current proxy counts
- `/top` - Best quality proxies
- `/fastest` - Lowest latency proxies
- `/sources` - All 16 active sources
- `/hunt` - Force immediate hunt

## Monitoring

Watch live hunting:
```bash
# Follow logs
sudo journalctl -u omega9-nexus.service -f

# Check database
sqlite3 ~/omega9-nexus/omega9.db "SELECT protocol, COUNT(*) FROM proxies WHERE active=1 GROUP BY protocol;"
```

## Next Hunt Cycle

The system is now hunting **every 2 minutes** with:
- 16 sources
- 500 proxies per source
- 20 concurrent validations

Expect to see working proxies accumulate quickly! 🚀
