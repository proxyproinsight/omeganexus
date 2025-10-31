#!/bin/bash
# Test premium (mobile/residential) proxies for actual browsing capability

echo "🔍 Testing Premium Proxies (Mobile & Residential)"
echo "=================================================="
echo ""

# Get premium proxies from database
sqlite3 omega9.db "SELECT protocol, host, port, proxy_type, country FROM proxies WHERE proxy_type IN ('mobile', 'residential') AND active = 1 LIMIT 10;" | while IFS='|' read -r protocol host port type country; do
    
    proxy_url="${protocol}://${host}:${port}"
    
    echo "Testing: $proxy_url ($type - $country)"
    
    # Test 1: Can we connect?
    if [ "$protocol" = "http" ] || [ "$protocol" = "https" ]; then
        # HTTP/HTTPS proxy test
        response=$(curl -x "$proxy_url" \
            --max-time 10 \
            --connect-timeout 5 \
            -s -o /dev/null -w "%{http_code}|%{time_total}" \
            https://api.ipify.org 2>&1)
        
        if [ $? -eq 0 ]; then
            http_code=$(echo "$response" | cut -d'|' -f1)
            time_total=$(echo "$response" | cut -d'|' -f2)
            
            if [ "$http_code" = "200" ]; then
                # Get IP to verify proxy is working
                actual_ip=$(curl -x "$proxy_url" --max-time 10 -s https://api.ipify.org 2>/dev/null)
                
                if [ ! -z "$actual_ip" ]; then
                    echo "  ✅ SUCCESS - Response: ${http_code}, Time: ${time_total}s, IP: ${actual_ip}"
                    
                    # Test 2: Can we browse a real website?
                    web_test=$(curl -x "$proxy_url" \
                        --max-time 10 \
                        -s -o /dev/null -w "%{http_code}" \
                        https://www.google.com 2>&1)
                    
                    if [ "$web_test" = "200" ] || [ "$web_test" = "301" ] || [ "$web_test" = "302" ]; then
                        echo "  ✅ WEB TEST - Google.com accessible (HTTP ${web_test})"
                    else
                        echo "  ⚠️  WEB TEST - Google.com failed (HTTP ${web_test})"
                    fi
                else
                    echo "  ⚠️  Connected but couldn't get IP"
                fi
            else
                echo "  ❌ FAILED - HTTP ${http_code}"
            fi
        else
            echo "  ❌ FAILED - Connection timeout or error"
        fi
        
    elif [ "$protocol" = "socks5" ] || [ "$protocol" = "socks4" ]; then
        # SOCKS proxy test (requires curl with socks support)
        response=$(curl --socks5 "${host}:${port}" \
            --max-time 10 \
            --connect-timeout 5 \
            -s -o /dev/null -w "%{http_code}|%{time_total}" \
            https://api.ipify.org 2>&1)
        
        if [ $? -eq 0 ]; then
            http_code=$(echo "$response" | cut -d'|' -f1)
            time_total=$(echo "$response" | cut -d'|' -f2)
            
            if [ "$http_code" = "200" ]; then
                actual_ip=$(curl --socks5 "${host}:${port}" --max-time 10 -s https://api.ipify.org 2>/dev/null)
                
                if [ ! -z "$actual_ip" ]; then
                    echo "  ✅ SUCCESS - Response: ${http_code}, Time: ${time_total}s, IP: ${actual_ip}"
                    
                    web_test=$(curl --socks5 "${host}:${port}" \
                        --max-time 10 \
                        -s -o /dev/null -w "%{http_code}" \
                        https://www.google.com 2>&1)
                    
                    if [ "$web_test" = "200" ] || [ "$web_test" = "301" ] || [ "$web_test" = "302" ]; then
                        echo "  ✅ WEB TEST - Google.com accessible (HTTP ${web_test})"
                    else
                        echo "  ⚠️  WEB TEST - Google.com failed (HTTP ${web_test})"
                    fi
                else
                    echo "  ⚠️  Connected but couldn't get IP"
                fi
            else
                echo "  ❌ FAILED - HTTP ${http_code}"
            fi
        else
            echo "  ❌ FAILED - Connection timeout or error"
        fi
    fi
    
    echo ""
done

echo ""
echo "📊 Summary:"
sqlite3 omega9.db "SELECT COUNT(*) FROM proxies WHERE proxy_type IN ('mobile', 'residential') AND active = 1;" | xargs echo "Total active premium proxies:"
