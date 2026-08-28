<template>
  <div class="agent-page">
    <!-- 中间：对话主区 -->
    <section class="agent-chat">
      <header class="chat-header">
        <div class="chat-id">
          <div class="chat-title">{{ projectName || '创作引导' }}</div>
        </div>
        <div class="chat-status">
          <span class="dot" :class="statusDot"></span>{{ statusText }}
        </div>
        <div class="chat-actions">
          <button class="ghost-btn" @click="showPrompt = true"><Settings2 :size="16" /><span>提示词</span></button>
          <button class="primary-btn" @click="onNewSession" :disabled="store.status === 'streaming'">
            <Plus :size="16" /><span>新建会话</span>
          </button>
        </div>
      </header>

      <!-- 6 步进度条 -->
      <StepIndicator
        :current="guideStep"
        :steps="guideSteps"
        :loading="store.status === 'streaming'"
      />

      <div class="messages" ref="messagesEl">
        <div class="msg-col">
          <div v-if="store.messages.length === 0" class="empty">
            <div class="empty-seal">笔</div>
            <div class="empty-title">开始你的创作之旅</div>
            <div class="empty-sub">用自然语言描述你的想法，向导会逐步帮你构建世界观、角色与剧情。</div>
            <div class="chips">
              <button v-for="s in suggestions" :key="s" class="chip" @click="send(s)">
                <Lightbulb :size="14" /><span>{{ s }}</span>
              </button>
            </div>
          </div>

          <ChatMessage
            v-for="(m, i) in store.messages"
            :key="i"
            :role="m.role"
            :content="m.content"
            :streaming="m.streaming"
            :formatted="m.formatted"
            @select="onSelect"
            @retry="onRetry"
            @format="onFormat(i)"
          />

          <!-- Thinking 指示器：用户发完消息 + LLM 还没返回第一 token 时显示
               越简单越好：一个小气泡 + 三个跳动的点 -->
          <div v-if="store.thinking" class="thinking-bubble">
            <span class="thinking-avatar">导</span>
            <div class="thinking-content">
              <span class="thinking-text">正在思考</span>
              <span class="thinking-dots">
                <span></span><span></span><span></span>
              </span>
            </div>
          </div>
        </div>
      </div>

      <div v-if="store.error" class="error-bar"><AlertTriangle :size="14" /> {{ store.error }}</div>

      <div class="composer">
        <textarea
          class="composer-input"
          v-model="inputText"
          rows="2"
          placeholder="描述你的创作想法，Enter 发送，Shift+Enter 换行"
          @keydown="onKeydown"
        ></textarea>
        <div class="composer-toolbar">
          <ConfirmAdvance
            class="composer-advance"
            :current-title="currentGuideTitle"
            :next-title="nextGuideTitle"
            :project-id="projectId"
            :flesh-steps="fleshStatuses"
            @advanced="onAdvanced"
          />
          <button
            class="send-btn"
            :class="{ primary: canSend }"
            :disabled="!canSend"
            @click="send(inputText)"
          >
            <Send :size="16" /><span>发送</span>
          </button>
        </div>
      </div>
    </section>

    <!-- 右侧：会话历史 -->
    <aside class="agent-history">
      <div class="history-head">
        <span class="history-title">历史会话</span>
      </div>
      <div class="history-list">
        <div v-if="store.sessions.length === 0" class="history-empty">暂无会话</div>
        <div
          v-for="s in store.sessions"
          :key="s.id"
          class="history-item"
          :class="{ active: s.id === store.sessionId }"
          @click="onSelectSession(s.id)"
        >
          <div class="hi-main">
            <div class="hi-title">{{ sessionTitle(s) }}</div>
            <div class="hi-meta">{{ sessionMeta(s) }}</div>
          </div>
          <div class="hi-actions" @click.stop>
            <button class="hi-btn" title="重命名" @click="startRename(s)"><Pencil :size="14" /></button>
            <button class="hi-btn danger" title="删除" @click="onDeleteSession(s.id)"><Trash2 :size="14" /></button>
          </div>
          <input
            v-if="editingId === s.id"
            class="hi-rename"
            v-model="editingTitle"
            v-focus
            @keyup.enter="commitRename(s.id)"
            @blur="commitRename(s.id)"
          />
        </div>
      </div>
    </aside>

    <PromptEditor v-model="showPrompt" />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, nextTick, watch } from 'vue'
