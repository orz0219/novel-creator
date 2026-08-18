#!/bin/bash
# Novel Engine 启动脚本
# 功能: 检测端口占用，自动 kill 占用进程，启动前后端服务

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# 端口配置
POSTGRES_PORT=5432
BACKEND_PORT=8080
FRONTEND_PORT=5173

# 数据库配置
DB_URL="postgresql://novel:novel_pass@localhost:$POSTGRES_PORT/novel_engine"

echo -e "$GREEN========================================$NC"
echo -e "$GREEN  Novel Engine 启动脚本$NC"
echo -e "$GREEN========================================$NC"
echo ""

# 函数: 检测并 kill 占用端口的进程
kill_port() {
    local port=$1
    local name=$2
    
    local pid=$(lsof -ti:$port 2>/dev/null)
    
    if [ -n "$pid" ]; then
        echo -e "$YELLOW⚠ 端口 $port ($name) 被进程 $pid 占用$NC"
        echo -e "$YELLOW  正在终止进程...$NC"
        kill -9 $pid 2>/dev/null || true
        sleep 1
        
        local check_pid=$(lsof -ti:$port 2>/dev/null)
        if [ -n "$check_pid" ]; then
            echo -e "$RED✗ 无法终止进程 $check_pid$NC"
            return 1
        else
            echo -e "$GREEN✓ 端口 $port 已释放$NC"
        fi
    else
        echo -e "$GREEN✓ 端口 $port ($name) 可用$NC"
    fi
    return 0
}

# 函数: 等待端口就绪
wait_port() {
    local port=$1
    local name=$2
    local max_wait=30
    local count=0
    
    echo -e "$YELLOW等待 $name 启动 (端口 $port)...$NC"
    while ! nc -z localhost $port 2>/dev/null; do
        sleep 1
        count=$((count + 1))
        if [ $count -ge $max_wait ]; then
            echo -e "$RED✗ $name 启动超时 ($max_wait s)$NC"
            return 1
        fi
    done
    echo -e "$GREEN✓ $name 已启动 (端口 $port)$NC"
    return 0
}

# 步骤 1: 检测并释放端口
echo -e "$GREEN[1/4] 检测端口占用...$NC"
kill_port $POSTGRES_PORT "PostgreSQL"
kill_port $BACKEND_PORT "Backend API"
kill_port $FRONTEND_PORT "Frontend Dev Server"
echo ""

# 步骤 2: 启动 PostgreSQL
echo -e "$GREEN[2/4] 启动 PostgreSQL...$NC"
docker-compose up -d
wait_port $POSTGRES_PORT "PostgreSQL"
echo ""

# 步骤 3: 启动后端
echo -e "$GREEN[3/4] 启动后端服务...$NC"
DATABASE_URL=$DB_URL cargo run --bin narrative-engine &
BACKEND_PID=$!
wait_port $BACKEND_PORT "Backend API"
echo ""

# 步骤 4: 启动前端
echo -e "$GREEN[4/4] 启动前端开发服务器...$NC"
cd frontend && npm run dev &
FRONTEND_PID=$!
wait_port $FRONTEND_PORT "Frontend Dev Server"
echo ""

# 显示启动信息
echo -e "$GREEN========================================$NC"
echo -e "$GREEN  Novel Engine 已启动$NC"
echo -e "$GREEN========================================$NC"
echo ""
echo -e "  前端: $GREEN http://localhost:$FRONTEND_PORT $NC"
echo -e "  后端: $GREEN http://localhost:$BACKEND_PORT $NC"
echo -e "  数据库: $GREEN localhost:$POSTGRES_PORT $NC"
echo ""
echo -e "  按 $YELLOW Ctrl+C $NC 停止所有服务"
echo ""

# 捕获 Ctrl+C 信号，优雅关闭
cleanup() {
    echo ""
    echo -e "$YELLOW正在停止服务...$NC"
    
    if [ -n "$FRONTEND_PID" ]; then
        kill $FRONTEND_PID 2>/dev/null || true
        echo -e "$GREEN✓ 前端已停止$NC"
    fi
    
    if [ -n "$BACKEND_PID" ]; then
        kill $BACKEND_PID 2>/dev/null || true
        echo -e "$GREEN✓ 后端已停止$NC"
    fi
    
    echo -e "$GREEN✓ 所有服务已停止$NC"
    exit 0
}

trap cleanup SIGINT SIGTERM

# 等待子进程
wait
