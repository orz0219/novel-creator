<template>
  <div class="msg" :class="role">
    <div class="avatar" :class="role">
      <span v-if="role === 'assistant'" class="serif">导</span>
      <span v-else-if="role === 'user'">我</span>
      <Wrench v-else :size="15" />
    </div>

    <!-- 气泡 + 其下方工具条的纵向容器（工具条右对齐到气泡右缘） -->
    <div class="msg-body">
    <!-- 工具卡片 -->
    <!-- 工具卡片：折叠态只占一行（说清"AI 做了什么"），点击展开细节 -->
    <div v-if="tool" class="bubble tool-card" :class="{ expanded }">
      <button class="tool-head" type="button" @click="expanded = !expanded">
        <Wrench :size="13" class="tool-icon" />
        <span class="tool-action">{{ toolActionText }}</span>
        <span v-if="toolSubjectText" class="tool-subject">「{{ toolSubjectText }}」</span>
        <span class="tool-badge" :class="tool.ok ? 'ok' : 'fail'">
          {{ tool.ok ? '完成' : '失败' }}
        </span>
        <ChevronDown :size="14" class="tool-chevron" />
      </button>

      <div v-if="expanded" class="tool-body">
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

    <!-- 普通文本气泡
         - formatted=true（默认，历史消息 / 流式完成后）：v-html 渲染 markdown
         - formatted=false（流式中）：v-text 纯文本，避免 marked 解析不完整 markdown
         简化版：不再看 streaming 字段（LLM 一口气吐完时切换不流畅），改用 formatted
    -->
    <div v-else class="bubble" :class="{ streaming, 'is-raw': !formatted }">
      <div v-if="formatted" ref="contentEl" class="content markdown-body" v-html="rendered"></div>
      <div v-else ref="contentEl" class="content markdown-body content-raw">{{ sanitizedContent }}<span v-if="streaming" class="caret"></span></div>
      <span v-if="!sanitizedContent && streaming" class="thinking">正在思考…</span>
    </div>

      <!--
        消息工具条：位于气泡正下方、右对齐到气泡右缘，鼠标悬停整条消息时出现。
        设计成"工具条"而不是单个按钮，是为了后续可平行扩展（复制、重试、引用…）。
        - 用户消息：删除（截断）
        - AI 普通文本消息：复制
      -->
      <div v-if="index !== undefined || showCopy" class="msg-toolbar">
        <button
          v-if="showCopy"
          class="msg-action copy"
          :class="{ copied }"
          type="button"
          :title="copied ? '已复制' : '复制这条消息'"
          @click="copyContent"
        >
          <Check v-if="copied" :size="13" />
          <Copy v-else :size="13" />
        </button>

        <button
          v-if="index !== undefined"
          class="msg-action"
          type="button"
          title="删除这条及其之后的全部内容"
          @click="emit('truncate', index as number)"
        >
          <Trash2 :size="13" />
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from 'vue'
import { Wrench, ChevronDown, Trash2, Copy, Check } from 'lucide-vue-next'
import { renderMarkdown, prettyJson } from '@/utils/markdown'
import {
  sanitizeAssistantText,
  toolAction,
  toolSubject,
} from '@/utils/agentDisplay'

const props = defineProps<{
  role: 'user' | 'assistant' | 'tool'
  content: string
  streaming?: boolean
  /** 消息是否已"格式化"（流式完成后由 store 置 true / 用户手动点 "排版" 也置 true） */
  formatted?: boolean
  /**
   * 该消息在会话中的序号；传入才会显示下方工具条。
   *
   * 目前只对**用户消息**传入 —— 删除是「回滚到这条之前」的语义：
   * 用户删掉自己发错的那句，连带清掉由它引发的 AI 回复与工具记录；
   * AI 自己的回复没有单独删除的意义（想重来直接删提问那条即可）。
   */
  index?: number
}>()

