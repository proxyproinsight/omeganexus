#!/bin/bash
# Omega9-NEXUS Proxy Tester
# Tests working proxies from the database

echo "🔥 Omega9-NEXUS Proxy Tester"
echo "================================"
echo ""

# Get your real IP
echo "🌐 Your Real IP:"
curl -s https://api.ipify.org?format=json | jq -r '.ip'
echo ""

# Get top 5 proxies from database
echo "🎯 Testing Top 5 Proxies:"
echo "================================"

sqlite3 /home/dappy/omega9-nexus/omega9.db "SELECT protocol, host, port, country, latency_ms, quality_score FROM proxies WHERE active=1 ORDER BY quality_score DESC LIMIT 5;" | while IFS='|' read -r protocol host port country latency quality; do
    echo ""
    echo "Testing: $protocol://$host:$port [$country] - ${latency}ms - Q:$quality"
    
    if [ "$protocol" = "http" ]; then
        result=$(timeout 5 curl -x http://$host:$port -s https://api.ipify.org?format=json 2>&1)
        if [ $? -eq 0 ]; then
            proxy_ip=$(echo $result | jq -r '.ip' 2>/dev/null)
            if [ -n "$proxy_ip" ] && [ "$proxy_ip" != "null" ]; then
                echo "  ✅ SUCCESS! Proxy IP: $proxy_ip"
            else
                echo "  ❌ Failed - Invalid response"
            fi
        else
            echo "  ❌ Failed - Timeout or connection error"
        fi
    elif [ "$protocol" = "socks5" ]; then
        result=$(timeout 5 curl --socks5 $host:$port -s https://api.ipify.org?format=json 2>&1)
        if [ $? -eq 0 ]; then
            proxy_ip=$(echo $result | jq -r '.ip' 2>/dev/null)
            if [ -n "$proxy_ip" ] && [ "$proxy_ip" != "null" ]; then
                echo "  ✅ SUCCESS! Proxy IP: $proxy_ip"
            else
                echo "  ❌ Failed - Invalid response"
            fi
        else
            echo "  ❌ Failed - Timeout or connection error"
        fi
    fi
done

echo ""
echo "================================"
echo "📊 Current Stats:"
curl -s http://localhost:8081/api/stats | jq .
