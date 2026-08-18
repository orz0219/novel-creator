#!/bin/bash
# Novel Engine 停止脚本

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

POSTGRES_PORT=5432
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
        kill -9 $pid 2>/dev/null || true
        echo -e "$GREEN✓ $name 已停止 (端口 $port)$NC"
    else
        echo -e "$GREEN✓ $name 未运行 (端口 $port)$NC"
    fi
}

kill_port $FRONTEND_PORT "Frontend"
kill_port $BACKEND_PORT "Backend"

echo ""
echo -e "$YELLOW是否停止 PostgreSQL? (y/N)$NC"
read -r response
if [[ "$response" =~ ^([yY][eE][sS]|[yY])$ ]]; then
    docker-compose down
    echo -e "$GREEN✓ PostgreSQL 已停止$NC"
else
    echo -e "$GREEN✓ PostgreSQL 保持运行$NC"
fi

echo ""
echo -e "$GREEN所有服务已停止$NC"
