<template>
  <div class="agent-page">
    <!-- 左侧：会话与工具面板 -->
    <aside class="side">
      <div class="side-header">
        <div class="side-title">
          <span class="seal serif">导</span>
          <div>
            <div class="title-main">创作引导</div>
            <div class="title-sub">Agent · 引导式小说创作</div>
          </div>
        </div>
        <button class="new-btn" @click="onNewSession" :disabled="store.status === 'streaming'">
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none"><path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>
          新建会话
        </button>
        <button class="prompt-btn" @click="showPrompt = true">
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none"><path d="M3 4h10M3 8h7M3 12h10" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/></svg>
          提示词
        </button>
      </div>

      <div class="history">
        <div class="section-label">历史会话</div>
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
            <button class="hi-btn" title="重命名" @click="startRename(s)">✎</button>
            <button class="hi-btn danger" title="删除" @click="onDeleteSession(s.id)">×</button>
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

      <div class="tools">
        <div class="section-label">可用工具</div>
        <div v-if="store.tools.length === 0" class="tools-empty">暂无可用工具</div>
        <div v-for="t in store.tools" :key="t.name" class="tool-card">
          <div class="tool-name mono">{{ t.name }}</div>
          <div class="tool-desc">{{ t.description }}</div>
        </div>
      </div>

      <div class="side-foot">
        Phase 1 · 会话与记忆已持久化到 Postgres，刷新或切换页面不丢失
      </div>
    </aside>

    <!-- 右侧：对话 -->
    <section class="chat">
      <header class="chat-header">
        <div class="chat-title">与创作向导对话</div>
        <div class="chat-status">
          <span class="dot" :class="statusDot"></span>{{ statusText }}
        </div>
        <div class="chat-sub" v-if="store.thinking">
          <span class="pulse"></span> 向导正在思考…
        </div>
      </header>

      <div class="messages" ref="messagesEl">
        <div v-if="store.messages.length === 0" class="empty">
          <div class="empty-seal serif">笔</div>
          <div class="empty-title">开始你的创作之旅</div>
          <div class="empty-sub">用自然语言描述你的想法，向导会逐步帮你构建世界观、角色与剧情。</div>
          <div class="chips">
            <button v-for="s in suggestions" :key="s" class="chip" @click="send(s)">{{ s }}</button>
          </div>
        </div>

        <ChatMessage
          v-for="(m, i) in store.messages"
          :key="i"
          :role="m.role"
          :content="m.content"
          :streaming="m.streaming"
          @select="onSelect"
        />
      </div>

      <div v-if="store.error" class="error-bar">⚠ {{ store.error }}</div>

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
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M2 8L14 2L9 14L7 8L2 8Z" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round" fill="currentColor"/></svg>
          发送
        </button>
      </div>
    </section>

    <PromptEditor v-model="showPrompt" />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, nextTick, watch } from 'vue'
import { useAgentStore } from '@/stores/agent'
import type { AgentSession } from '@/api/agent'
import ChatMessage from '@/components/agent/ChatMessage.vue'
import PromptEditor from '@/components/agent/PromptEditor.vue'

const store = useAgentStore()
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
  await store.deleteSession(id)
}

async function onNewSession() {
  inputText.value = ''
  try {
    await store.newSession()
  } catch (e) {
    // 错误已在 store 记录
  }
}

async function send(text: string) {
  const t = text.trim()
  if (!t || store.status === 'streaming') return
  inputText.value = ''
  await store.sendMessage(t)
}

