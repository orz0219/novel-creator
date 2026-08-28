// Agent store (P1) - 管理会话、消息流、工具列表
import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as agentApi from '@/api/agent'

export interface ChatMessage {
  role: 'user' | 'assistant' | 'tool'
  content: string
  streaming?: boolean
  /**
   * 消息是否已"格式化"（流式完成）：
   * - true: 渲染 markdown
   * - false: 纯文本（流式中或刚收到时）
   * 历史消息（restoreSession）默认 true
   */
  formatted?: boolean
}

export const useAgentStore = defineStore('agent', () => {
  const sessionId = ref<string | null>(null)
  const messages = ref<ChatMessage[]>([])
  const sessions = ref<agentApi.AgentSession[]>([])
  const tools = ref<agentApi.ToolMeta[]>([])
  const status = ref<'idle' | 'streaming' | 'error'>('idle')
  const thinking = ref(false)
  const error = ref<string | null>(null)

  // 当前会话按项目分别持久化（对话-项目绑定）
  function keyFor(projectId: string): string {
    return `agent:currentSessionId:${projectId}`
  }

  function readPersistedSession(projectId: string): string | null {
    try {
      return localStorage.getItem(keyFor(projectId))
    } catch {
      return null
    }
  }

  function persistCurrent(id: string, projectId: string) {
    try {
      localStorage.setItem(keyFor(projectId), id)
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

  async function loadSessions(projectId: string) {
    try {
      sessions.value = await agentApi.listSessions(projectId)
    } catch {
      sessions.value = []
    }
  }

  async function newSession(projectId: string) {
    const { session_id } = await agentApi.createSession(projectId)
    sessionId.value = session_id
    messages.value = []
    error.value = null
    status.value = 'idle'
    persistCurrent(session_id, projectId)
    await loadSessions(projectId)
  }

  /** 从服务端恢复某个会话（刷新/切换项目后回填消息）。 */
  async function restoreSession(id: string): Promise<agentApi.AgentSession> {
    const s = await agentApi.getSession(id)
    sessionId.value = s.id
    messages.value = s.messages.map((m) => ({
      role: m.role as ChatMessage['role'],
      content: m.content,
      formatted: true, // 历史消息直接按 markdown 渲染
    }))
    error.value = null
    status.value = 'idle'
    if (s.project_id) persistCurrent(s.id, s.project_id)
    return s
  }

  /** 切换到某个已有会话（按所属项目刷新列表）。 */
  async function selectSession(id: string) {
    const s = await restoreSession(id)
    await loadSessions(s.project_id)
  }

  async function deleteSession(id: string, projectId: string) {
    try {
      await agentApi.deleteSession(id)
    } catch {
      // 忽略删除失败
    }
    sessions.value = sessions.value.filter((s) => s.id !== id)
    if (sessionId.value === id) {
      await newSession(projectId)
    } else {
      await loadSessions(projectId)
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

  /** 用户手动点 "排版" 按钮：把指定消息标 formatted=true 触发 markdown 渲染 */
  function markFormatted(index: number) {
    if (index < 0 || index >= messages.value.length) return
    messages.value[index] = { ...messages.value[index], formatted: true }
  }

  async function ensureSession(projectId: string) {
    // 当前会话不属于本项目时也重新创建，避免串项目
    if (!sessionId.value || !sessions.value.some((s) => s.id === sessionId.value)) {
      await newSession(projectId)
    }
    return sessionId.value as string
  }

  async function sendMessage(text: string, projectId: string) {
    const sid = await ensureSession(projectId)
    const content = text.trim()
    if (!content) return

    messages.value.push({ role: 'user', content })
    status.value = 'streaming'
    thinking.value = true
    error.value = null
    // 当前正在流式填充的助手文本气泡索引（-1 表示尚无）
    let textIdx = -1

    // 取得/新建一个流式助手文本气泡
    const ensureTextBubble = (): number => {
      const n = messages.value.length
      if (n > 0 && messages.value[n - 1].role === 'assistant' && messages.value[n - 1].streaming) {
        return n - 1
      }
      messages.value.push({ role: 'assistant', content: '', streaming: true, formatted: false })
      return messages.value.length - 1
    }
    // 收尾当前文本气泡（关闭 streaming），并复位索引
    const finalizeText = () => {
      if (textIdx >= 0 && messages.value[textIdx]) {
        messages.value[textIdx].streaming = false
      }
      textIdx = -1
    }

    try {
      await agentApi.streamChat(sid, content, {
        onStatus: () => {
          thinking.value = true
        },
        onToken: (t) => {
          thinking.value = false
          textIdx = ensureTextBubble()
          messages.value[textIdx].content += t
        },
        onQuestion: (data) => {
          thinking.value = false
          finalizeText()
          // 与后端持久化格式一致：选择题存为 assistant 的 <<ASK_QUESTION>> 标记
          const payload = JSON.stringify({ question: data.question, options: data.options })
          textIdx = ensureTextBubble()
          messages.value[textIdx].content = `<<ASK_QUESTION>>${payload}<<END>>`
          messages.value[textIdx].streaming = false
          textIdx = -1
          status.value = 'idle'
        },
        onTool: (data) => {
          thinking.value = false
          // 工具调用前先收尾当前文本气泡，使后续文本另起一条
          finalizeText()
          const payload = JSON.stringify({
            name: data.name,
            input: data.input,
            ok: data.ok,
            output: data.output,
          })
          messages.value.push({
            role: 'tool',
            content: `<<TOOL_RESULT>>${payload}<<END>>`,
            streaming: false,
          })
        },
        onDone: () => {
          thinking.value = false
          finalizeText()
          // 把刚刚 streaming 的消息标 formatted=true（让 ChatMessage 切到 v-html）。
          // 关键：用新对象替换整条消息才能触发 Vue 响应式（直接改属性追踪不到）。
          for (let i = messages.value.length - 1; i >= 0; i--) {
            if (messages.value[i].role === 'assistant' && !messages.value[i].formatted) {
              messages.value[i] = { ...messages.value[i], formatted: true }
              break
            }
          }
          status.value = 'idle'
        },
        onError: (e) => {
          thinking.value = false
          finalizeText()
          textIdx = ensureTextBubble()
          messages.value[textIdx].content += `\n\n[出错] ${e}`
          messages.value[textIdx].streaming = false
          textIdx = -1
          status.value = 'error'
          error.value = e
        },
      })
    } catch (e) {
      thinking.value = false
      finalizeText()
      const msg = e instanceof Error ? e.message : String(e)
      textIdx = ensureTextBubble()
      messages.value[textIdx].content += `\n\n[出错] ${msg}`
      messages.value[textIdx].streaming = false
      textIdx = -1
      status.value = 'error'
      error.value = msg
    }
    // 刷新历史列表（更新预览 / 时间 / 顺序）
    void loadSessions(projectId)
  }

  async function executeTool(name: string, input: unknown, projectId: string) {
    return agentApi.executeTool(name, input, projectId)
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
    markFormatted,
    ensureSession,
    sendMessage,
    executeTool,
  }
})
