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
  /** 当前会话的上下文用量（估算，用于「快超了」预警）。 */
  const contextUsage = ref<agentApi.ContextUsage | null>(null)
  /** 进行中的 SSE 请求控制器：供「停止生成」中断。 */
  const streamAbort = ref<AbortController | null>(null)
  /**
   * 本轮 LLM 调用的用量统计（含提示缓存命中率）。
   *
   * 用途：判断网关侧的提示缓存是否在正常工作——命中率长期为 0
   * 通常意味着每轮都在重算（更慢、更贵），或缓存被网关驱逐。
   */
  const lastUsage = ref<agentApi.LlmUsage | null>(null)

  /**
   * 本轮已完成的工具调用数。
   *
   * 用途：批量操作（例如一次改 10 个地点）时，用户需要知道「还在跑第几个」，
   * 而不是面对一个静止的界面猜它是不是卡住了。
   */
  const roundToolCount = ref(0)

  /**
   * 上一轮结束后的小结（操作数 + 耗时）。
   *
   * 仅在**本轮确实调用过工具**时设置 —— 批量操作最容易让人困惑"到底结束没有"，
   * 普通的一问一答则不需要这种收尾提示。
   * 这是「本轮」的过程信息，刷新页面后清空。
   */
  const lastRoundSummary = ref<{ toolCalls: number; elapsedMs: number } | null>(null)

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
    contextUsage.value = null
    persistCurrent(session_id, projectId)
    await loadSessions(projectId)
  }

  /**
   * 刷新当前会话的上下文用量（后端按会话全部消息估算）。
   *
   * 失败时清空用量并把原因写入 error —— 不静默，避免用户以为「一切正常」。
   */
  async function refreshContextUsage() {
    const sid = sessionId.value
    if (!sid) {
      contextUsage.value = null
      return
    }
    try {
      contextUsage.value = await agentApi.getContextUsage(sid)
    } catch (e) {
      contextUsage.value = null
      error.value = `上下文用量获取失败：${(e as Error).message}`
    }
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
    await refreshContextUsage()
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

  /** 停止当前正在进行的生成（中断 SSE 请求）。 */
  function stopStreaming() {
    streamAbort.value?.abort()
  }

  /**
   * 截断会话：删除第 `fromIndex` 条消息及其之后的全部内容。
   *
   * 服务端返回截断后的完整会话，直接用它替换界面。
   * 注意：这同时清掉了后续的 AI 回复与工具记录，因此错误消息不会再污染上下文。
   */
  async function truncateFrom(fromIndex: number) {
    const sid = sessionId.value
    if (!sid) throw new Error('当前没有会话，无法删除')
    const s = await agentApi.truncateSession(sid, fromIndex)
    sessionId.value = s.id
    messages.value = s.messages.map((m) => ({
      role: m.role as ChatMessage['role'],
      content: m.content,
      formatted: true,
    }))
    error.value = null
    await refreshContextUsage()
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

    // 本轮进度：工具计数归零、清掉上一轮小结、记录开始时间
    roundToolCount.value = 0
    lastRoundSummary.value = null
    const startedAt = Date.now()

    // 「停止生成」用的中断控制器
    const controller = new AbortController()
    streamAbort.value = controller

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
        },        onToken: (t) => {
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
          // 累计本轮操作数：界面靠它显示「正在执行第 N 个操作」
          roundToolCount.value += 1
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
        onDone: async () => {
          thinking.value = false
          finalizeText()
          status.value = 'idle'
          // 立刻把本轮**所有**尚未格式化的助手气泡切到 markdown 渲染。
          // 一条回复里若有工具调用，工具前/后各有一条文本气泡；原先只处理最后一条，
          // 导致前面那条一直被当成纯文本显示（用户看到的"格式乱"，刷新后才正常）。
          for (let i = 0; i < messages.value.length; i++) {
            if (messages.value[i].role === 'assistant' && !messages.value[i].formatted) {
              messages.value[i] = { ...messages.value[i], formatted: true }
            }
          }
          // 注意：这里**不能**立刻向服务端拉取会话内容。
          // `done` 事件发出时，服务端的收尾落库可能仍在进行，此时读到的可能是
          // 尚未写入完成的中间状态（表现为"对话被截断到很靠前"）。
          // 统一改为等整个 SSE 流结束后再对齐（见 sendMessage 尾部）。
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
        onUsage: (u) => {
          lastUsage.value = u
        },
      }, controller.signal)
    } catch (e) {
      thinking.value = false
      finalizeText()

      // 用户主动「停止生成」：中断请求，丢弃这条未完成的助手气泡。
      // 服务端只有整轮跑完才落库，因此这条内容本来就不会被保存——
      // 前端一并移除才能保持前后端一致（否则刷新后它会凭空消失）。
      if (e instanceof DOMException && e.name === 'AbortError') {
        messages.value = messages.value.filter(
          (m) => !(m.role === 'assistant' && m.streaming),
        )
        textIdx = -1
        status.value = 'idle'
        return
      }

      const msg = e instanceof Error ? e.message : String(e)
      textIdx = ensureTextBubble()
      messages.value[textIdx].content += `\n\n[出错] ${msg}`
      messages.value[textIdx].streaming = false
      textIdx = -1
      status.value = 'error'
      error.value = msg
    } finally {
      streamAbort.value = null
    }

    // SSE 流已结束 —— 此时服务端的收尾落库也已完成，可以安全地以服务端为准对齐界面。
    // （等价于"刷新页面后看到的结果"，但不需要用户手动刷新。）
    if (status.value !== 'error') {
      try {
        await restoreSession(sid)
      } catch (e) {
        error.value = `对话已完成，但与服务端对齐内容失败：${(e as Error).message}`
      }

      // 本轮调用过工具时给出明确的收尾信号：批量操作（如一次改 10 个地点）
      // 最容易让人不确定"到底跑完没有"，这里补一条小结，避免对着静止界面猜。
      if (roundToolCount.value > 0) {
        lastRoundSummary.value = {
          toolCalls: roundToolCount.value,
          elapsedMs: Date.now() - startedAt,
        }
      }
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
    contextUsage,
    lastUsage,
    roundToolCount,
    lastRoundSummary,
    readPersistedSession,
    loadTools,
    loadSessions,
    newSession,
    restoreSession,
    refreshContextUsage,
    selectSession,
    deleteSession,
    renameSession,
    ensureSession,
    sendMessage,
    stopStreaming,
    truncateFrom,
    executeTool,
  }
})
