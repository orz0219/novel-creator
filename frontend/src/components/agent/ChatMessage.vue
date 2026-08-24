<template>
  <div class="msg" :class="role">
    <div class="avatar" :class="role">
      <span v-if="role === 'assistant'" class="serif">导</span>
      <span v-else>我</span>
    </div>

    <!-- 选择题卡片 -->
    <div v-if="q" class="bubble question-card">
      <div class="role-label">创作向导 · 请选择</div>
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

    <!-- 普通文本气泡 -->
    <div v-else class="bubble" :class="{ streaming }">
      <div class="role-label">{{ role === 'assistant' ? '创作向导' : '你' }}</div>
      <div class="content">
        <span v-if="!content && streaming" class="thinking">正在思考…</span>
        <span class="text">{{ content }}</span><span v-if="streaming && content" class="caret"></span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'

const props = defineProps<{
  role: 'user' | 'assistant'
  content: string
  streaming?: boolean
}>()

const emit = defineEmits<{ select: [text: string] }>()

const MARKER = '<<ASK_QUESTION>>'
const END = '<<END>>'

const isQuestion = computed(() => props.content.includes(MARKER))
const q = computed(() => {
  if (!isQuestion.value) return null
  const start = props.content.indexOf(MARKER) + MARKER.length
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
.msg.user .role-label { text-align: right; }

.content {
  font-size: var(--text-md);
  line-height: var(--leading-relaxed);
  color: var(--text-primary);
  white-space: pre-wrap;
  word-break: break-word;
}
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