// 选择题选项 / 自由输入 → 作为下一轮用户消息继续对话
function onSelect(text: string) {
  send(text)
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

onMounted(async () => {
  await store.loadTools()
  await store.loadSessions()
  const persisted = store.readPersistedSession()
  if (persisted && store.sessions.some((s) => s.id === persisted)) {
    await store.restoreSession(persisted)
  } else {
    try {
      await store.newSession()
    } catch {
      // 后端不可用时由 error 状态展示
    }
  }
  scrollToBottom()
})
</script>

<style scoped>
.agent-page { display: flex; height: 100%; width: 100%; overflow: hidden; }

/* ---------- 左侧面板 ---------- */
.side {
  width: 300px;
  flex-shrink: 0;
  background: var(--bg-panel);
  border-right: 1px solid var(--border-default);
  display: flex;
  flex-direction: column;
  padding: var(--space-4);
  gap: var(--space-4);
  overflow: hidden;
}
.side-header { display: flex; flex-direction: column; gap: var(--space-3); }
.side-title { display: flex; align-items: center; gap: var(--space-3); }
.seal {
  width: 36px; height: 36px;
  display: flex; align-items: center; justify-content: center;
  background: var(--color-primary); color: #fff;
  border-radius: var(--radius-md);
  font-size: var(--text-lg); font-weight: 700;
}
.title-main { font-size: var(--text-md); font-weight: 600; color: var(--text-primary); }
.title-sub { font-size: var(--text-xs); color: var(--text-tertiary); margin-top: 2px; }

.new-btn {
  display: flex; align-items: center; justify-content: center; gap: var(--space-2);
  padding: var(--space-2);
  background: var(--color-primary); color: #fff;
  border: 1px solid var(--color-primary);
  border-radius: var(--radius-sm);
  font-size: var(--text-sm); font-family: inherit; cursor: pointer;
  transition: background var(--transition-fast);
}
.new-btn:hover:not(:disabled) { background: var(--color-primary-hover); }
.new-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.prompt-btn {
  display: flex; align-items: center; justify-content: center; gap: var(--space-2);
  padding: var(--space-2);
  background: var(--bg-panel-secondary); color: var(--text-secondary);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm);
  font-size: var(--text-sm); font-family: inherit; cursor: pointer;
  transition: all var(--transition-fast);
}
.prompt-btn:hover { background: var(--bg-hover); color: var(--text-primary); border-color: var(--border-emphasis); }

.session-box {
  background: var(--bg-panel-secondary);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  padding: var(--space-3);
  display: flex; flex-direction: column; gap: var(--space-2);
}
.kv { display: flex; align-items: center; justify-content: space-between; font-size: var(--text-xs); }
.k { color: var(--text-tertiary); }
.v { color: var(--text-secondary); display: flex; align-items: center; gap: var(--space-2); }
.mono { font-family: var(--font-mono); }
.dot { width: 7px; height: 7px; border-radius: 50%; background: var(--text-disabled); }
.dot.ok { background: var(--color-success); }
.dot.busy { background: var(--color-warning); animation: pulse 1s infinite; }
.dot.err { background: var(--color-error); }

.tools { display: flex; flex-direction: column; gap: var(--space-2); flex: 0 0 auto; max-height: 38%; overflow-y: auto; }
.section-label {
  font-size: var(--text-xs); font-weight: 600; letter-spacing: 0.06em;
  color: var(--text-tertiary); text-transform: uppercase;
}
.tools-empty { font-size: var(--text-sm); color: var(--text-tertiary); }
.tool-card {
  background: var(--bg-panel-secondary);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  padding: var(--space-2) var(--space-3);
}
.tool-name { color: var(--color-accent); font-size: var(--text-xs); margin-bottom: 2px; }
.tool-desc { color: var(--text-secondary); font-size: var(--text-xs); line-height: var(--leading-normal); }

.side-foot {
  font-size: var(--text-xs); color: var(--text-tertiary);
  border-top: 1px solid var(--border-muted); padding-top: var(--space-3);
}

