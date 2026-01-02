#!/bin/bash

# Quick diagnostic script to check hybrid development setup

set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Load environment variables
source "${SCRIPT_DIR}/load-env.sh"

echo -e "${BLUE}🔍 Checking Hybrid Development Setup${NC}"
echo ""

# Check Docker containers
echo -e "${BLUE}📦 Docker Containers:${NC}"
if docker ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}" | grep -E "(NAME|arangodb|redis)" > /dev/null 2>&1; then
    docker ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}" | grep -E "(NAME|arangodb|redis)"
    echo ""
else
    echo -e "${RED}❌ No ArangoDB or Redis containers found${NC}"
    echo -e "${YELLOW}   Run: ./scripts/setup-hybrid-dev.sh${NC}"
    echo ""
fi

# Check ports
echo -e "${BLUE}🌐 Port Status:${NC}"
declare -a PORTS
PORTS=("${ARANGODB_PORT}:ArangoDB" "${BACKEND_PORT}:Backend" "${FRONTEND_PORT}:Frontend" "${REDIS_PORT}:Redis")
ALL_OK=true

for port_info in "${PORTS[@]}"; do
    IFS=':' read -r port name <<< "$port_info"
    if netstat -tlnp 2>/dev/null | grep -q ":$port " || ss -tlnp 2>/dev/null | grep -q ":$port "; then
        echo -e "  ${GREEN}✅ Port $port ($name) is listening${NC}"
    else
        echo -e "  ${RED}❌ Port $port ($name) is NOT listening${NC}"
        ALL_OK=false
        case $name in
            "Backend")
                echo -e "     ${YELLOW}   Start backend: VSCode Debug → 'Debug Backend (Hybrid Dev)' or:${NC}"
                echo -e "     ${YELLOW}   cargo run --package backend --bin backend${NC}"
                ;;
            "Frontend")
                echo -e "     ${YELLOW}   Start frontend: ./scripts/start-frontend.sh${NC}"
                ;;
            "ArangoDB")
                echo -e "     ${YELLOW}   Start dependencies: ./scripts/setup-hybrid-dev.sh${NC}"
                ;;
        esac
    fi
done
echo ""

# Check backend connectivity
if netstat -tlnp 2>/dev/null | grep -q ":${BACKEND_PORT} " || ss -tlnp 2>/dev/null | grep -q ":${BACKEND_PORT} "; then
    echo -e "${BLUE}🔌 Testing Backend API:${NC}"
    if curl -s -o /dev/null -w "%{http_code}" http://localhost:${BACKEND_PORT}/api/health 2>/dev/null | grep -q "200"; then
        echo -e "  ${GREEN}✅ Backend API is responding${NC}"
    else
        echo -e "  ${YELLOW}⚠️  Backend is running but /api/health returned non-200${NC}"
    fi
    echo ""
fi

# Check frontend connectivity
if netstat -tlnp 2>/dev/null | grep -q ":${FRONTEND_PORT} " || ss -tlnp 2>/dev/null | grep -q ":${FRONTEND_PORT} "; then
    echo -e "${BLUE}🔌 Testing Frontend:${NC}"
    if curl -s -o /dev/null -w "%{http_code}" http://localhost:${FRONTEND_PORT} 2>/dev/null | grep -q "200"; then
        echo -e "  ${GREEN}✅ Frontend is responding${NC}"
    else
        echo -e "  ${YELLOW}⚠️  Frontend is running but returned non-200${NC}"
    fi
    echo ""
    
    # Test proxy
    echo -e "${BLUE}🔌 Testing Frontend → Backend Proxy:${NC}"
    if curl -s -o /dev/null -w "%{http_code}" http://localhost:${FRONTEND_PORT}/api/health 2>/dev/null | grep -q "200"; then
        echo -e "  ${GREEN}✅ Frontend proxy to backend is working${NC}"
    else
        echo -e "  ${RED}❌ Frontend proxy to backend is NOT working${NC}"
        echo -e "     ${YELLOW}   Make sure backend is running on port ${BACKEND_PORT}${NC}"
    fi
    echo ""
fi

# Summary
if [ "$ALL_OK" = true ] && (netstat -tlnp 2>/dev/null | grep -q ":${BACKEND_PORT} " || ss -tlnp 2>/dev/null | grep -q ":${BACKEND_PORT} ") && (netstat -tlnp 2>/dev/null | grep -q ":${FRONTEND_PORT} " || ss -tlnp 2>/dev/null | grep -q ":${FRONTEND_PORT} "); then
    echo -e "${GREEN}✅ All services are running!${NC}"
    echo ""
    echo -e "${BLUE}📝 Access:${NC}"
    echo -e "  Frontend: ${GREEN}http://localhost:${FRONTEND_PORT}${NC}"
    echo -e "  Backend API: ${GREEN}http://localhost:${BACKEND_PORT}/api/${NC}"
else
    echo -e "${YELLOW}⚠️  Some services are not running. See above for details.${NC}"
fi

