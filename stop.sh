#!/bin/bash
# Novel Engine 停止脚本

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

BACKEND_PORT=8080
FRONTEND_PORT=5173

echo -e "$YELLOW正在停止 Novel Engine 服务...$NC"
echo ""

# 停止占用端口的进程
kill_port() {
    local port=$1
    local name=$2
    local pid=$(lsof -ti:$port 2>/dev/null)
    
    if [ -n "$pid" ]; then
        kill $pid 2>/dev/null || true
        echo -e "$GREEN✓ $name 已停止 (端口 $port)$NC"
    else
        echo -e "$GREEN✓ $name 未运行$NC"
    fi
}

kill_port $FRONTEND_PORT "Frontend"
kill_port $BACKEND_PORT "Backend"

echo ""
echo -e "$GREEN服务已停止 (PostgreSQL 保持运行)$NC"
echo -e "$YELLOW如需停止 PostgreSQL: brew services stop postgresql$NC"
