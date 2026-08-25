<template>
  <div class="msg" :class="role">
    <div class="avatar" :class="role">
      <span v-if="role === 'assistant'" class="serif">导</span>
      <span v-else-if="role === 'user'">我</span>
      <Wrench v-else :size="15" />
    </div>

    <!-- 工具卡片 -->
    <div v-if="tool" class="bubble tool-card">
      <div class="tool-head">
        <span class="tool-name">{{ tool.name || '(解析失败)' }}</span>
        <span class="tool-badge" :class="tool.ok ? 'ok' : 'fail'">{{ tool.ok ? '成功' : '失败' }}</span>
      </div>
      <div class="tool-section" v-if="hasInput">
        <div class="tool-label">入参</div>
        <pre class="tool-json">{{ pretty(tool.input) }}</pre>
      </div>
      <div class="tool-section">
        <div class="tool-label">结果</div>
        <pre class="tool-json">{{ pretty(tool.output) }}</pre>
      </div>
      <button v-if="!tool.ok" class="tool-retry" @click="retry">重试</button>
    </div>

    <!-- 选择题卡片 -->
    <div v-else-if="q" class="bubble question-card">
      <div class="role-label">请选择</div>
      <div class="q-text">{{ q.question }}</div>
      <div class="options">
        <button
          v-for="(opt, i) in q.options"
          :key="i"
          class="option"
          :disabled="answered"
          @click="choose(opt)"
        >
          {{ opt }}
        </button>
      </div>
      <div class="manual">
        <input
          class="manual-input"
          v-model="manualText"
          :disabled="answered"
          placeholder="以上都不合适？在这里输入你的回答…"
          @keydown.enter="submitManual"
        />
        <button class="manual-btn" :disabled="answered || !manualText.trim()" @click="submitManual">
          发送
        </button>
      </div>
      <div v-if="answered" class="answered">已回答：{{ chosen }}</div>
    </div>

    <!-- 普通文本气泡（Markdown 渲染） -->
    <div v-else class="bubble" :class="{ streaming }">
      <div class="content" v-html="rendered"></div>
      <span v-if="!content && streaming" class="thinking">正在思考…</span>
      <span v-if="streaming && content" class="caret"></span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { Wrench } from 'lucide-vue-next'
import { renderMarkdown, prettyJson } from '@/utils/markdown'

const props = defineProps<{
  role: 'user' | 'assistant' | 'tool'
  content: string
  streaming?: boolean
}>()

const emit = defineEmits<{
  select: [text: string]
  retry: [payload: { name: string; input: unknown }]
}>()

const ASK = '<<ASK_QUESTION>>'
const END = '<<END>>'
const TOOL = '<<TOOL_RESULT>>'

const isQuestion = computed(() => props.content.includes(ASK))
const q = computed(() => {
  if (!isQuestion.value) return null
  const start = props.content.indexOf(ASK) + ASK.length
  const end = props.content.indexOf(END, start)
  const json = end === -1 ? props.content.slice(start) : props.content.slice(start, end)
  try {
    const v = JSON.parse(json.trim())
    return {
      question: String(v.question || ''),
      options: Array.isArray(v.options) ? v.options.map((x: unknown) => String(x)) : [],
    }
  } catch {
    return null
  }
})

const tool = computed(() => {
  if (props.role !== 'tool') return null
  const start = props.content.indexOf(TOOL) + TOOL.length
  if (start < TOOL.length) return null
  const end = props.content.indexOf(END, start)
  const json = end === -1 ? props.content.slice(start) : props.content.slice(start, end)
  try {
    const v = JSON.parse(json.trim())
    return {
      name: String(v.name || ''),
      input: v.input,
      ok: !!v.ok,
      output: String(v.output || ''),
    }
  } catch {
    return null
  }
})

const hasInput = computed(
  () =>
    tool.value != null &&
    tool.value.input != null &&
    typeof tool.value.input === 'object' &&
    Object.keys(tool.value.input as object).length > 0,
)

