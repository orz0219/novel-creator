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

# 函数: 检测端口是否被占用
is_port_used() {
    lsof -ti:$1 >/dev/null 2>&1
}

# 函数: 检测 PostgreSQL 是否可用
is_pg_ready() {
    pg_isready -h localhost -p $POSTGRES_PORT -q 2>/dev/null
}

# 函数: 检测并 kill 占用端口的进程 (跳过已运行的 PostgreSQL)
kill_port() {
    local port=$1
    local name=$2
    local skip_if_pg=$3
    
    # 如果是 PostgreSQL 端口且已可用，跳过
    if [ "$skip_if_pg" = "true" ] && is_pg_ready; then
        echo -e "$GREEN✓ $name 已在运行 (端口 $port) - 跳过$NC"
        return 0
    fi
    
    local pid=$(lsof -ti:$port 2>/dev/null)
    
    if [ -n "$pid" ]; then
        echo -e "$YELLOW⚠ 端口 $port ($name) 被进程 $pid 占用$NC"
        echo -e "$YELLOW  正在终止进程...$NC"
        kill -9 $pid 2>/dev/null || true
        sleep 1
        echo -e "$GREEN✓ 端口 $port 已释放$NC"
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
    
    echo -e "$YELLOW等待 $name 启动...$NC"
    while ! nc -z localhost $port 2>/dev/null; do
        sleep 1
        count=$((count + 1))
        if [ $count -ge $max_wait ]; then
            echo -e "$RED✗ $name 启动超时 ($max_wait s)$NC"
            return 1
        fi
    done
    echo -e "$GREEN✓ $name 已就绪$NC"
    return 0
}

# 步骤 1: 检测并释放端口 (PostgreSQL 如果已运行则跳过)
echo -e "$GREEN[1/4] 检测端口占用...$NC"
kill_port $POSTGRES_PORT "PostgreSQL" "true"
kill_port $BACKEND_PORT "Backend API" "false"
kill_port $FRONTEND_PORT "Frontend Dev Server" "false"
echo ""

# 步骤 2: 确保 PostgreSQL 运行
echo -e "$GREEN[2/4] 检查 PostgreSQL...$NC"
if is_pg_ready; then
    echo -e "$GREEN✓ PostgreSQL 已在运行$NC"
else
    echo -e "$YELLOWPostgreSQL 未运行，尝试启动...$NC"
    
    # 尝试 brew
    if command -v brew >/dev/null 2>&1; then
        brew services start postgresql@16 2>/dev/null || brew services start postgresql 2>/dev/null || true
    fi
    
    # 尝试 pg_ctl
    if command -v pg_ctl >/dev/null 2>&1; then
        pg_ctl -D /opt/homebrew/var/postgresql@16 start 2>/dev/null || pg_ctl -D /opt/homebrew/var/postgresql start 2>/dev/null || true
    fi
    
    sleep 2
    
    if is_pg_ready; then
        echo -e "$GREEN✓ PostgreSQL 已启动$NC"
    else
        echo -e "$RED✗ PostgreSQL 未安装或无法启动$NC"
        echo ""
        echo -e "$YELLOW请安装 PostgreSQL:$NC"
        echo -e "  brew install postgresql@16"
        echo -e "  brew services start postgresql@16"
        echo ""
        echo -e "$YELLOW或使用 Docker:$NC"
        echo -e "  docker run -d --name novel-pg \"
        echo -e "    -e POSTGRES_USER=novel \"
        echo -e "    -e POSTGRES_PASSWORD=novel_pass \"
        echo -e "    -e POSTGRES_DB=novel_engine \"
        echo -e "    -p 5432:5432 postgres:16-alpine"
        echo ""
        exit 1
    fi
fi
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
echo -e "$GREEN[4/4] 启动前端开发服务器...$NC"
if is_port_used $FRONTEND_PORT; then
    echo -e "$YELLOW前端已在运行 (端口 $FRONTEND_PORT)$NC"
else
    cd /Users/wangxingchao/Documents/novel/frontend
    npm run dev &
    FRONTEND_PID=$!
    wait_port $FRONTEND_PORT "Frontend Dev Server"
fi
echo ""

# 显示启动信息
echo -e "$GREEN========================================$NC"
echo -e "$GREEN  Novel Engine 已启动$NC"
echo -e "$GREEN========================================$NC"
echo ""
echo -e "  前端:    $GREEN http://localhost:$FRONTEND_PORT $NC"
echo -e "  后端:    $GREEN http://localhost:$BACKEND_PORT $NC"
echo -e "  数据库:  $GREEN localhost:$POSTGRES_PORT $NC"
echo ""
echo -e "  按 $YELLOW Ctrl+C $NC 停止服务"
echo ""

# 捕获 Ctrl+C
cleanup() {
    echo ""
    echo -e "$YELLOW正在停止服务...$NC"
    [ -n "$FRONTEND_PID" ] && kill $FRONTEND_PID 2>/dev/null
    [ -n "$BACKEND_PID" ] && kill $BACKEND_PID 2>/dev/null
    echo -e "$GREEN✓ 服务已停止$NC"
    exit 0
}
trap cleanup SIGINT SIGTERM
wait 2>/dev/null || true
