<template>
  <div class="agent-page">
    <!-- 中间：对话主区 -->
    <section class="agent-chat">
      <header class="chat-header">
        <div class="chat-id">
          <div class="chat-title">{{ projectName || '创作引导' }}</div>
          <div class="chat-step" v-if="currentStep">{{ currentStep }}</div>
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
            @select="onSelect"
            @retry="onRetry"
          />
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
        <button
          class="send-btn"
          :class="{ primary: canSend }"
          :disabled="!canSend"
          @click="send(inputText)"
        >
          <Send :size="16" /><span>发送</span>
        </button>
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
import ChatMessage from '@/components/agent/ChatMessage.vue'
import PromptEditor from '@/components/agent/PromptEditor.vue'

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

.composer { display: flex; gap: var(--space-3); padding: var(--space-4) var(--space-5); border-top: 1px solid var(--border-default); background: var(--bg-panel); align-items: flex-end; }
.composer-input {
  flex: 1; padding: var(--space-3);
  background: var(--bg-base); border: 1px solid var(--border-default);
  border-radius: var(--radius-md); color: var(--text-primary);
  font-size: var(--text-sm); font-family: inherit; outline: none; resize: none;
  transition: border-color var(--transition-fast); line-height: var(--leading-normal);
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