const rendered = computed(() => renderMarkdown(props.content))
const pretty = (v: unknown) => prettyJson(v)

const manualText = ref('')
const answered = ref(false)
const chosen = ref('')

function choose(opt: string) {
  if (answered.value) return
  chosen.value = opt
  answered.value = true
  emit('select', opt)
}

function submitManual() {
  const t = manualText.value.trim()
  if (!t || answered.value) return
  chosen.value = t
  answered.value = true
  emit('select', t)
}

function retry() {
  if (tool.value) emit('retry', { name: tool.value.name, input: tool.value.input })
}
</script>

<style scoped>
.msg {
  display: flex;
  gap: var(--space-3);
  max-width: 86%;
  animation: fade-in 240ms ease;
}
.msg.user { margin-left: auto; flex-direction: row-reverse; }
.msg.assistant { margin-right: auto; }

.avatar {
  flex-shrink: 0;
  width: 32px; height: 32px;
  border-radius: var(--radius-md);
  display: flex; align-items: center; justify-content: center;
  font-size: var(--text-sm); font-weight: 600;
  user-select: none;
}
.avatar.assistant {
  background: var(--color-primary-subtle);
  color: var(--color-primary-text);
  border: 1px solid var(--border-primary);
}
.avatar.assistant .serif { font-family: var(--font-serif); font-size: var(--text-md); }
.avatar.user {
  background: var(--bg-active);
  color: var(--text-secondary);
  border: 1px solid var(--border-default);
}
.avatar.tool {
  background: var(--bg-active);
  color: var(--color-accent);
  border: 1px solid var(--border-default);
}

.bubble {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  padding: var(--space-3) var(--space-4);
  border-radius: var(--radius-lg);
  border: 1px solid var(--border-default);
  min-width: 80px;
}
.msg.assistant .bubble {
  background: var(--bg-panel-secondary);
  border-top-left-radius: var(--radius-sm);
}
.msg.user .bubble {
  background: var(--color-primary-subtle);
  border-color: var(--border-primary);
  border-top-right-radius: var(--radius-sm);
}