import { useRoute } from 'vue-router'
import {
  Plus, Pencil, Trash2, AlertTriangle, Lightbulb, Settings2, Send,
} from 'lucide-vue-next'
import { useAgentStore } from '@/stores/agent'
import { useProjectStore } from '@/stores/project'
import type { AgentSession } from '@/api/agent'
import { characterApi } from '@/api/character'
import { locationApi } from '@/api/location'
import { worldApi } from '@/api/world'
import ChatMessage from '@/components/agent/ChatMessage.vue'
import PromptEditor from '@/components/agent/PromptEditor.vue'
import StepIndicator from '@/components/agent/StepIndicator.vue'
import ConfirmAdvance, { type FleshStep } from '@/components/agent/ConfirmAdvance.vue'

const store = useAgentStore()
const projectStore = useProjectStore()
const route = useRoute()
const projectId = computed(() => route.params.id as string)
const inputText = ref('')
const messagesEl = ref<HTMLElement | null>(null)
const showPrompt = ref(false)
const editingId = ref('')
const editingTitle = ref('')

// 内联重命名输入框自动聚焦
const vFocus = { mounted: (el: HTMLElement) => el.focus() }

const suggestions = [
  '我想写一个东方玄幻小说，主角是个被废掉修为的少年',
  '帮我设计一个魔法世界的底层规则',
  '创建一个叫黑炎帝国的反派势力',
]

const projectName = computed(() => projectStore.currentProject?.name ?? '')
const activeSession = computed(() => store.sessions.find((s) => s.id === store.sessionId))
const currentStep = computed(() => activeSession.value?.current_step ?? '')

// ---------- 10 步引导（5 骨架 + 4 血肉 + 1 占位） ----------
const guideSteps = [
  // 骨架
  { key: 'premise',         title: '脑洞',   group: 'skeleton' as const },
  { key: 'world',           title: '世界观', group: 'skeleton' as const },
  // 血肉
  { key: 'world.map',       title: '地图',   group: 'flesh' as const },
  { key: 'world.factions',  title: '势力',   group: 'flesh' as const },
  { key: 'world.items',     title: '道具',   group: 'flesh' as const },
  // 骨架
  { key: 'golden_finger',   title: '金手指', group: 'skeleton' as const },
  { key: 'protagonist',     title: '主角',   group: 'skeleton' as const },
  // 血肉
  { key: 'characters.supporting', title: '配角', group: 'flesh' as const },
  // 骨架
  { key: 'storylines.main',      title: '故事线', group: 'skeleton' as const },
  // 血肉
  { key: 'storylines.branches',   title: '副线',   group: 'flesh' as const },
  // 骨架
  { key: 'beats',                title: '细纲',   group: 'skeleton' as const },
]
// 当前阶段：优先读 project.config.current_step（项目级，多 session 共享）；
// 兜底用 session.current_step；都没有则默认 'premise'。
const guideStep = computed(() => {
  // 优先：project.config.current_step（项目级，多 session 共享）
  const fromConfig = (projectStore.currentProject?.config as any)?.current_step as string | undefined
  if (fromConfig) return fromConfig
  // 兜底：session.current_step——但后端默认是中文'项目初始化'，
  // 不在 guideSteps 里，所以只在它能匹配时用
  if (currentStep.value && guideSteps.some((s) => s.key === currentStep.value)) {
    return currentStep.value
  }
  return 'premise'
})
const guideIndex = computed(() => guideSteps.findIndex((s) => s.key === guideStep.value))
const currentGuideTitle = computed(
  () => guideSteps[guideIndex.value]?.title ?? guideStep.value,
)
const nextGuideTitle = computed(
  () => guideSteps[guideIndex.value + 1]?.title ?? '',
)

