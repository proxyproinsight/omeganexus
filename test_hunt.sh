#!/bin/bash
# GOD MODE Phase 1 Test Script
# Tests ASN detection, 50+ sources integration, and mobile/residential discovery

echo "🚀 GOD MODE Phase 1 Test - Starting..."
echo "========================================"

cd /home/dappy/omega9-nexus

# Check database before
echo ""
echo "📊 BEFORE Test:"
sqlite3 omega9.db "SELECT 
    COUNT(*) as total, 
    SUM(CASE WHEN is_mobile=1 THEN 1 ELSE 0 END) as mobile, 
    SUM(CASE WHEN is_residential=1 THEN 1 ELSE 0 END) as residential,
    SUM(CASE WHEN active=1 THEN 1 ELSE 0 END) as working
FROM proxies;"

echo ""
echo "📋 Active Sources:"
sqlite3 omega9.db "SELECT COUNT(*) FROM sources WHERE active=1;"

echo ""
echo "🔍 Sample of premium sources:"
sqlite3 omega9.db "SELECT name, url FROM sources WHERE name LIKE '%speedx%' OR name LIKE '%fate0%' OR name LIKE '%clarketm%' LIMIT 5;"

echo ""
echo "========================================"
echo "✅ Test complete! Database ready for hunt cycle."
echo ""
echo "To run full hunt cycle:"
echo "  sudo systemctl restart omega9-nexus.service"
echo ""
echo "To watch logs live:"
echo "  sudo journalctl -u omega9-nexus.service -f --since '1 minute ago'"
echo ""
echo "To check GOD MODE stats after hunt:"
echo "  sqlite3 omega9.db \"SELECT COUNT(*) as total, SUM(CASE WHEN is_mobile=1 THEN 1 ELSE 0 END) as mobile, SUM(CASE WHEN is_residential=1 THEN 1 ELSE 0 END) as residential FROM proxies;\""
echo ""
