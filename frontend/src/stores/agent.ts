// Agent store (P1) - 管理会话、消息流、工具列表
import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as agentApi from '@/api/agent'

export interface ChatMessage {
  role: 'user' | 'assistant'
  content: string
  streaming?: boolean
}

const CURRENT_KEY = 'agent:currentSessionId'

export const useAgentStore = defineStore('agent', () => {
  const sessionId = ref<string | null>(null)
  const messages = ref<ChatMessage[]>([])
  const sessions = ref<agentApi.AgentSession[]>([])
  const tools = ref<agentApi.ToolMeta[]>([])
  const status = ref<'idle' | 'streaming' | 'error'>('idle')
  const thinking = ref(false)
  const error = ref<string | null>(null)

  function readPersistedSession(): string | null {
    try {
      return localStorage.getItem(CURRENT_KEY)
    } catch {
      return null
    }
  }

  function persistCurrent(id: string) {
    try {
      localStorage.setItem(CURRENT_KEY, id)
    } catch {
      // localStorage 不可用时忽略（仅影响刷新恢复）
    }
  }

  async function loadTools() {
    try {
      const data = await agentApi.listTools()
      tools.value = data.tools
    } catch (e) {
      // 工具列表失败不阻断聊天，仅留空
      tools.value = []
    }
  }

  async function loadSessions() {
    try {
      sessions.value = await agentApi.listSessions()
    } catch {
      sessions.value = []
    }
  }

  async function newSession(projectId?: string) {
    const { session_id } = await agentApi.createSession(projectId)
    sessionId.value = session_id
    messages.value = []
    error.value = null
    status.value = 'idle'
    persistCurrent(session_id)
    await loadSessions()
  }

  /** 从服务端恢复某个会话（刷新/切换页面后回填消息）。 */
  async function restoreSession(id: string) {
    const s = await agentApi.getSession(id)
    sessionId.value = s.id
    messages.value = s.messages.map((m) => ({
      role: (m.role === 'assistant' ? 'assistant' : 'user') as 'user' | 'assistant',
      content: m.content,
    }))
    error.value = null
    status.value = 'idle'
    persistCurrent(s.id)
  }

  /** 切换到某个已有会话。 */
  async function selectSession(id: string) {
    await restoreSession(id)
    await loadSessions()
  }

  async function deleteSession(id: string) {
    try {
      await agentApi.deleteSession(id)
    } catch {
      // 忽略删除失败
    }
    sessions.value = sessions.value.filter((s) => s.id !== id)
    if (sessionId.value === id) {
      await newSession()
    } else {
      await loadSessions()
    }
  }

  async function renameSession(id: string, title: string) {
    const t = title.trim()
    try {
      await agentApi.renameSession(id, t)
    } catch {
      // 忽略重命名失败
    }
    sessions.value = sessions.value.map((s) => (s.id === id ? { ...s, title: t } : s))
  }

  async function ensureSession(projectId?: string) {
    if (!sessionId.value) await newSession(projectId)
    return sessionId.value as string
  }

  async function sendMessage(text: string, projectId?: string) {
    const sid = await ensureSession(projectId)
    const content = text.trim()
    if (!content) return

    messages.value.push({ role: 'user', content })
    messages.value.push({ role: 'assistant', content: '', streaming: true })
    // 通过数组的响应式代理（按索引）修改，避免直接 mutate 原始对象导致不刷新
    const i = messages.value.length - 1
    status.value = 'streaming'
    thinking.value = true
    error.value = null

    try {
      await agentApi.streamChat(sid, content, {
        onStatus: () => {
          thinking.value = true
        },
        onToken: (t) => {
          thinking.value = false
          messages.value[i].content += t
        },
        onDone: () => {
          thinking.value = false
          messages.value[i].streaming = false
          status.value = 'idle'
        },
        onQuestion: (data) => {
          thinking.value = false
          // 把当前（空的）助手气泡替换为问题卡片（与后端持久化格式一致）
          const payload = JSON.stringify({ question: data.question, options: data.options })
          messages.value[i].content = `<<ASK_QUESTION>>${payload}<<END>>`
          messages.value[i].streaming = false
          status.value = 'idle'
        },
        onError: (e) => {
          thinking.value = false
          messages.value[i].streaming = false
          messages.value[i].content += `\n\n[出错] ${e}`
          status.value = 'error'
          error.value = e
        },
      })
    } catch (e) {
      thinking.value = false
      messages.value[i].streaming = false
      const msg = e instanceof Error ? e.message : String(e)
      messages.value[i].content += `\n\n[出错] ${msg}`
      status.value = 'error'
      error.value = msg
    }
    // 刷新历史列表（更新预览 / 时间 / 顺序）
    void loadSessions()
  }

  async function executeTool(name: string, input: unknown) {
    return agentApi.executeTool(name, input)
  }

  return {
    sessionId,
    messages,
    sessions,
    tools,
    status,
    thinking,
    error,
    readPersistedSession,
    loadTools,
    loadSessions,
    newSession,
    restoreSession,
    selectSession,
    deleteSession,
    renameSession,
    ensureSession,
    sendMessage,
    executeTool,
  }
})