// ---------- 血肉 step 完成度（驱动小选择器） ----------
const fleshStatuses = ref<FleshStep[]>([
  { key: 'world.map',              title: '地图', done: false },
  { key: 'world.factions',         title: '势力', done: false },
  { key: 'world.items',            title: '道具', done: false },
  { key: 'characters.supporting',  title: '配角', done: false },
])

/** 从后端拉 4 类血肉的实际数量，更新 fleshStatuses.done。
 *  注：4 个 API 失败时静默 catch——某些 entity 端点可能临时 500
 *  （如 /entities?type=* 偶发 500 已知），不能因此阻塞小选择器渲染；
 *  失败则保持上次状态（不更新 done）。
 */
async function refreshFleshStatus() {
  const pid = projectId.value
  if (!pid) return
  // 拿主 world_id
  let wid: string | null = null
  try {
    const world = await worldApi.get(pid)
    if (world) wid = world.id
  } catch {
    return // world 拿不到就不更新
  }
  if (!wid) return

  // 并行 4 个查询——任一失败不阻塞其它
  const [locations, factions, items, characters] = await Promise.all([
    locationApi.list(wid).catch(() => null),
    worldApi.listEntities(wid, 'Faction').catch(() => null),
    worldApi.listEntities(wid, 'Item').catch(() => null),
    characterApi.list(wid).catch(() => null),
  ])

  // 哪个查到了就更新，没查到的保留旧值
  const cur = fleshStatuses.value
  fleshStatuses.value = [
    {
      key: 'world.map',
      title: '地图',
      done: locations != null ? locations.length > 0 : cur[0].done,
    },
    {
      key: 'world.factions',
      title: '势力',
      done: factions != null ? factions.length > 0 : cur[1].done,
    },
    {
      key: 'world.items',
      title: '道具',
      done: items != null ? items.length > 0 : cur[2].done,
    },
    {
      key: 'characters.supporting',
      title: '配角',
      done: characters != null ? Math.max(0, characters.length - 1) > 0 : cur[3].done,
    },
  ]
}

// 触发条件已内联到 ConfirmAdvance 中（按血肉完成度 + store 状态自行判断）

const statusText = computed(() => {
  if (store.status === 'streaming') return '生成中'
  if (store.status === 'error') return '出错'
  return store.sessionId ? '就绪' : '未开始'
})
const statusDot = computed(() => {
  if (store.status === 'streaming') return 'busy'
  if (store.status === 'error') return 'err'
  return 'ok'
})
const canSend = computed(() => inputText.value.trim().length > 0 && store.status !== 'streaming')

function sessionTitle(s: AgentSession): string {
  if (s.title) return s.title
  const firstUser = s.messages.find((m) => m.role === 'user')
  if (firstUser) return firstUser.content.slice(0, 24)
  return '新会话'
}
function timeAgo(iso: string): string {
  const diff = (Date.now() - new Date(iso).getTime()) / 1000
  if (diff < 60) return '刚刚'
  if (diff < 3600) return `${Math.floor(diff / 60)} 分钟前`
  if (diff < 86400) return `${Math.floor(diff / 3600)} 小时前`
  return `${Math.floor(diff / 86400)} 天前`
}
function sessionMeta(s: AgentSession): string {
  return `${s.messages.length} 条 · ${timeAgo(s.updated_at)}`
}

function startRename(s: AgentSession) {
  editingId.value = s.id
  editingTitle.value = s.title || sessionTitle(s)
}
async function commitRename(id: string) {
  if (editingId.value !== id) return
  const t = editingTitle.value.trim()
  editingId.value = ''
  if (t) await store.renameSession(id, t)
}
async function onSelectSession(id: string) {
  await store.selectSession(id)
}