const emit = defineEmits<{
  select: [text: string]
  retry: [payload: { name: string; input: unknown }]
  /** 删除该条消息及其之后的全部内容（截断） */
  truncate: [index: number]
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

/** 内部协议标记不外泄：渲染前统一清理（流式残留 / 未执行的调用文本）。 */
const sanitizedContent = computed(() => sanitizeAssistantText(props.content))
const rendered = computed(() => renderMarkdown(sanitizedContent.value))
const pretty = (v: unknown) => prettyJson(v)

// ---------- 复制 AI 消息 ----------
/**
 * 只有「AI 的普通文本消息」才显示复制按钮：
 * 工具卡片、选择题本身是内部协议文本，复制出来没有意义。
 */
const showCopy = computed(
  () => props.role === 'assistant' && !q.value && !tool.value && !props.streaming,
)

const copied = ref(false)
/** 普通文本气泡的 DOM 引用：复制时取「渲染后的纯文本」，避免带出 markdown 源符号。 */
const contentEl = ref<HTMLElement | null>(null)
let copyTimer: ReturnType<typeof setTimeout> | undefined

function markCopied() {
  copied.value = true
  clearTimeout(copyTimer)
  copyTimer = setTimeout(() => {
    copied.value = false
  }, 1500)
}

async function copyContent() {
  // 用户看到什么就复制什么；DOM 不可用时回退到清理过协议标记的原文。
  const text = (contentEl.value?.innerText ?? sanitizedContent.value).trim()
  if (!text) return
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text)
      markCopied()
      return
    }
  } catch {
    // 非安全上下文 / 无权限时走下面的降级方案
  }
  fallbackCopy(text)
}

/** 降级复制：execCommand 在旧浏览器或非 HTTPS 环境下仍可使用。 */
function fallbackCopy(text: string) {
  const el = document.createElement('textarea')
  el.value = text
  el.style.position = 'fixed'
  el.style.top = '-9999px'
  el.style.opacity = '0'
  document.body.appendChild(el)
  el.select()
  try {
    if (document.execCommand('copy')) markCopied()
  } catch {
    // 复制失败不阻断阅读，静默处理
  } finally {
    document.body.removeChild(el)
  }
}

onBeforeUnmount(() => clearTimeout(copyTimer))

// ---------- 工具卡片（折叠态一行） ----------
const expanded = ref(false)
const toolActionText = computed(() => toolAction(tool.value?.name ?? ''))
const toolSubjectText = computed(() => toolSubject(tool.value?.input))

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
  /*
   * 单行气泡的最小高度 —— 也是头像尺寸的基准（见 .avatar）。
   * 由「内容行高(14px × 1.7 ≈ 24px) + 上下内边距(6px × 2) + 边框(2px)」得出，
   * 正好等于头像，单行时气泡与头像齐平，不会显得"头小框大"。
   * 定义在组件根元素上供子元素继承（scoped 样式里 :root 不生效）。
   */
  --bubble-min-h: 38px;
  position: relative;
  display: flex;
  gap: var(--space-3);
  max-width: 86%;
  animation: fade-in 240ms ease;
}
.msg.user { margin-left: auto; flex-direction: row-reverse; }
.msg.assistant { margin-right: auto; }

/*
 * 气泡 + 工具条的纵向容器。
 * 工具条用绝对定位挂在气泡下方（而不是作为普通流式项），这样它不会占位：
 * 未悬停时无空白、悬停时也不会把下方消息推开。
 */
.msg-body {
  position: relative;
  display: flex;
  flex-direction: column;
  flex: 0 1 auto;
  min-width: 0;
}

/*
 * 消息工具条：贴气泡底部、右对齐；默认隐形，悬停整条消息时出现。
 * 之所以做成"条"而非单按钮，是为了后续可扩展更多操作（复制 / 重试 / 引用…）。
 */
.msg-toolbar {
  position: absolute;
  top: 100%;
  right: 0;
  display: flex;
  align-items: center;
  gap: var(--space-1);
  padding-top: 1px;
  opacity: 0;
  transition: opacity var(--transition-fast);
}
.msg:hover .msg-toolbar { opacity: 1; }

