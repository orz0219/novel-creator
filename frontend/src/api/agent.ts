// Agent API (P1) - /api/v1/agent/*
//
// 注意：现有 client.ts 的 BASE_URL 是 /api/v1，但 agent 端点走 POST + 命名 SSE 事件，
// 且 createSSE 用的 EventSource 只支持 GET + 默认 message 事件，故这里独立封装
// fetch + ReadableStream 手动解析 SSE。

const BASE = '/api/v1/agent'

async function errorText(resp: Response): Promise<string> {
  try {
    const j = await resp.json()
    return (j && (j.error || j.message)) || resp.statusText
  } catch {
    return resp.statusText
  }
}

export interface CreateSessionResponse {
  session_id: string
}
export interface ToolMeta {
  name: string
  description: string
  input_schema: unknown
}
export interface ListToolsResponse {
  tools: ToolMeta[]
}
export interface ExecuteToolResponse {
  name: string
  result: unknown
}

export interface ChatMessage {
  role: 'user' | 'assistant' | 'tool'
  content: string
  created_at?: string
}

export interface AgentSession {
  id: string
  project_id: string
  title: string | null
  messages: ChatMessage[]
  current_step: string
  created_at: string
  updated_at: string
}

export interface PromptView {
  scope: string
  /** 当前生效提示词（自定义或内置默认） */
  system_prompt: string
  /** 内置默认基座（用于「恢复默认」） */
  default_prompt: string
  /** 是否存在用户自定义覆盖 */
  is_customized: boolean
}

export async function createSession(projectId: string): Promise<CreateSessionResponse> {
  const resp = await fetch(BASE + '/session', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ project_id: projectId }),
  })
  if (!resp.ok) throw new Error(await errorText(resp))
  return resp.json()
}

export async function getSession(id: string): Promise<AgentSession> {
  const resp = await fetch(BASE + '/session/' + id)
  if (!resp.ok) throw new Error(await errorText(resp))
  return resp.json()
}

/** 列出某项目下的会话（历史侧栏，按项目隔离）。 */
export async function listSessions(projectId: string): Promise<AgentSession[]> {
  const resp = await fetch(BASE + '/sessions?project_id=' + encodeURIComponent(projectId))
  if (!resp.ok) throw new Error(await errorText(resp))
  return resp.json()
}

/** 删除会话（含消息）。 */
export async function deleteSession(id: string): Promise<void> {
  const resp = await fetch(BASE + '/session/' + id, { method: 'DELETE' })
  if (!resp.ok) throw new Error(await errorText(resp))
}

/** 重命名会话。 */
export async function renameSession(id: string, title: string): Promise<void> {
  const resp = await fetch(BASE + '/session/' + id, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ title }),
  })
  if (!resp.ok) throw new Error(await errorText(resp))
}

export async function listTools(): Promise<ListToolsResponse> {
  const resp = await fetch(BASE + '/tools')
  if (!resp.ok) throw new Error(await errorText(resp))
  return resp.json()
}

export async function executeTool(
  name: string,
  input: unknown,
  projectId: string,
): Promise<ExecuteToolResponse> {
  const resp = await fetch(BASE + '/tool/execute', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ name, input, project_id: projectId }),
  })
  if (!resp.ok) throw new Error(await errorText(resp))
  return resp.json()
}

/** confirm_step 的返回结构（来自后端 guide::ValidationReport 的 JSON 序列化） */
export interface ConfirmStepResult {
  passed: boolean
  current_step: string
  current_title: string
  next_step?: string
  next_title?: string
  missing?: Array<{ kind: string; detail: string }>
}

/** 推进当前引导阶段到下一步（用户在前端点"确认推进"按钮触发）。
 *  `targetStep` 可选：传入则跳到指定 step（血肉小选择器用）。
 */
export async function confirmGuideStep(
  projectId: string,
  targetStep?: string,
): Promise<ConfirmStepResult> {
  const resp = await fetch(BASE + '/guide/confirm', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ project_id: projectId, target_step: targetStep }),
  })
  if (!resp.ok) throw new Error(await errorText(resp))
  return resp.json()
}