async function onDeleteSession(id: string) {
  await store.deleteSession(id, projectId.value)
}

async function onNewSession() {
  inputText.value = ''
  try {
    await store.newSession(projectId.value)
  } catch (e) {
    // 错误已在 store 记录
  }
}

async function send(text: string) {
  const t = text.trim()
  if (!t || store.status === 'streaming') return
  inputText.value = ''
  await store.sendMessage(t, projectId.value)
}

// 选择题选项 / 自由输入 → 作为下一轮用户消息继续对话
function onSelect(text: string) {
  send(text)
}

// 工具卡片「重试」：用 store.executeTool 真正执行，再把结果回灌对话让模型继续
async function onRetry(payload: { name: string; input: unknown }) {
  if (store.status === 'streaming') return
  try {
    const r = await store.executeTool(payload.name, payload.input, projectId.value)
    await send(`工具 ${payload.name} 已重试，结果：${JSON.stringify(r.result)}`)
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e)
    await send(`工具 ${payload.name} 重试失败：${msg}`)
  }
}

// 推进成功：刷新项目 + 会话，让 step indicator 反映新阶段
async function onAdvanced() {
  // 重拉项目（project.config.current_step 已变更）
  try {
    await projectStore.fetchProject(projectId.value)
  } catch {
    // ignore
  }
  if (store.sessionId) {
    try {
      await store.restoreSession(store.sessionId)
    } catch {
      // ignore
    }
  }
  await store.loadSessions(projectId.value)
  // 血肉完成度也要刷新（推进后可能新建了 entity）
  void refreshFleshStatus()
}

// 用户手动点 "排版" 按钮：把第 i 条消息标 formatted=true
function onFormat(index: number) {
  store.markFormatted(index)
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    send(inputText.value)
  }
}

function scrollToBottom() {
  nextTick(() => {
    const el = messagesEl.value
    if (el) el.scrollTop = el.scrollHeight
  })
}

watch(
  () => store.messages.map((m) => m.content).join('|') + ':' + store.messages.length,
  scrollToBottom,
)

// 进入某项目时加载其会话列表，并恢复/新建该项目的当前会话
async function activateProject(pid: string) {
  await store.loadSessions(pid)
  const persisted = store.readPersistedSession(pid)
  if (persisted && store.sessions.some((s) => s.id === persisted)) {
    await store.restoreSession(persisted)
  } else {
    try {
      await store.newSession(pid)
    } catch {
      // 后端不可用时由 error 状态展示
    }
  }
  // 拉血肉完成度（小选择器驱动）
  void refreshFleshStatus()
  scrollToBottom()
}

onMounted(async () => {
  await store.loadTools()
  await activateProject(projectId.value)
})

// 在项目间切换（/project/A/agent → /project/B/agent 复用同一组件实例）
watch(
  () => projectId.value,
  (pid) => {
    void activateProject(pid)
  },
)
</script>

<style scoped>
.agent-page { display: flex; height: 100%; width: 100%; overflow: hidden; }