/* 工具卡片 */
.tool-card {
  width: 100%;
  background: var(--bg-panel-secondary);
  border-color: var(--border-primary);
  border-top-left-radius: var(--radius-sm);
  gap: var(--space-2);
}
.tool-head { display: flex; align-items: center; gap: var(--space-2); }
.tool-name {
  font-size: var(--text-sm); font-weight: 600;
  color: var(--text-primary);
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  word-break: break-all;
}
.tool-badge { font-size: var(--text-xs); padding: 1px 8px; border-radius: 999px; flex-shrink: 0; }
.tool-badge.ok { background: rgba(63, 185, 80, 0.15); color: #3fb950; }
.tool-badge.fail { background: rgba(248, 81, 73, 0.15); color: #f85149; }
.tool-section { display: flex; flex-direction: column; gap: 4px; }
.tool-label { font-size: var(--text-xs); color: var(--text-tertiary); letter-spacing: 0.04em; }
.tool-json {
  margin: 0;
  background: var(--bg-base);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  padding: var(--space-2) var(--space-3);
  font-size: var(--text-xs); line-height: 1.5;
  color: var(--text-secondary);
  overflow-x: auto;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  white-space: pre-wrap; word-break: break-word;
}
.tool-retry {
  align-self: flex-start; margin-top: var(--space-1);
  padding: var(--space-1) var(--space-3);
  background: var(--color-primary); border: 1px solid var(--color-primary);
  border-radius: var(--radius-md); color: #fff; font-size: var(--text-xs); cursor: pointer;
}
.tool-retry:hover { background: var(--color-primary-hover); }

/* 选择题卡片 */
.question-card {
  width: 100%;
  background: var(--bg-panel-secondary);
  border-color: var(--border-primary);
  border-top-left-radius: var(--radius-sm);
  gap: var(--space-3);
}
.q-text {
  font-size: var(--text-md);
  line-height: var(--leading-relaxed);
  color: var(--text-primary);
}
.options {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-2);
}
.option {
  text-align: left;
  padding: var(--space-2) var(--space-3);
  background: var(--bg-base);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  color: var(--text-secondary);
  font-size: var(--text-sm); font-family: inherit;
  cursor: pointer;
  transition: all var(--transition-fast);
  line-height: var(--leading-normal);
}
.option:hover:not(:disabled) {
  border-color: var(--color-primary);
  color: var(--text-primary);
  background: var(--bg-hover);
}
.option:disabled { opacity: 0.55; cursor: default; }

.manual { display: flex; gap: var(--space-2); margin-top: var(--space-1); }
.manual-input {
  flex: 1;
  padding: var(--space-2) var(--space-3);
  background: var(--bg-base);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  font-size: var(--text-sm); font-family: inherit;
  outline: none;
}
.manual-input:focus { border-color: var(--color-primary); }
.manual-input:disabled { opacity: 0.6; }
.manual-btn {
  padding: var(--space-2) var(--space-4);
  background: var(--color-primary);
  border: 1px solid var(--color-primary);
  border-radius: var(--radius-md);
  color: #fff; font-size: var(--text-sm); font-family: inherit; cursor: pointer;
  transition: background var(--transition-fast);
}
.manual-btn:hover:not(:disabled) { background: var(--color-primary-hover); }
.manual-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.answered { font-size: var(--text-xs); color: var(--color-primary-text); }

.role-label {
  font-size: var(--text-xs);
  color: var(--text-tertiary);
  letter-spacing: 0.04em;
}

.content {
  font-size: var(--text-md);
  line-height: var(--leading-relaxed);
  color: var(--text-primary);
  white-space: normal;
  word-break: break-word;
}
/* Markdown 子元素（v-html 注入，需用 :deep 穿透 scoped） */
.content :deep(h1),
.content :deep(h2),
.content :deep(h3) { margin: 0.4em 0 0.3em; line-height: 1.3; }
.content :deep(h1) { font-size: 1.25em; }
.content :deep(h2) { font-size: 1.12em; }
.content :deep(h3) { font-size: 1.02em; }
.content :deep(p) { margin: 0.5em 0; }
.content :deep(p:first-child) { margin-top: 0; }
.content :deep(p:last-child) { margin-bottom: 0; }
.content :deep(ul),
.content :deep(ol) { margin: 0.5em 0; padding-left: 1.4em; }
.content :deep(li) { margin: 0.2em 0; }
.content :deep(a) { color: var(--color-accent); text-decoration: underline; }
.content :deep(strong) { color: var(--text-primary); font-weight: 600; }
.content :deep(blockquote) {
  margin: 0.5em 0; padding-left: 0.8em;
  border-left: 3px solid var(--border-default); color: var(--text-secondary);
}
.content :deep(code) {
  background: var(--bg-base); padding: 0.1em 0.35em;
  border-radius: var(--radius-sm); font-size: 0.9em;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.content :deep(pre) {
  background: var(--bg-base); border: 1px solid var(--border-default);
  border-radius: var(--radius-md); padding: var(--space-3);
  overflow-x: auto; margin: 0.5em 0;
}
.content :deep(pre code) { background: none; padding: 0; font-size: 0.85em; }
.content :deep(table) { border-collapse: collapse; margin: 0.5em 0; width: 100%; }
.content :deep(th),
.content :deep(td) { border: 1px solid var(--border-default); padding: 0.3em 0.6em; text-align: left; }
.content :deep(hr) { border: none; border-top: 1px solid var(--border-default); margin: 0.7em 0; }

.thinking { color: var(--text-tertiary); font-style: italic; }
.caret {
  display: inline-block;
  width: 7px; height: 1.1em;
  margin-left: 1px;
  background: var(--color-primary);
  vertical-align: text-bottom;
  animation: blink 1s step-end infinite;
}

@keyframes blink { 50% { opacity: 0; } }
@keyframes fade-in { from { opacity: 0; transform: translateY(4px); } to { opacity: 1; transform: none; } }
</style>
