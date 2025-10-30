#!/bin/bash
set -e

echo "🔥 Omega9-NEXUS v15.0 Deployment Script"
echo "========================================"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if running as root
if [[ $EUID -eq 0 ]]; then
   echo -e "${RED}This script should NOT be run as root${NC}"
   exit 1
fi

# Verify Docker is installed
if ! command -v docker &> /dev/null; then
    echo -e "${RED}Docker is not installed. Install it first:${NC}"
    echo "sudo apt install -y docker.io docker-compose"
    exit 1
fi

# Verify environment variables
if [ -z "$TELOXIDE_TOKEN" ]; then
    echo -e "${YELLOW}TELOXIDE_TOKEN not set in environment${NC}"
    echo "Either export it or add to .env file"
    echo "Example: export TELOXIDE_TOKEN='your_bot_token'"
    
    if [ ! -f .env ]; then
        echo -e "${YELLOW}Creating .env from .env.example${NC}"
        cp .env.example .env
        echo -e "${RED}Please edit .env and add your TELOXIDE_TOKEN, then run this script again${NC}"
        exit 1
    fi
fi

# Create data directory
echo -e "${GREEN}Creating data directory...${NC}"
mkdir -p data

# Stop existing containers
echo -e "${GREEN}Stopping existing containers...${NC}"
docker compose down 2>/dev/null || true

# Clean old builds
echo -e "${GREEN}Cleaning old builds...${NC}"
docker system prune -f

# Build new image
echo -e "${GREEN}Building Docker image...${NC}"
docker compose build

# Start services
echo -e "${GREEN}Starting services...${NC}"
docker compose up -d

# Wait for services to start
echo -e "${GREEN}Waiting for services to start...${NC}"
sleep 5

# Check container status
echo -e "${GREEN}Container status:${NC}"
docker ps | grep omega9

# Show logs
echo ""
echo -e "${GREEN}Recent logs:${NC}"
docker logs --tail 50 omega9-nexus

echo ""
echo -e "${GREEN}========================================"
echo "✅ Deployment complete!"
echo "========================================"
echo ""
echo "📊 Dashboard: http://localhost:8080"
echo "📱 Telegram: Send /start to your bot"
echo "📝 Logs: docker logs -f omega9-nexus"
echo "🛑 Stop: docker compose down"
echo ""
echo "Environment variables loaded:"
if [ -f .env ]; then
    echo "  - From .env file"
fi
if [ ! -z "$TELOXIDE_TOKEN" ]; then
    echo "  - TELOXIDE_TOKEN: ${TELOXIDE_TOKEN:0:10}..."
fi
echo ""
echo -e "${YELLOW}Note: First hunt cycle will start in ~5 minutes${NC}"
echo -e "${YELLOW}Discovery cycle will start in ~1 hour${NC}"
echo ""