.msg-action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 2px;
  background: transparent;
  border: none;
  border-radius: var(--radius-sm);
  color: var(--text-tertiary);
  cursor: pointer;
  transition: color var(--transition-fast), background var(--transition-fast);
}
/* 删除是危险操作：悬停变红 */
.msg-action:hover { color: var(--color-error); background: var(--bg-hover); }
/* 复制不是危险操作：悬停用主题色；复制成功后图标变对勾并短暂变绿作为反馈 */
.msg-action.copy:hover { color: var(--color-primary); background: var(--bg-hover); }
.msg-action.copy:active { color: var(--color-primary-active); }
.msg-action.copy.copied { color: var(--color-success); }

/*
 * 头像尺寸与单行气泡高度（--bubble-min-h）保持一致：
 * 两者不齐平时，会出现"小头像配大气泡"的别扭感。
 */
.avatar {
  flex-shrink: 0;
  width: var(--bubble-min-h);
  height: var(--bubble-min-h);
  border-radius: var(--radius-md);
  display: flex; align-items: center; justify-content: center;
  font-size: var(--text-md); font-weight: 600;
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
  /* 内容垂直居中：单行消息与 32px 头像对齐时不会一头沉 */
  justify-content: center;
  gap: var(--space-1);
  /* 上下内边距 6px：与内容行高、边框合计正好等于 --bubble-min-h（38px） */
  padding: 6px var(--space-4);
  border-radius: var(--radius-lg);
  border: 1px solid var(--border-default);
  min-width: 80px;
  /*
   * 统一最小高度：用户消息走纯文本（white-space: pre-wrap）、AI 消息走 markdown（<p>），
   * 两条渲染路径的行盒计算存在细微差别；用同一个高度下限兜住，
   * 保证单行时两种气泡高度一致，且与头像齐平。
   */
  min-height: var(--bubble-min-h);
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

/* 工具卡片：折叠态只占一行（AI 做了什么），点击展开入参 / 结果 */
.tool-card {
  width: 100%;
  background: var(--bg-panel-secondary);
  border-color: var(--border-default);
  border-top-left-radius: var(--radius-sm);
  gap: 0;
  padding: 0;
  overflow: hidden;
}
.tool-head {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  width: 100%;
  padding: var(--space-2) var(--space-3);
  background: transparent;
  border: none;
  color: inherit;
  font-family: inherit;
  font-size: var(--text-sm);
  text-align: left;
  cursor: pointer;
  transition: background var(--transition-fast);
}
.tool-head:hover { background: var(--bg-hover); }
.tool-icon { flex-shrink: 0; color: var(--color-accent); }
.tool-action { flex-shrink: 0; font-weight: 600; color: var(--text-primary); }
.tool-subject {
  flex: 1 1 auto;
  min-width: 0;
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.tool-badge { font-size: var(--text-xs); padding: 1px 8px; border-radius: 999px; flex-shrink: 0; margin-left: auto; }
.tool-badge.ok { background: rgba(63, 185, 80, 0.15); color: #3fb950; }
.tool-badge.fail { background: rgba(248, 81, 73, 0.15); color: #f85149; }
.tool-chevron {
  flex-shrink: 0;
  color: var(--text-tertiary);
  transition: transform var(--transition-fast);
}
.tool-card.expanded .tool-chevron { transform: rotate(180deg); }
.tool-body {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  padding: 0 var(--space-3) var(--space-3);
}
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
/* 流式时的纯文本容器：保留换行 + 让用户能实时看到加粗等字符原样（流完才渲染） */
.content.content-raw {
  white-space: pre-wrap;
  /* 流式不渲染 markdown，所以 **、_、# 都原样显示——这是预期行为 */
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
.content :deep(strong) {
  /* 朱砂红字 + 加粗 700（无背景，让用户只看字色对比） */
  color: var(--color-primary);
  font-weight: 700;
}
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
