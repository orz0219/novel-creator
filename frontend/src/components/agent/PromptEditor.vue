<template>
  <NeDialog :model-value="modelValue" title="提示词调优" size="lg" @update:model-value="emit('update:modelValue', $event)">
    <div class="prompt-editor">
      <p class="hint">
        编辑系统提示词的<strong>人格与引导策略</strong>部分。工具列表、提问协议、当前阶段等结构性段落由系统自动追加在末尾（见下方预览），无需手写。
      </p>

      <div class="meta">
        <span class="chip">作用域：全局</span>
        <span class="chip" :class="isCustomized ? 'on' : ''">
          {{ isCustomized ? '已自定义' : '使用默认' }}
        </span>
        <span v-if="dirty" class="chip warn">有未保存改动</span>
        <span v-else-if="saved" class="chip ok">已保存</span>
      </div>

      <div class="editor-wrap" :style="{ fontFamily: 'var(--font-mono)' }">
        <NeTextarea
          v-model="draft"
          :rows="12"
          placeholder="在此编辑系统提示词基座…"
        />
      </div>

      <button class="preview-toggle" @click="showPreview = !showPreview">
        {{ showPreview ? '▾' : '▸' }} 预览最终生效提示词
      </button>
      <pre v-if="showPreview" class="preview">{{ preview }}</pre>
    </div>

    <template #footer>
      <NeButton variant="ghost" @click="restoreDefault" :disabled="saving">恢复默认</NeButton>
      <NeButton variant="primary" @click="save" :disabled="!dirty || saving" :loading="saving">保存</NeButton>
    </template>
  </NeDialog>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import NeDialog from '@/components/ui/NeDialog.vue'
import NeTextarea from '@/components/ui/NeTextarea.vue'
import NeButton from '@/components/ui/NeButton.vue'
import { getPrompt, savePrompt, deletePrompt, type PromptView } from '@/api/agent'
import { useAgentStore } from '@/stores/agent'

const props = defineProps<{ modelValue: boolean }>()
const emit = defineEmits<{ 'update:modelValue': [boolean] }>()

const store = useAgentStore()
const draft = ref('')
const original = ref('')
const defaultPrompt = ref('')
const isCustomized = ref(false)
const saving = ref(false)
const saved = ref(false)
const showPreview = ref(true)

const dirty = computed(() => draft.value !== original.value)

const appended = computed(() => {
  const lines = [
    '',
    '[以下由系统自动追加，无需手写]',
    '当前引导阶段：项目初始化',
    '你当前可用的工具：',
  ]
  for (const t of store.tools) lines.push(`- ${t.name}：${t.description}`)
  lines.push('提问交互协议（重要）：当用户需从多个选项中选择时，以 <<ASK_QUESTION>>{"question":"...","options":["选项1",...]}<<END>> 输出。')
  return lines.join('\n')
})

const preview = computed(() => draft.value + appended.value)

async function load() {
  saved.value = false
  try {
    const v: PromptView = await getPrompt('global')
    draft.value = v.system_prompt
    original.value = v.system_prompt
    defaultPrompt.value = v.default_prompt
    isCustomized.value = v.is_customized
  } catch (e) {
    // 加载失败不阻断，保持空白可编辑
  }
}

function restoreDefault() {
  draft.value = defaultPrompt.value
}

async function save() {
  if (!dirty.value || saving.value) return
  saving.value = true
  try {
    // 若编辑回内置默认，则删除自定义覆盖（真正恢复默认）；否则 upsert。
    if (draft.value === defaultPrompt.value) {
      await deletePrompt('global')
    } else {
      await savePrompt(draft.value, 'global')
    }
    const v: PromptView = await getPrompt('global')
    original.value = v.system_prompt
    defaultPrompt.value = v.default_prompt
    isCustomized.value = v.is_customized
    saved.value = true
  } finally {
    saving.value = false
  }
}

watch(
  () => props.modelValue,
  (open) => {
    if (open) load()
  },
)
</script>

<style scoped>
.prompt-editor { display: flex; flex-direction: column; gap: var(--space-3); }
.hint { font-size: var(--text-sm); color: var(--text-secondary); line-height: var(--leading-normal); margin: 0; }
.hint strong { color: var(--text-primary); }

.meta { display: flex; gap: var(--space-2); flex-wrap: wrap; }
.chip {
  font-size: var(--text-xs); padding: 2px var(--space-2);
  border: 1px solid var(--border-default); border-radius: 999px;
  color: var(--text-tertiary); background: var(--bg-panel-secondary);
}
.chip.on { color: var(--color-primary-text); border-color: var(--border-primary); background: var(--color-primary-subtle); }
.chip.warn { color: var(--color-warning); border-color: var(--color-warning); }
.chip.ok { color: var(--color-success); border-color: var(--color-success); }

.editor-wrap :deep(.ne-textarea) {
  font-size: var(--text-sm);
  line-height: var(--leading-relaxed);
  min-height: 260px;
  resize: vertical;
}

.preview-toggle {
  align-self: flex-start;
  background: transparent; border: none; cursor: pointer;
  color: var(--text-secondary); font-size: var(--text-xs); font-family: inherit;
  padding: 0;
}
.preview-toggle:hover { color: var(--text-primary); }

.preview {
  margin: 0; padding: var(--space-3);
  background: var(--bg-base); border: 1px dashed var(--border-default);
  border-radius: var(--radius-sm);
  font-family: var(--font-mono); font-size: var(--text-xs);
  line-height: var(--leading-relaxed); color: var(--text-secondary);
  white-space: pre-wrap; word-break: break-word; max-height: 240px; overflow-y: auto;
}
</style>
