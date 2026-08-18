#!/bin/bash
# Novel Engine 启动脚本
# 适配 Colima + Docker 环境

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

POSTGRES_PORT=5432
BACKEND_PORT=8080
FRONTEND_PORT=5173
DB_URL="postgresql://novel:novel_pass@localhost:$POSTGRES_PORT/novel_engine"

echo -e "$GREEN========================================$NC"
echo -e "$GREEN  Novel Engine 启动脚本$NC"
echo -e "$GREEN========================================$NC"
echo ""

# 函数: 检测端口占用
is_port_used() { lsof -ti:$1 >/dev/null 2>&1; }

# 函数: 检测 PostgreSQL 容器状态
get_pg_container_status() {
    docker ps -a --filter name=novel-postgres --format '{{.Status}}' 2>/dev/null
}

# 函数: 确保 Colima + Docker 运行
ensure_docker() {
    if ! docker info >/dev/null 2>&1; then
        echo -e "$YELLOWDocker 未运行，启动 Colima...$NC"
        colima start 2>&1 || true
        sleep 3
        if ! docker info >/dev/null 2>&1; then
            echo -e "$RED✗ 无法启动 Docker/Colima$NC"
            exit 1
        fi
    fi
    echo -e "$GREEN✓ Docker 已就绪$NC"
}

# 函数: 确保 PostgreSQL 运行
ensure_postgres() {
    local status=$(get_pg_container_status)
    
    if echo "$status" | grep -q "Up"; then
        echo -e "$GREEN✓ PostgreSQL 容器已在运行$NC"
        return 0
    fi
    
    if echo "$status" | grep -q "Exited"; then
        echo -e "$YELLOWPostgreSQL 容器已停止，正在启动...$NC"
        docker start novel-postgres
        sleep 2
        echo -e "$GREEN✓ PostgreSQL 容器已启动$NC"
        return 0
    fi
    
    # 容器不存在，创建
    echo -e "$YELLOWPostgreSQL 容器不存在，正在创建...$NC"
    docker run -d --name novel-postgres \
        -e POSTGRES_USER=novel \
        -e POSTGRES_PASSWORD=novel_pass \
        -e POSTGRES_DB=novel_engine \
        -p $POSTGRES_PORT:5432 \
        postgres:16-alpine
    sleep 3
    echo -e "$GREEN✓ PostgreSQL 容器已创建并启动$NC"
}

# 函数: 等待端口就绪
wait_port() {
    local port=$1
    local name=$2
    local max_wait=30
    local count=0
    echo -e "$YELLOW等待 $name 启动...$NC"
    while ! nc -z localhost $port 2>/dev/null; do
        sleep 1
        count=$((count + 1))
        if [ $count -ge $max_wait ]; then
            echo -e "$RED✗ $name 启动超时$NC"
            return 1
        fi
    done
    echo -e "$GREEN✓ $name 已就绪$NC"
}

# 步骤 1: 确保 Docker 运行
echo -e "$GREEN[1/4] 检查 Docker...$NC"
ensure_docker
echo ""

# 步骤 2: 确保 PostgreSQL 运行
echo -e "$GREEN[2/4] 检查 PostgreSQL...$NC"
ensure_postgres
wait_port $POSTGRES_PORT "PostgreSQL"
echo ""

# 步骤 3: 启动后端
echo -e "$GREEN[3/4] 启动后端服务...$NC"
if is_port_used $BACKEND_PORT; then
    echo -e "$YELLOW后端已在运行 (端口 $BACKEND_PORT)$NC"
else
    cd /Users/wangxingchao/Documents/novel
    DATABASE_URL=$DB_URL cargo run --bin narrative-engine &
    BACKEND_PID=$!
    wait_port $BACKEND_PORT "Backend API"
fi
echo ""

# 步骤 4: 启动前端
echo -e "$GREEN[4/4] 启动前端...$NC"
if is_port_used $FRONTEND_PORT; then
    echo -e "$YELLOW前端已在运行 (端口 $FRONTEND_PORT)$NC"
else
    cd /Users/wangxingchao/Documents/novel/frontend
    npm run dev &
    FRONTEND_PID=$!
    wait_port $FRONTEND_PORT "Frontend Dev Server"
fi
echo ""

echo -e "$GREEN========================================$NC"
echo -e "$GREEN  Novel Engine 已启动$NC"
echo -e "$GREEN========================================$NC"
echo ""
echo -e "  前端:    http://localhost:$FRONTEND_PORT"
echo -e "  后端:    http://localhost:$BACKEND_PORT"
echo -e "  数据库:  localhost:$POSTGRES_PORT"
echo ""
echo -e "  按 Ctrl+C 停止服务"
echo ""

cleanup() {
    echo ""
    echo -e "$YELLOW正在停止服务...$NC"
    [ -n "$FRONTEND_PID" ] && kill $FRONTEND_PID 2>/dev/null
    [ -n "$BACKEND_PID" ] && kill $BACKEND_PID 2>/dev/null
    echo -e "$GREEN✓ 服务已停止 (PostgreSQL 保持运行)$NC"
    exit 0
}
trap cleanup SIGINT SIGTERM
wait 2>/dev/null || true
