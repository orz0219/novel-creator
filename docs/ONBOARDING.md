# 上手指南（Onboarding）

> 目标读者：第一次接手本项目、需要**把它跑起来**或**读懂源码后再动手改**的开发者。所有步骤均基于 `macmini` 实测环境（Rust 1.97.1 / Node v24.18.0 / Postgres 16）。**未经静默兜底，遇错即停。**

## 1. 环境准备

| 组件 | 版本/状态 | 验证命令 |
|---|---|---|
| Rust (cargo) | 1.97.1 | `cargo --version` |
| Node / npm | v24.18.0 / 11.16.0 | `node -v && npm -v` |
| Postgres | 16（docker 容器 `novel-postgres`，端口 5432，已起） | `docker ps` 看 `novel-postgres` |
| sqlx 编译期宏 | **未使用** `query!` 宏 | `grep -r "query!" crates` 无命中 → 后端可**离线编译**（无需 DB 连接即可 `cargo build`） |

> 网络受限时可用本地代理 `http://127.0.0.1:1087`（仅下载慢/连不上时用，不要为健壮性加兜底）。

## 2. 启动数据库（Postgres）

项目依赖 docker `novel-postgres` 容器（已在 5432 监听）。若未起：

```bash
# 在项目根
docker compose up -d postgres      # 或等价命令拉起 novel-postgres
# 确认
docker ps | grep novel-postgres
```

数据库迁移位于 `crates/db/migrations/`（21 个，001 为权威 schema）。**运行后端服务前必须确保迁移已应用**：

```bash
# 设置连接串（请按实际环境填 password/host）
export DATABASE_URL="postgres://novel:novel@localhost:5432/novel"
# 应用迁移（sqlx-cli 或 db crate 自带 migrate 命令，具体见 crates/db 的 bin/README）
cargo run -p db --bin migrate
```

> 若迁移工具路径与预期不同，**先读 `crates/db/README.md` 或 `crates/db/Cargo.toml` 的 bin 定义**，不要猜测。

## 3. 后端构建与运行

```bash
cd /Users/wangxingchao/Documents/novel

# 编译（无需 DB，离线即可过）
cargo build                     # 全 workspace
cargo build -p narrative-engine # 仅顶层 API 服务

# 跑测试（DB 相关集成测试需 DATABASE_URL 已就绪）
cargo test

# 运行 API 服务（需要 DATABASE_URL + 已应用迁移）
export DATABASE_URL="postgres://novel:novel@localhost:5432/novel"
cargo run -p narrative-engine
# 默认监听 /api/v1/*，含 /api/health 健康检查
```

后端 crate 一览（依赖方向见 `docs/ARCHITECTURE_OVERVIEW.md` §2）：
`domain → db/infrastructure → application/ai/agent/runtime → narrative-engine`（顶层 host/API）。

## 4. 前端构建与运行

```bash
cd /Users/wangxingchao/Documents/novel/frontend
npm install
npm run dev        # 开发服务器（Vite）
# 或
npm run build      # 产物到 dist/
```

- 类型检查：`npm run type-check`（或 `vue-tsc`）。
- 单元测试：`npm run test`（vitest）。
- 前端调用前缀：常规 `/api/v1`（`client.ts:3` BASE_URL）；agent 模块独立硬编码 `/api/v1/agent`（`agent.ts:7`）。**两者当前一致**，但属重复定义（见 `docs/modules/api.md` §7.2）。

## 5. 阅读源码的推荐顺序

本仓库「先乱后清」，建议按以下顺序读，再配合 `docs/modules/`：

1. **顶层设计意图**：`ARCHITECTURE.md` → `BACKEND_SPEC.md` → `docs/ARCHITECTURE_OVERVIEW.md`。
2. **领域核心**：`crates/domain/src/`（实体、ports、validation、contract、generation）。这是一切的真源。
3. **依赖倒置出口**：`crates/db/src/`（Repo 实现 + migrations）+ `crates/infrastructure/src/`（LLM/artifacts）。
4. **编排层**：`crates/application` → `crates/runtime` → `crates/agent` → `crates/ai`。
5. **组合根 + API**：`crates/narrative-engine/src/main.rs` 与 `crates/narrative-engine/src/api/`。
6. **前端**：先 `frontend/src/types`（契约）→ `api` → `stores` → `pages`/`components`/`app`。

对应文档：`docs/modules/{domain,db,infrastructure,application,ai,agent,runtime,narrative-engine,types,api,stores,composables,pages,app,components}.md`。

## 6. 常见坑（源码已证实）

- **Context 功能整块不可用**：后端 `api/context.rs` 的 6 个 handler 全返回 501，前端 context 面板永远空白（`docs/modules/api.md` §7.1）。
- **生成进度条永不亮**：前端 `types/generation.ts` 缺后端 `Running` 态、多出中间态（`docs/modules/types.md` §4.6）。
- **角色字段读不到**：前端 `types/character.ts` 仍是 R2 前旧字段（`docs/modules/types.md` §4.3）。
- **`infrastructure` 的 LLM 调用忽略你传的 `model`**：`port_impl.rs:44` 丢弃 `model` 并硬编码 `max_tokens/temperature`（`docs/modules/infrastructure.md` §8.2）。
- **CORS 全开**：`api/mod.rs:28-31` 用 `Any/Any/Any`，仅适合本地开发，上线前必须收窄。

## 7. 改代码前的自检清单

- [ ] 改动是否绕过了「唯一写入口」原则？（见 `docs/ARCHITECTURE_OVERVIEW.md` §5.1 双写入口问题）
- [ ] 是否同步更新了 `domain` 实体 + `db` 迁移 + `frontend/src/types` 三源？任何一端遗漏都会造成 §6 的字段错位。
- [ ] 新增的 `ai`/`agent` 代码是否真的被 `lib.rs` 的 `mod` 声明？（避免重蹈 `ai` crate 三文件未编译的覆辙）
- [ ] 是否引入了新的死代码 / 重复组合根构造？（参考 `docs/ROADMAP.md` 阶段 0）