/** 引导步骤的就绪情况（`GET /projects/{id}/guide/status`，后端真源）。 */
export interface GuideStepStatus {
  key: string
  title: string
  group: 'skeleton' | 'flesh'
  /** current = 当前所处阶段；complete = 产物已就绪；pending = 还缺产物 */
  status: 'current' | 'complete' | 'pending'
  /** 本步产物是否已满足——步骤条的对勾只看它 */
  ready: boolean
  next: string | null
  missing: Array<{ kind: string; detail: string }>
}

export interface GuideStatus {
  current_step: string
  current_title: string | null
  /** 当前阶段是否已达到推进条件 */
  current_ready: boolean
  steps: GuideStepStatus[]
}

/** 读取项目引导进度。
 *
 *  步骤条的对勾、每步的分组与标题、推进按钮的"还差什么"全部取自这里：
 *  与 `confirm_step` 的推进校验用的是后端同一套快照与同一个 `validate_step`。
 *  **不要在前端另行数数量判断完成度**——那会多出第二份判定标准，
 *  "副线"就曾因此永远不打勾（前端漏了这一步，且没有任何报错）。
 */
export async function getGuideStatus(projectId: string): Promise<GuideStatus> {
  const resp = await fetch(`/api/v1/projects/${projectId}/guide/status`)
  if (!resp.ok) throw new Error(await errorText(resp))
  return resp.json()
}

/** 读取当前生效提示词视图（含内置默认与是否自定义）。 */
export async function getPrompt(scope = 'global'): Promise<PromptView> {
  const resp = await fetch(`${BASE}/prompt?scope=${encodeURIComponent(scope)}`)
  if (!resp.ok) throw new Error(await errorText(resp))
  return resp.json()
}

/** 保存（upsert）自定义提示词基座。 */
export async function savePrompt(systemPrompt: string, scope = 'global'): Promise<PromptView> {
  const resp = await fetch(`${BASE}/prompt`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ scope, system_prompt: systemPrompt }),
  })
  if (!resp.ok) throw new Error(await errorText(resp))
  return resp.json()
}

/** 删除自定义覆盖（恢复内置默认）。 */
export async function deletePrompt(scope = 'global'): Promise<void> {
  const resp = await fetch(`${BASE}/prompt?scope=${encodeURIComponent(scope)}`, {
    method: 'DELETE',
  })
  if (!resp.ok) throw new Error(await errorText(resp))
}

/** 本轮调用的用量统计（含提示缓存命中）。 */
export interface LlmUsage {
  prompt_tokens: number
  completion_tokens: number
  total_tokens: number
  /** 命中提示缓存的 prompt token 数；网关未提供该信息时为 null。 */
  cached_tokens: number | null
  /** 缓存命中率 0~1；网关未提供缓存信息时为 null。 */
  cache_hit_rate: number | null
}

export interface ChatStreamHandlers {
  onStatus?: (data: string) => void
  onToken?: (data: string) => void
  onDone?: () => void
  onError?: (data: string) => void
  onQuestion?: (data: { question: string; options: string[] }) => void
  onTool?: (data: { name: string; input: unknown; ok: boolean; output: string }) => void
  /** 本轮用量统计（在 done 之前下发一次）。 */
  onUsage?: (data: LlmUsage) => void
}

/** 会话上下文用量（估算值，用于聊天页「快超了」预警）。 */
export interface ContextUsage {
  used_tokens: number
  limit_tokens: number
  message_count: number
  model: string
}

/** 读取某会话的上下文用量（后端按会话全部消息估算）。 */
export async function getContextUsage(sessionId: string): Promise<ContextUsage> {
  const resp = await fetch(`${BASE}/session/${sessionId}/context`)
  if (!resp.ok) throw new Error(await errorText(resp))
  return resp.json()
}

/**
 * 截断会话：删除第 `fromIndex` 条消息及其之后的**全部**内容
 * （含随后产生的用户消息、AI 回复与工具记录），返回截断后的完整会话。
 *
 * 用于清掉发错或跑偏的消息，避免它污染后续上下文。
 */
