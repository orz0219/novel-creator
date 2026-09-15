<!--
  ConfirmAdvance.vue
  内联推进按钮：放在 composer 工具栏的左下。
  - 朱砂红按钮：产物够就能点
  - 角标：当前阶段还差几项（来自后端 guide/status 的 missing）
  - hover：可看具体差什么
  - 点击：调 confirm_step → 失败则显示错误条（盖在 composer 顶部）
  - 不再有"驳回"输入框——驳回文本就是 composer 的 textarea 本身
  - 走到最后一步（正文写作）时不再是禁用的「已是最后一步」，
    而是换成可点的「去写作页」——引导走完不等于无事可做
-->
<template>
  <div class="ca-inline">
    <!-- 引导已走完（最后一站是「正文」）：不再给一个禁用的死按钮，
         而是把用户送去真正该去的地方——写作页。 -->
    <button
      v-if="isLastStep"
      class="ca-btn done"
      :disabled="disabled === true"
      :title="terminalHint"
      @click="goWriting"
    >
      <PenLine :size="14" />
      <span>去写作页</span>
    </button>
    <button
      v-else
      class="ca-btn"
      :class="{ 'has-missing': missingItems.length > 0 }"
      :disabled="isDisabled"
      @click="onConfirm"
      :title="hintTitle"
    >
      <ArrowRight :size="14" />
      <span>{{ busy ? '推进中…' : buttonText }}</span>
      <span v-if="missingItems.length > 0" class="ca-badge">
        {{ missingItems.length }}
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
import { useRouter } from 'vue-router'
import { ArrowRight, AlertCircle, PenLine } from 'lucide-vue-next'
import { confirmGuideStep } from '@/api/agent'

const props = defineProps<{
  /** 当前阶段 title（用于按钮文案兜底） */
  currentTitle: string
  /** 下一个阶段 title（按钮文案）；为空表示引导已到最后一步（现在的最后一站是「正文」） */
  nextTitle: string
  /** 项目 id（调 confirm_step 必备，也用于最后一步跳写作页） */
  projectId: string
  /** 当前阶段未就绪时后端给出的缺失项描述（驱动角标 + hover 提示）。
   *  由后端 `guide/status` 提供——前端不自己判断"缺什么"，
   *  否则会出现「按钮说齐了、点下去报缺东西」这类分叉。 */
  missing?: string[]
  /** 外部禁用（例如 AI 正在生成 / 调用工具时，不允许并发推进） */
  disabled?: boolean
}>()

const emit = defineEmits<{
  /** 推进成功后，通知父组件重新拉取 current_step */
  advanced: []
}>()

const router = useRouter()
const busy = ref(false)
const error = ref<string | null>(null)

/** 后端给出的缺失项 */
const missingItems = computed<string[]>(() => props.missing ?? [])

/** 已是最后一步（正文写作是流程终点）：没有可推进的下一步。
 *  注意这里不再把按钮置灰——引导走完不等于无事可做，而是该去写作页了。 */
const isLastStep = computed(() => !props.nextTitle)

/** 生成中或自身请求中不可点（最后一步的「去写作页」不受生成中影响之外的限制） */
const isDisabled = computed(() => busy.value || props.disabled === true)

const buttonText = computed(() =>
  isLastStep.value ? '去写作页' : `推进到「${props.nextTitle}」`,
)

const terminalHint = computed(
  () => `引导流程已走完（最后一站是「${props.currentTitle || '正文'}」）：去写作页选场景开始写`,
)

/** hover 提示文案 */
const hintTitle = computed(() => {
  if (missingItems.value.length === 0) {
    return `当前阶段「${props.currentTitle}」产物已足，点此推进到「${props.nextTitle}」`
  }
  return `推进还差 ${missingItems.value.length} 项：${missingItems.value.join('；')}`
})

function goWriting() {
  router.push(`/project/${props.projectId}/write`)
}

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
.ca-btn.has-missing {
  background: var(--bg-panel-secondary);
  color: var(--text-secondary);
  border-style: dashed;
  border-color: var(--border-emphasis);
  box-shadow: none;
}
.ca-btn.has-missing:hover:not(:disabled) {
  background: var(--bg-hover);
  color: var(--text-primary);
  border-color: var(--color-primary);
  transform: none;
}

/* 引导走完（最后一站「正文」）：按钮变成去写作页的出口，
   用中性底 + 绿色描边表示"已完成"，而不是待推进的朱砂红。 */
.ca-btn.done {
  background: var(--bg-panel-secondary);
  color: var(--text-primary);
  border-color: var(--color-success);
  font-family: var(--font-sans, inherit);
}
.ca-btn.done:hover:not(:disabled) {
  background: var(--bg-hover);
  transform: none;
  box-shadow: none;
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
