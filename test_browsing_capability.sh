#!/bin/bash
# Comprehensive test of premium proxy browsing capability

echo "🌐 Premium Proxy Browsing Capability Test"
echo "=========================================="
echo ""

working=0
browsable=0
total=0

# Test top 15 premium proxies
sqlite3 omega9.db "SELECT protocol, host, port, proxy_type, country, ROUND(quality_score, 3) FROM proxies WHERE proxy_type IN ('mobile', 'residential') AND active = 1 ORDER BY quality_score DESC LIMIT 15;" | while IFS='|' read -r protocol host port type country score; do
    
    total=$((total + 1))
    proxy_url="${protocol}://${host}:${port}"
    
    printf "%-50s %s %s (Q:%.3f)\n" "$proxy_url" "$type" "$country" "$score"
    
    if [ "$protocol" = "http" ] || [ "$protocol" = "https" ]; then
        # Test connectivity
        if curl -x "$proxy_url" --max-time 8 -s https://api.ipify.org > /dev/null 2>&1; then
            working=$((working + 1))
            printf "  ✅ Connected "
            
            # Test Google
            google_code=$(curl -x "$proxy_url" --max-time 8 -s -o /dev/null -w "%{http_code}" https://www.google.com 2>&1)
            
            # Test Amazon
            amazon_code=$(curl -x "$proxy_url" --max-time 8 -s -o /dev/null -w "%{http_code}" https://www.amazon.com 2>&1)
            
            if [ "$google_code" = "200" ] || [ "$google_code" = "301" ] || [ "$google_code" = "302" ]; then
                printf "| ✅ Google "
                browsable=$((browsable + 1))
            else
                printf "| ❌ Google "
            fi
            
            if [ "$amazon_code" = "200" ] || [ "$amazon_code" = "301" ] || [ "$amazon_code" = "302" ]; then
                printf "| ✅ Amazon"
            else
                printf "| ❌ Amazon"
            fi
            echo ""
        else
            echo "  ❌ Connection failed"
        fi
        
    elif [ "$protocol" = "socks5" ]; then
        if curl --socks5 "${host}:${port}" --max-time 8 -s https://api.ipify.org > /dev/null 2>&1; then
            working=$((working + 1))
            printf "  ✅ Connected "
            
            google_code=$(curl --socks5 "${host}:${port}" --max-time 8 -s -o /dev/null -w "%{http_code}" https://www.google.com 2>&1)
            amazon_code=$(curl --socks5 "${host}:${port}" --max-time 8 -s -o /dev/null -w "%{http_code}" https://www.amazon.com 2>&1)
            
            if [ "$google_code" = "200" ] || [ "$google_code" = "301" ] || [ "$google_code" = "302" ]; then
                printf "| ✅ Google "
                browsable=$((browsable + 1))
            else
                printf "| ❌ Google "
            fi
            
            if [ "$amazon_code" = "200" ] || [ "$amazon_code" = "301" ] || [ "$amazon_code" = "302" ]; then
                printf "| ✅ Amazon"
            else
                printf "| ❌ Amazon"
            fi
            echo ""
        else
            echo "  ❌ Connection failed"
        fi
    fi
    
    echo ""
done

echo "=========================================="
echo "📊 Results Summary:"
echo "  Total tested: $total"
echo "  Connectable: $working"
echo "  Fully browsable: $browsable"
if [ $total -gt 0 ]; then
    success_rate=$((browsable * 100 / total))
    echo "  Success rate: ${success_rate}%"
fi
