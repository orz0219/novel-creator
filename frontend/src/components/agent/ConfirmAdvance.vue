<!--
  ConfirmAdvance.vue
  内联推进按钮：放在 composer 工具栏的左下。
  - 朱砂红按钮：产物够就能点
  - 角标：未补血肉数
  - hover：可看血肉完成度
  - 点击：调 confirm_step → 失败则显示错误条（盖在 composer 顶部）
  - 不再有"驳回"输入框——驳回文本就是 composer 的 textarea 本身
-->
<template>
  <div class="ca-inline">
    <button
      class="ca-btn"
      :class="{ 'has-flesh': unfinishedFlesh.length > 0 }"
      :disabled="isDisabled"
      @click="onConfirm"
      :title="hintTitle"
    >
      <ArrowRight :size="14" />
      <span>{{ busy ? '推进中…' : `推进到「${nextTitle}」` }}</span>
      <span v-if="unfinishedFlesh.length > 0" class="ca-badge">
        {{ unfinishedFlesh.length }}
      </span>
    </button>

    <!-- 错误条：跨整行（在 composer 工具栏上方） -->
    <div v-if="error" class="ca-error">
      <AlertCircle :size="12" /> {{ error }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { ArrowRight, AlertCircle } from 'lucide-vue-next'
import { confirmGuideStep } from '@/api/agent'

export interface FleshStep {
  key: string
  title: string
  /** true = 已完成，false = 未完成 */
  done: boolean
}

const props = defineProps<{
  /** 当前阶段 title（用于按钮文案兜底） */
  currentTitle: string
  /** 下一个阶段 title（按钮文案） */
  nextTitle: string
  /** 项目 id（调 confirm_step 必备） */
  projectId: string
  /** 血肉 step 列表（驱动角标 + hint） */
  fleshSteps?: FleshStep[]
  /** 外部禁用（例如 AI 正在生成 / 调用工具时，不允许并发推进） */
  disabled?: boolean
}>()

const emit = defineEmits<{
  /** 推进成功后，通知父组件重新拉取 current_step */
  advanced: []
}>()

const busy = ref(false)
const error = ref<string | null>(null)

/** 生成中或自身请求中时都不可点，避免并发推进把会话状态打回 idle。 */
const isDisabled = computed(() => busy.value || props.disabled === true)

/** 全部血肉 step 列表 */
const fleshSteps = computed<FleshStep[]>(() => props.fleshSteps || [])

/** 未完成血肉 */
const unfinishedFlesh = computed<FleshStep[]>(() =>
  fleshSteps.value.filter((f) => !f.done),
)

/** hover 提示文案 */
const hintTitle = computed(() => {
  if (unfinishedFlesh.value.length === 0) {
    return `当前阶段「${props.currentTitle}」产物已足，点此推进到「${props.nextTitle}」`
  }
  return `还有 ${unfinishedFlesh.value.length} 个血肉未补：${unfinishedFlesh.value
    .map((f) => f.title)
    .join('、')}`
})

async function onConfirm() {
  if (isDisabled.value) return
  busy.value = true
  error.value = null
  try {
    const r = await confirmGuideStep(props.projectId)
    if (!r.passed) {
      error.value = `还差：${(r.missing || []).map((m: any) => m.detail).join('、') || '未知'}`
      return
    }
    emit('advanced')
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    busy.value = false
  }
}
</script>

<style scoped>
.ca-inline { display: contents; }

/* 推进按钮：朱砂红 + 角标 */
.ca-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  background: var(--color-primary);
  color: #fff;
  border: 1px solid var(--color-primary);
  border-radius: var(--radius-md);
  font-size: 12px;
  font-family: var(--font-serif);
  font-weight: 600;
  letter-spacing: 0.5px;
  cursor: pointer;
  transition: all 0.15s ease;
  position: relative;
}
.ca-btn:hover:not(:disabled) {
  background: var(--color-primary-hover);
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(200, 75, 49, 0.3);
}
.ca-btn:disabled {
  opacity: 0.7;
  cursor: not-allowed;
}

/* 有未补血肉：按钮变虚线边框 + 灰色（视觉提示产物不够） */
.ca-btn.has-flesh {
  background: var(--bg-panel-secondary);
  color: var(--text-secondary);
  border-style: dashed;
  border-color: var(--border-emphasis);
  box-shadow: none;
}
.ca-btn.has-flesh:hover:not(:disabled) {
  background: var(--bg-hover);
  color: var(--text-primary);
  border-color: var(--color-primary);
  transform: none;
}

.ca-badge {
  min-width: 16px;
  height: 16px;
  padding: 0 4px;
  background: #ff4d4f;
  color: #fff;
  font-size: 10px;
  font-weight: 700;
  border-radius: 8px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-family: var(--font-sans, inherit);
}

/* 错误条 */
.ca-error {
  position: absolute;
  bottom: 100%;
  left: 0;
  right: 0;
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--color-error);
  padding: 6px 12px;
  background: var(--color-error-subtle);
  border: 1px solid var(--color-error);
  border-bottom: none;
  border-radius: var(--radius-md) var(--radius-md) 0 0;
  font-family: var(--font-sans, inherit);
}
</style>
