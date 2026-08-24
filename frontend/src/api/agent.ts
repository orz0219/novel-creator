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
  role: 'user' | 'assistant'
  content: string
  created_at?: string
}

export interface AgentSession {
  id: string
  project_id: string | null
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

export async function createSession(projectId?: string): Promise<CreateSessionResponse> {
  const resp = await fetch(BASE + '/session', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ project_id: projectId ?? null }),
  })
  if (!resp.ok) throw new Error(await errorText(resp))
  return resp.json()
}

export async function getSession(id: string): Promise<AgentSession> {
  const resp = await fetch(BASE + '/session/' + id)
  if (!resp.ok) throw new Error(await errorText(resp))
  return resp.json()
}

/** 列出全部会话（历史侧栏）。 */
export async function listSessions(): Promise<AgentSession[]> {
  const resp = await fetch(BASE + '/sessions')
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

export async function executeTool(name: string, input: unknown): Promise<ExecuteToolResponse> {
  const resp = await fetch(BASE + '/tool/execute', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ name, input }),
  })
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

export interface ChatStreamHandlers {
  onStatus?: (data: string) => void
  onToken?: (data: string) => void
  onDone?: () => void
  onError?: (data: string) => void
  onQuestion?: (data: { question: string; options: string[] }) => void
}

function parseSSEBlock(block: string): { event?: string; data?: string } {
  let event: string | undefined
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
 */
export async function streamChat(
  sessionId: string,
  message: string,
  handlers: ChatStreamHandlers,
): Promise<void> {
  const resp = await fetch(BASE + '/chat', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ session_id: sessionId, message }),
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
    else if (ev.event === 'question') {
      try {
        handlers.onQuestion?.(JSON.parse(ev.data))
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