/* ---------- 历史会话列表 ---------- */
.history { display: flex; flex-direction: column; gap: var(--space-2); flex: 1 1 auto; min-height: 0; overflow-y: auto; }
.history-empty { font-size: var(--text-sm); color: var(--text-tertiary); padding: var(--space-2) 0; }
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
.hi-title {
  font-size: var(--text-sm); color: var(--text-primary);
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
.history-item.active .hi-title { color: var(--color-primary-text); }
.hi-meta { font-size: var(--text-xs); color: var(--text-tertiary); margin-top: 2px; }
.hi-actions { display: flex; gap: var(--space-1); flex-shrink: 0; }
.hi-btn {
  width: 24px; height: 24px; display: flex; align-items: center; justify-content: center;
  background: transparent; border: 1px solid var(--border-default); border-radius: var(--radius-sm);
  color: var(--text-tertiary); font-size: var(--text-sm); cursor: pointer; font-family: inherit;
  transition: all var(--transition-fast);
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
.chat { flex: 1; display: flex; flex-direction: column; min-width: 0; background: var(--bg-base); }
.chat-header {
  display: flex; align-items: baseline; gap: var(--space-3);
  padding: var(--space-3) var(--space-5);
  border-bottom: 1px solid var(--border-default);
  background: var(--bg-panel);
}
.chat-title { font-size: var(--text-md); font-weight: 600; color: var(--text-primary); }
.chat-status { display: flex; align-items: center; gap: var(--space-2); font-size: var(--text-xs); color: var(--text-tertiary); }
.chat-sub { font-size: var(--text-xs); color: var(--color-warning); display: flex; align-items: center; gap: var(--space-2); }
.pulse { width: 7px; height: 7px; border-radius: 50%; background: var(--color-warning); animation: pulse 1s infinite; }

.messages {
  flex: 1; overflow-y: auto;
  padding: var(--space-5);
  display: flex; flex-direction: column; gap: var(--space-4);
}

.empty { margin: auto; text-align: center; max-width: 460px; padding: var(--space-8) var(--space-4); }
.empty-seal {
  width: 56px; height: 56px; margin: 0 auto var(--space-4);
  display: flex; align-items: center; justify-content: center;
  background: var(--color-primary-subtle); color: var(--color-primary-text);
  border: 1px solid var(--border-primary);
  border-radius: var(--radius-lg);
  font-size: var(--text-2xl); font-weight: 700;
}
.empty-title { font-size: var(--text-lg); font-weight: 600; color: var(--text-primary); margin-bottom: var(--space-2); }
.empty-sub { font-size: var(--text-sm); color: var(--text-secondary); line-height: var(--leading-relaxed); margin-bottom: var(--space-5); }
.chips { display: flex; flex-wrap: wrap; gap: var(--space-2); justify-content: center; }
.chip {
  padding: var(--space-2) var(--space-3);
  background: var(--bg-panel-secondary);
  border: 1px solid var(--border-default);
  border-radius: 999px;
  color: var(--text-secondary);
  font-size: var(--text-xs); font-family: inherit; cursor: pointer;
  transition: all var(--transition-fast);
}
.chip:hover { border-color: var(--border-primary); color: var(--text-primary); background: var(--bg-hover); }

.error-bar {
  margin: 0 var(--space-5);
  padding: var(--space-2) var(--space-3);
  background: var(--color-error-subtle);
  border: 1px solid var(--color-error);
  border-radius: var(--radius-sm);
  color: var(--color-error);
  font-size: var(--text-xs);
}

.composer {
  display: flex; gap: var(--space-3);
  padding: var(--space-4) var(--space-5);
  border-top: 1px solid var(--border-default);
  background: var(--bg-panel);
  align-items: flex-end;
}
.composer-input {
  flex: 1;
  padding: var(--space-3);
  background: var(--bg-base);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  font-size: var(--text-sm); font-family: inherit;
  outline: none; resize: none;
  transition: border-color var(--transition-fast);
  line-height: var(--leading-normal);
}
.composer-input:focus { border-color: var(--color-primary); }
.send-btn {
  display: inline-flex; align-items: center; gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  background: var(--bg-panel-secondary);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  color: var(--text-disabled);
  font-size: var(--text-sm); font-family: inherit; cursor: not-allowed;
  transition: all var(--transition-fast);
}
.send-btn.primary {
  background: var(--color-primary); border-color: var(--color-primary); color: #fff; cursor: pointer;
}
.send-btn.primary:hover { background: var(--color-primary-hover); }

@keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.35; } }
</style>
