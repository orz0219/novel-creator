#!/bin/bash
# Novel Engine 停止脚本

# 强制 UTF-8 编码
export LANG=en_US.UTF-8
export LC_ALL=en_US.UTF-8

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

BACKEND_PORT=8080
FRONTEND_PORT=5173

echo -e "${YELLOW}正在停止 Novel Engine 服务...${NC}"
echo ""

kill_port() {
    local port=$1
    local name=$2
    local pid=$(lsof -ti:$port 2>/dev/null)
    
    if [ -n "$pid" ]; then
        kill $pid 2>/dev/null || true
        echo -e "${GREEN}[OK] $name 已停止 (端口 $port)${NC}"
    else
        echo -e "${GREEN}[OK] $name 未运行${NC}"
    fi
}

kill_port $FRONTEND_PORT "Frontend"
kill_port $BACKEND_PORT "Backend"

echo ""
echo -e "${GREEN}服务已停止 (PostgreSQL 保持运行)${NC}"
echo -e "${YELLOW}如需停止 PostgreSQL: brew services stop postgresql${NC}"
