# 🔍 Omega9-NEXUS Dashboard - Search & Filter Guide

## Overview
The dashboard now includes comprehensive search and filtering capabilities to help you find the perfect proxies for your needs.

## Filter Options

### 1. **Search by Country**
- Type any country name (case-insensitive)
- Examples: "canada", "russia", "france"
- Searches the geo-location database

### 2. **Protocol Filter**
- **All** - Show all protocols
- **HTTP** - HTTP proxies only (50 available)
- **HTTPS** - HTTPS proxies only
- **SOCKS4** - SOCKS4 proxies only
- **SOCKS5** - SOCKS5 proxies only

### 3. **Anonymity Level**
- **All** - No anonymity filtering
- **Transparent** - Non-anonymous proxies
- **Anonymous** - Medium anonymity
- **Elite** - Highest anonymity (24 available)

### 4. **Max Latency Slider**
- Range: 100ms - 5000ms
- Default: 5000ms
- Find fastest proxies by lowering this value
- **<1000ms**: Only 1 ultra-fast proxy available

### 5. **Min Quality Slider**
- Range: 0.0 - 1.0
- Default: 0.0 (no filter)
- Quality based on 7-layer validation
- **>0.7**: Only 1 premium proxy available

### 6. **Sort Options**
- **Quality (High to Low)** - Default, best proxies first
- **Fastest (Low Latency)** - Speed-optimized sorting
- **Country (A-Z)** - Alphabetical by location

## Quick Filter Buttons

### 👑 Elite Only
Instantly filters to show only elite anonymity proxies with quality >0.6
- Current count: **24 proxies**

### ⚡ Fastest (<1s)
Shows proxies with latency under 1000ms, sorted by speed
- Current count: **1 proxy**

### ⭐ High Quality (>0.7)
Displays only premium proxies with quality score above 0.7
- Current count: **1 proxy**

### 🔄 Reset Filters
Clears all filters and returns to default view showing all proxies

## Real-Time Stats

The dashboard displays:
- **Total Proxies**: 185 active proxies
- **Working**: 185 validated proxies
- **Avg Latency**: 5886ms
- **Avg Quality**: 0.40
- **Active Sources**: 32 proxy sources

## Results Display

Shows filtered proxy count in real-time:
```
Showing X of Y proxies
```

## Usage Examples

### Example 1: Find Fast Elite Proxies
1. Click **👑 Elite Only** button
2. Adjust **Max Latency** slider to 2000ms
3. Result: Elite proxies under 2 seconds latency

### Example 2: Country-Specific Search
1. Type "United States" in Search box
2. Select **HTTPS** protocol
3. Set **Min Quality** to 0.5
4. Result: US-based HTTPS proxies with decent quality

### Example 3: Speed Testing
1. Click **⚡ Fastest (<1s)** button
2. Verify the ultra-fast proxy details
3. Copy address for immediate use

## API Endpoints Used

The dashboard leverages these v18.0 API endpoints:

- `GET /api/proxies` - Fetch all proxies
- `GET /api/proxies/filter` - Advanced filtering
- `GET /api/sources/health` - Source monitoring
- `WebSocket /ws` - Real-time stats updates

## Browser Requirements

- Modern browser with JavaScript enabled
- WebSocket support for live updates
- Recommended: Chrome, Firefox, Edge

## Tips

1. **Combine Filters**: Use multiple filters together for precise results
2. **Check Real-Time**: Stats update automatically via WebSocket
3. **Sort Strategically**: Sort by quality for reliability, latency for speed
4. **Quick Actions**: Use preset buttons for common filter scenarios
5. **Reset Often**: Hit reset between different search queries

## Current Proxy Pool Stats

- **Total Active**: 185 proxies
- **Elite Anonymous**: 24 proxies
- **Ultra-Fast (<1s)**: 1 proxy
- **High Quality (>0.7)**: 1 proxy
- **HTTP Protocol**: 50 proxies

---

**Note**: Filters are applied in real-time using the backend API. All searches are case-insensitive and results update instantly.