/* ---------- 右侧历史 ---------- */
.agent-history {
  width: 280px;
  flex-shrink: 0;
  background: var(--bg-panel);
  border-left: 1px solid var(--border-default);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.history-head {
  display: flex; align-items: center; justify-content: space-between;
  padding: var(--space-3) var(--space-3);
  border-bottom: 1px solid var(--border-muted);
  flex-shrink: 0;
}
.history-title { font-size: var(--text-sm); font-weight: 600; color: var(--text-primary); }
.icon-btn {
  display: flex; align-items: center; justify-content: center;
  width: 28px; height: 28px; border: none; background: transparent;
  color: var(--text-tertiary); border-radius: var(--radius-sm); cursor: pointer;
  transition: all var(--transition-fast);
}
.icon-btn:hover { background: var(--bg-hover); color: var(--text-primary); }

.history-list { flex: 1; overflow-y: auto; padding: var(--space-2); display: flex; flex-direction: column; gap: var(--space-2); }
.history-empty { font-size: var(--text-sm); color: var(--text-tertiary); padding: var(--space-2); }
.history-item {
  position: relative;
  display: flex; align-items: center; justify-content: space-between; gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  background: var(--bg-panel-secondary);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all var(--transition-fast);
}
.history-item:hover { border-color: var(--border-emphasis); background: var(--bg-hover); }
.history-item.active { border-color: var(--border-primary); background: var(--color-primary-subtle); }
.hi-main { min-width: 0; flex: 1; }
.hi-title { font-size: var(--text-sm); color: var(--text-primary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.history-item.active .hi-title { color: var(--color-primary-text); }
.hi-meta { font-size: var(--text-xs); color: var(--text-tertiary); margin-top: 2px; }
.hi-actions { display: flex; gap: var(--space-1); flex-shrink: 0; }
.hi-btn {
  width: 24px; height: 24px; display: flex; align-items: center; justify-content: center;
  background: transparent; border: 1px solid var(--border-default); border-radius: var(--radius-sm);
  color: var(--text-tertiary); cursor: pointer; transition: all var(--transition-fast);
}
.hi-btn:hover { color: var(--text-primary); border-color: var(--border-emphasis); }
.hi-btn.danger:hover { color: var(--color-error); border-color: var(--color-error); }
.hi-rename {
  position: absolute; inset: 0;
  padding: 0 var(--space-3);
  background: var(--bg-base); border: 1px solid var(--color-primary); border-radius: var(--radius-md);
  color: var(--text-primary); font-size: var(--text-sm); font-family: inherit; outline: none;
}

/* ---------- 右侧对话 ---------- */
.agent-chat { flex: 1; display: flex; flex-direction: column; min-width: 0; background: var(--bg-base); }
.chat-header {
  display: flex; align-items: center; gap: var(--space-3);
  padding: var(--space-3) var(--space-5);
  border-bottom: 1px solid var(--border-default);
  background: var(--bg-panel);
  flex-shrink: 0;
}
.chat-id { display: flex; flex-direction: column; min-width: 0; }
.chat-title { font-size: var(--text-md); font-weight: 600; color: var(--text-primary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.chat-step { font-size: var(--text-xs); color: var(--color-primary-text); margin-top: 2px; }
.chat-status { display: flex; align-items: center; gap: var(--space-2); font-size: var(--text-xs); color: var(--text-tertiary); margin-left: var(--space-2); }
.chat-actions { margin-left: auto; display: flex; align-items: center; gap: var(--space-2); }

.ghost-btn, .primary-btn {
  display: inline-flex; align-items: center; gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  border-radius: var(--radius-md); font-size: var(--text-sm); font-family: inherit; cursor: pointer;
  transition: all var(--transition-fast);
}
.ghost-btn { background: var(--bg-panel-secondary); color: var(--text-secondary); border: 1px solid var(--border-default); }
.ghost-btn:hover { background: var(--bg-hover); color: var(--text-primary); border-color: var(--border-emphasis); }
.primary-btn { background: var(--color-primary); color: #fff; border: 1px solid var(--color-primary); }
.primary-btn:hover:not(:disabled) { background: var(--color-primary-hover); }
.primary-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.dot { width: 7px; height: 7px; border-radius: 50%; background: var(--text-disabled); }
.dot.ok { background: var(--color-success); }
.dot.busy { background: var(--color-warning); animation: pulse 1s infinite; }
.dot.err { background: var(--color-error); }

.messages { flex: 1; overflow-y: auto; padding: var(--space-6) var(--space-5); }
.msg-col { width: 100%; display: flex; flex-direction: column; gap: var(--space-4); }

.empty { margin: auto; text-align: center; max-width: 460px; padding: var(--space-10) var(--space-4); }
.empty-seal {
  width: 56px; height: 56px; margin: 0 auto var(--space-4);
  display: flex; align-items: center; justify-content: center;
  background: var(--color-primary-subtle); color: var(--color-primary-text);
  border: 1px solid var(--border-primary);
  border-radius: var(--radius-lg);
  font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif);
}
.empty-title { font-size: var(--text-lg); font-weight: 600; color: var(--text-primary); margin-bottom: var(--space-2); }
.empty-sub { font-size: var(--text-sm); color: var(--text-secondary); line-height: var(--leading-relaxed); margin-bottom: var(--space-5); }
.chips { display: flex; flex-wrap: wrap; gap: var(--space-2); justify-content: center; }
.chip {
  display: inline-flex; align-items: center; gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  background: var(--bg-panel-secondary);
  border: 1px solid var(--border-default);
  border-radius: 999px;
  color: var(--text-secondary);
  font-size: var(--text-xs); font-family: inherit; cursor: pointer;
  transition: all var(--transition-fast);
  max-width: 320px;
}
.chip:hover { border-color: var(--border-primary); color: var(--text-primary); background: var(--bg-hover); }
.chip span { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

.error-bar {
  display: flex; align-items: center; gap: var(--space-2);
  margin: 0 var(--space-5);
  padding: var(--space-2) var(--space-3);
  background: var(--color-error-subtle);
  border: 1px solid var(--color-error);
  border-radius: var(--radius-sm);
  color: var(--color-error);
  font-size: var(--text-xs);
}

.composer {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  padding: var(--space-4) var(--space-5);
  border-top: 1px solid var(--border-default);
  background: var(--bg-panel);
}

/* 工具栏：左推进 / 右发送，对称 */
.composer-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
}
.composer-advance { display: inline-flex; }

/* Thinking 指示器（LLM 还没返回第一 token 时） */
.thinking-bubble {
  display: flex;
  gap: var(--space-3);
  align-items: center;
  padding: 8px 0;
  opacity: 0.7;
}
.thinking-avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: var(--color-primary-subtle);
  color: var(--color-primary-text);
  border: 1px solid var(--border-primary);
  display: flex;
  align-items: center;
  justify-content: center;
  font-family: var(--font-serif);
  font-weight: 700;
  font-size: 13px;
  flex-shrink: 0;
}
.thinking-content {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  color: var(--text-tertiary);
  font-style: italic;
}
.thinking-dots { display: inline-flex; gap: 2px; }
.thinking-dots span {
  width: 4px; height: 4px;
  border-radius: 50%;
  background: var(--text-tertiary);
  animation: thinking-bounce 1.2s ease-in-out infinite;
}
.thinking-dots span:nth-child(2) { animation-delay: 0.15s; }
.thinking-dots span:nth-child(3) { animation-delay: 0.3s; }
@keyframes thinking-bounce {
  0%, 60%, 100% { transform: translateY(0); opacity: 0.4; }
  30% { transform: translateY(-4px); opacity: 1; }
}
.composer-input {
  width: 100%;
  padding: var(--space-3);
  background: var(--bg-base); border: 1px solid var(--border-default);
  border-radius: var(--radius-md); color: var(--text-primary);
  font-size: var(--text-sm); font-family: inherit; outline: none; resize: none;
  transition: border-color var(--transition-fast); line-height: var(--leading-normal);
  box-sizing: border-box;
}
.composer-input:focus { border-color: var(--color-primary); }
.send-btn {
  display: inline-flex; align-items: center; gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  background: var(--bg-panel-secondary); border: 1px solid var(--border-default);
  border-radius: var(--radius-md); color: var(--text-disabled);
  font-size: var(--text-sm); font-family: inherit; cursor: not-allowed; transition: all var(--transition-fast);
}
.send-btn.primary { background: var(--color-primary); border-color: var(--color-primary); color: #fff; cursor: pointer; }
.send-btn.primary:hover { background: var(--color-primary-hover); }

@keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.35; } }
</style>