export async function truncateSession(
  sessionId: string,
  fromIndex: number,
): Promise<AgentSession> {
  const resp = await fetch(`${BASE}/session/${sessionId}/truncate`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ from_index: fromIndex }),
  })
  if (!resp.ok) throw new Error(await errorText(resp))
  return resp.json()
}

/** 滚动摘要内容（固定字段，与后端 `domain::session_summary::SessionSummary` 对齐）。 */
export interface SessionSummary {
  story_state: string
  confirmed: string[]
  open_threads: string[]
  next_step: string
}

/** 带元信息的滚动摘要（项目级恒定一份）。 */
export interface StoredSessionSummary {
  content: SessionSummary
  updated_at: string
}

/**
 * 读取当前会话所属项目的滚动摘要；从未收尾过时返回 `null`（不是错误）。
 */
export async function getSessionSummary(
  sessionId: string,
): Promise<StoredSessionSummary | null> {
  const resp = await fetch(`${BASE}/session/${sessionId}/summary`)
  if (!resp.ok) throw new Error(await errorText(resp))
  return resp.json()
}

/**
 * 会话收尾：让模型把这轮会话归纳成结构化摘要并**覆盖**项目那一份。
 *
 * 会连同「旧摘要」一起作为基线喂给模型，因此重复收尾不会丢掉前几轮的结论；
 * 生成后写进项目记忆，新建会话会自动读到。
 */
export async function summarizeSession(
  sessionId: string,
): Promise<StoredSessionSummary> {
  const resp = await fetch(`${BASE}/session/${sessionId}/summary`, {
    method: 'POST',
  })
  if (!resp.ok) throw new Error(await errorText(resp))
  return resp.json()
}

function parseSSEBlock(block: string): { event?: string; data?: string } {  let event: string | undefined
  let data = ''
  for (const line of block.split('\n')) {
    if (line.startsWith('event:')) event = line.slice(6).trim()
    else if (line.startsWith('data:')) data += line.slice(5).replace(/^ /, '')
  }
  return { event, data }
}

/**
 * 发送一条消息并消费 SSE 流。
 * 后端按命名事件下发：status(thinking) → 多个 token → done / error。
 *
 * `signal` 用于「停止生成」：中断后本函数会抛出 AbortError，由调用方识别为主动停止。
 */
export async function streamChat(
  sessionId: string,
  message: string,
  handlers: ChatStreamHandlers,
  signal?: AbortSignal,
): Promise<void> {
  const resp = await fetch(BASE + '/chat', {
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

  const flushBlock = (block: string) => {
    const ev = parseSSEBlock(block)
    if (!ev.event || ev.data === undefined) return
    if (ev.event === 'status') handlers.onStatus?.(ev.data)
    else if (ev.event === 'token') handlers.onToken?.(ev.data)
    else if (ev.event === 'done') handlers.onDone?.()
    else if (ev.event === 'error') handlers.onError?.(ev.data)
    else if (ev.event === 'usage') {
      try {
        handlers.onUsage?.(JSON.parse(ev.data) as LlmUsage)
      } catch (e) {
        // 用量是辅助信息，解析失败不应打断对话；但要留下痕迹便于排查
        console.warn('[agent] 用量事件解析失败:', e, ev.data)
      }
    } else if (ev.event === 'question') {
      try {
        handlers.onQuestion?.(JSON.parse(ev.data))
      } catch {
        // 解析失败忽略
      }
    } else if (ev.event === 'tool') {
      try {
        handlers.onTool?.(JSON.parse(ev.data))
      } catch {
        // 解析失败忽略
      }
    }
  }

  while (true) {
    const { done, value } = await reader.read()
    if (done) break
    buffer += decoder.decode(value, { stream: true })
    let idx: number
    while ((idx = buffer.indexOf('\n\n')) !== -1) {
      const block = buffer.slice(0, idx)
      buffer = buffer.slice(idx + 2)
      flushBlock(block)
    }
  }
  // 处理末尾可能未以 \n\n 结尾的残留块
  if (buffer.trim()) flushBlock(buffer)
}
