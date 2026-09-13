// 与电脑端完全一致的 HTTP API 客户端。
// 手机端引擎在 http://127.0.0.1:8080，接口路径与电脑端相同。

const BASE = 'http://127.0.0.1:8080/api/v1'

async function request(path, options = {}) {
  const resp = await fetch(BASE + path, {
    ...options,
    headers: { 'Content-Type': 'application/json', ...(options.headers || {}) },
  })
  if (!resp.ok) {
    const body = await resp.text()
    let detail = body
    try { detail = JSON.parse(body).error || JSON.parse(body).message || body } catch {}
    throw new Error(`HTTP ${resp.status} ${path} — ${detail || resp.statusText}`)
  }
  if (resp.status === 204) return undefined
  const text = await resp.text()
  return text ? JSON.parse(text) : undefined
}

const get = (p) => request(p)
const post = (p, data) => request(p, { method: 'POST', body: data ? JSON.stringify(data) : undefined })
const put = (p, data) => request(p, { method: 'PUT', body: data ? JSON.stringify(data) : undefined })
const del = (p) => request(p, { method: 'DELETE' })

export const api = {
  get, post, put, del,

  // 健康检查
  health: () => fetch(BASE + '/health').then((r) => r.text()),

  // 引擎就绪状态（含 AI 网关是否已配置）
  engineReady: () => get('/engine/ready'),

  // 项目
  listProjects: () => get('/projects'),
  getProject: (id) => get(`/projects/${id}`),
  createProject: (name, description) => post('/projects', { name, description }),
  updateProject: (id, data) => put(`/projects/${id}`, data),
  deleteProject: (id) => del(`/projects/${id}`),
  getWorld: (projectId) => get(`/projects/${projectId}/world`),

  // 会话
  listSessions: (projectId) => get(`/agent/sessions?project_id=${encodeURIComponent(projectId)}`),
  createSession: (projectId) => post('/agent/session', { project_id: projectId }),
  getSession: (id) => get(`/agent/session/${id}`),
  deleteSession: (id) => del(`/agent/session/${id}`),
  listTools: () => get('/agent/tools'),

  // 设置（字段名与电脑端设置页一致：aiBaseUrl / aiApiKey / defaultModel / …）
  getSettings: () => get('/settings'),
  saveSettings: (data) => put('/settings', data),
  testConnection: (data) => post('/settings/test-connection', data),
  listModels: (data) => post('/settings/models', data),
  /** 内置模型目录：模型 id → 上下文上限（网关不返回该数据，由后端内置）。 */
  getModelCatalog: () => get('/settings/model-catalog'),

  // 详情页附加数据
  characterRelationships: (id) => get(`/characters/${id}/relationships`),
  characterKnowledge: (id) => get(`/characters/${id}/knowledge`),
  characterState: (id) => get(`/characters/${id}/state`),
  locationEntities: (id) => get(`/locations/${id}/entities`),
  locationEvents: (id) => get(`/locations/${id}/events`),

  // 编辑（详情页就地改）
  //   实体基础信息：**部分更新**，只传想改的字段（不传的保持原值）
  //   档案 / 当前状态：**整行覆盖**，必须回传完整对象（漏掉的字段会被清空）
  //
  // 路径写成显式常量而不是拼字符串：写错资源名时 check-api-routes.js 能立刻发现。
  updateCharacter: (id, data) => put(`/characters/${id}`, data),
  updateLocation: (id, data) => put(`/locations/${id}`, data),
  updateFaction: (id, data) => put(`/factions/${id}`, data),
  /** 物品等通用实体（后端只有 /entities/{id} 这一个入口） */
  updateEntityGeneric: (id, data) => put(`/entities/${id}`, data),
  updateCharacterProfile: (id, data) => put(`/characters/${id}/profile`, data),
  updateCharacterState: (id, data) => put(`/characters/${id}/state`, data),
  updateLocationProfile: (id, data) => put(`/locations/${id}/profile`, data),
  updateFactionProfile: (id, data) => put(`/factions/${id}/profile`, data),

  /** 实体历史版本（服务端在每次改动前留下快照） */
  entityVersions: (id) => get(`/entities/${id}/versions`),
  /** 档案改动历史（角色档案 / 当前状态 / 地点档案 / 势力档案） */
  entityProfileHistory: (id) => get(`/entities/${id}/profile-history`),

  // 世界数据
  listRules: (worldId) => get(`/worlds/${worldId}/rules`),
  listRelations: (worldId) => get(`/worlds/${worldId}/relations`),
  listCharacters: (worldId) => get(`/worlds/${worldId}/characters`),
  listLocations: (worldId) => get(`/worlds/${worldId}/locations`),
  listFactions: (worldId) => get(`/worlds/${worldId}/factions`),

  // 故事
  listNodes: (projectId) => get(`/projects/${projectId}/narrative`),
  getNode: (id) => get(`/narrative/${id}`),
  createNode: (projectId, data) => post(`/projects/${projectId}/narrative`, data),
  updateNode: (id, data) => put(`/narrative/${id}`, data),
  deleteNode: (id) => del(`/narrative/${id}`),
  listStorylines: (projectId) => get(`/projects/${projectId}/storylines`),
  listForeshadows: (projectId) => get(`/projects/${projectId}/foreshadows`),

  // 更多：AI 提案（审批流）
  listProposals: (projectId) => get(`/projects/${projectId}/proposals`),
  getProposal: (id) => get(`/proposals/${id}`),
  acceptProposal: (id) => post(`/proposals/${id}/accept`),
  rejectProposal: (id) => post(`/proposals/${id}/reject`),
  validateProposal: (id) => post(`/proposals/${id}/validate`),

  // 更多：项目快照（存档与回滚）
  listSnapshots: (projectId) => get(`/projects/${projectId}/snapshots`),
  createSnapshot: (projectId, data) => post(`/projects/${projectId}/snapshots`, data),
  restoreSnapshot: (id) => post(`/snapshots/${id}/restore`),
  deleteSnapshot: (id) => del(`/snapshots/${id}`),

  listEvents: (projectId) => get(`/projects/${projectId}/events`),

  // AI 文本抽取：把一段正文抽成实体/关系候选（同时落成待审批草稿）
  //
  // 注意这是**同步等 LLM** 的接口，一次可能十几到几十秒；调用方要处理
  // 「切后台被挂起」的情况（手机端见 app.js 的 runExtract）。
  extractText: (projectId, text) => post(`/projects/${projectId}/extract`, { text }),
}

// ------------------------------------------------ 与电脑端通信（局域网导入）

/** 规范化用户输入的电脑地址，如 "192.168.1.11" 或 "192.168.1.11:8080" */
export function normalizeHost(input) {
  let h = (input || '').trim().replace(/\/+$/, '')
  if (!h) throw new Error('请填写电脑地址')
  if (!/^https?:\/\//.test(h)) h = 'http://' + h
  const url = new URL(h)
  // 未写端口时默认 8080（电脑端后端端口）
  if (!url.port) url.port = '8080'
  return url.origin
}

/** 列出电脑端（局域网）上的项目 */
export async function fetchRemoteProjects(host) {
  const base = normalizeHost(host)
  const resp = await fetch(`${base}/api/v1/projects`, { cache: 'no-store' })
  if (!resp.ok) throw new Error(`电脑端返回 HTTP ${resp.status}`)
  const list = await resp.json()
  if (!Array.isArray(list)) throw new Error('电脑端返回的项目列表格式异常')
  return list
}

/**
 * 拉取电脑端的全局设置（用于把 AI 网关配置带到手机）。
 *
 * 注意：电脑端的 API Key 通常来自环境变量而不写库，故这里多半拿不到
 * `aiApiKey`；密钥由构建时注入（见 src-tauri/build.rs）。
 */
export async function fetchRemoteSettings(host) {
  const base = normalizeHost(host)
  const resp = await fetch(`${base}/api/v1/settings`, { cache: 'no-store' })
  if (!resp.ok) throw new Error(`电脑端返回 HTTP ${resp.status}`)
  const data = await resp.json()
  if (!data || typeof data !== 'object') throw new Error('电脑端返回的设置格式异常')
  return data
}

/** 从电脑端拉取某个项目的导出数据，并导入本机。返回导入摘要。 */
export async function importFromComputer(host, projectId) {
  const base = normalizeHost(host)
  const resp = await fetch(`${base}/api/v1/projects/${projectId}/export`, { cache: 'no-store' })
  if (!resp.ok) throw new Error(`拉取项目失败 HTTP ${resp.status}`)
  const exportText = await resp.text()

  const local = await fetch('http://127.0.0.1:8080/api/v1/import', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: exportText,
  })
  const bodyText = await local.text()
  if (!local.ok) {
    let detail = bodyText
    try { detail = JSON.parse(bodyText).error || bodyText } catch {}
    throw new Error(detail)
  }
  return JSON.parse(bodyText)
}

/** 解析 SSE 块 */
function parseBlock(block) {
  let event = null
  let data = ''
  for (const line of block.split('\n')) {
    if (line.startsWith('event:')) event = line.slice(6).trim()
    else if (line.startsWith('data:')) data += line.slice(5).replace(/^ /, '')
  }
  return { event, data }
}

/**
 * 发送消息并消费 SSE 流。
 * 后端事件：status(thinking) → 多个 token → 可选 tool/question/usage → done | error
 */
export async function streamChat(sessionId, message, handlers, signal) {
  const resp = await fetch(BASE + '/agent/chat', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ session_id: sessionId, message }),
    signal,
  })
  if (!resp.ok || !resp.body) {
    handlers.onError?.(`HTTP ${resp.status}`)
    return
  }

  const reader = resp.body.getReader()
  const decoder = new TextDecoder()
  let buffer = ''

  const flush = (block) => {
    const { event, data } = parseBlock(block)
    if (!event) return
    try {
      if (event === 'status') handlers.onStatus?.(data)
      else if (event === 'token') handlers.onToken?.(data)
      else if (event === 'done') handlers.onDone?.()
      else if (event === 'error') handlers.onError?.(data)
      else if (event === 'usage') handlers.onUsage?.(JSON.parse(data))
      else if (event === 'question') handlers.onQuestion?.(JSON.parse(data))
      else if (event === 'tool') handlers.onTool?.(JSON.parse(data))
    } catch (e) {
      // 单个事件解析失败不应中断整条流，但要留下痕迹
      console.warn('[api] SSE 事件处理失败', event, e)
    }
  }

  for (;;) {
    const { done, value } = await reader.read()
    if (done) break
    buffer += decoder.decode(value, { stream: true })
    let idx
    while ((idx = buffer.indexOf('\n\n')) !== -1) {
      flush(buffer.slice(0, idx))
      buffer = buffer.slice(idx + 2)
    }
  }
  if (buffer.trim()) flush(buffer)
}
