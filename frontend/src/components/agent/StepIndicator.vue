<!--
  StepIndicator.vue
  10 步引导进度条（印章式设计 + 骨架/血肉区分）
  - 骨架 step：实色 + 必走
  - 血肉 step：空心 + 虚线 + 可自由顺序
  - 已完成：实色 + 勾
  - 当前：朱砂红实心 + 脉冲
  - 未完成：虚化
-->
<template>
  <div class="step-indicator" :class="{ 'is-loading': loading }">
    <div class="step-rail">
      <template v-for="(s, i) in steps" :key="s.key">
        <!-- 骨架→血肉切换：加虚线分割 -->
        <div
          v-if="i > 0 && s.group === 'flesh' && steps[i - 1].group === 'skeleton'"
          class="group-divider"
          :title="`切换到血肉 step（可自由顺序）`"
        >
          <span class="divider-text">血肉</span>
        </div>
        <div
          v-else-if="i > 0 && s.group === 'skeleton' && steps[i - 1].group === 'flesh'"
          class="group-divider flesh-to-skel"
          :title="`回到骨架 step（严格顺序）`"
        >
          <span class="divider-text">骨架</span>
        </div>
        <div
          class="step-node"
          :class="{
            done: stateOf(s.key) === 'done',
            current: stateOf(s.key) === 'current',
            upcoming: stateOf(s.key) === 'upcoming',
            flesh: s.group === 'flesh',
            skeleton: s.group === 'skeleton',
          }"
          :title="`${s.title} (${s.group === 'skeleton' ? '骨架' : '血肉'} step)`"
        >
          <div class="seal">
            <Check v-if="stateOf(s.key) === 'done'" :size="14" />
            <span v-else>{{ i + 1 }}</span>
          </div>
          <div class="step-label">{{ s.title }}</div>
          <div v-if="i < steps.length - 1" class="step-line"></div>
        </div>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { Check } from 'lucide-vue-next'

export interface StepDef {
  key: string
  title: string
  group: 'skeleton' | 'flesh'
}

const props = defineProps<{
  current: string
  steps: StepDef[]
  loading?: boolean
}>()

function stateOf(key: string): 'done' | 'current' | 'upcoming' {
  const cur = props.steps.findIndex((s) => s.key === props.current)
  const i = props.steps.findIndex((s) => s.key === key)
  if (cur === -1) return 'upcoming'
  if (i < cur) return 'done'
  if (i === cur) return 'current'
  return 'upcoming'
}
</script>

<style scoped>
.step-indicator {
  padding: 10px 20px 12px;
  background: var(--bg-panel);
  border-bottom: 1px solid var(--border-default);
  flex-shrink: 0;
  overflow-x: auto;
}
.step-indicator.is-loading { opacity: 0.85; }

.step-rail {
  display: flex;
  align-items: center;
  justify-content: flex-start;
  gap: 0;
  min-width: max-content;
}

.step-node {
  display: flex;
  flex-direction: column;
  align-items: center;
  position: relative;
  flex: 0 0 auto;
  width: 80px;
  min-width: 0;
}

.seal {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-family: var(--font-serif);
  font-weight: 700;
  font-size: 13px;
  border: 1px solid var(--border-default);
  background: var(--bg-panel-secondary);
  color: var(--text-tertiary);
  transition: all 0.25s ease;
  position: relative;
  z-index: 2;
}

.step-label {
  margin-top: 6px;
  font-size: 11px;
  color: var(--text-tertiary);
  white-space: nowrap;
  font-family: var(--font-serif);
  letter-spacing: 0.5px;
  transition: color 0.25s ease;
  text-align: center;
}

.step-line {
  position: absolute;
  top: 14px;
  left: calc(50% + 18px);
  width: 80px;
  height: 1px;
  background: var(--border-default);
  z-index: 1;
}

/* ===== 骨架 step（实色） ===== */
.step-node.skeleton.done .seal {
  background: var(--color-primary-subtle);
  border-color: var(--color-primary);
  color: var(--color-primary-text);
}
.step-node.skeleton.done .step-label { color: var(--color-primary-text); }
.step-node.skeleton.done .step-line { background: var(--color-primary); }

.step-node.skeleton.current .seal {
  background: var(--color-primary);
  border-color: var(--color-primary);
  color: #fff;
  box-shadow: 0 0 0 0 rgba(200, 75, 49, 0.5);
  animation: pulse-seal 1.8s ease-out infinite;
}
.step-node.skeleton.current .step-label {
  color: var(--text-primary);
  font-weight: 600;
}

/* ===== 血肉 step（空心虚线） ===== */
.step-node.flesh .seal {
  /* 默认状态：空心 */
  background: var(--bg-base);
  border-style: dashed;
  border-color: var(--border-emphasis);
  color: var(--text-tertiary);
}
.step-node.flesh .step-line {
  /* 虚线连接 */
  background: repeating-linear-gradient(
    to right,
    var(--border-default) 0,
    var(--border-default) 4px,
    transparent 4px,
    transparent 8px
  );
}

.step-node.flesh.done .seal {
  background: var(--bg-panel-secondary);
  border-style: solid;
  border-color: var(--color-primary);
  color: var(--color-primary-text);
}
.step-node.flesh.done .step-label { color: var(--color-primary-text); }
.step-node.flesh.done .step-line { background: var(--color-primary); }

.step-node.flesh.current .seal {
  background: var(--color-primary-subtle);
  border-style: solid;
  border-color: var(--color-primary);
  color: var(--color-primary-text);
  font-weight: 600;
  animation: pulse-seal-soft 2s ease-out infinite;
}
.step-node.flesh.current .step-label { color: var(--text-primary); font-weight: 600; }

@keyframes pulse-seal {
  0% { box-shadow: 0 0 0 0 rgba(200, 75, 49, 0.55); }
  70% { box-shadow: 0 0 0 8px rgba(200, 75, 49, 0); }
  100% { box-shadow: 0 0 0 0 rgba(200, 75, 49, 0); }
}
@keyframes pulse-seal-soft {
  0%, 100% { box-shadow: 0 0 0 0 rgba(200, 75, 49, 0.35); }
  50% { box-shadow: 0 0 0 6px rgba(200, 75, 49, 0); }
}

/* ===== 骨架/血肉分割条 ===== */
.group-divider {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  flex: 0 0 auto;
  width: 60px;
  height: 28px;
  margin-top: -16px;
  position: relative;
}
.group-divider::before {
  content: '';
  position: absolute;
  top: 14px;
  left: 0;
  right: 0;
  height: 1px;
  border-top: 1px dashed var(--text-disabled);
}
.divider-text {
  position: relative;
  z-index: 1;
  font-size: 10px;
  color: var(--text-disabled);
  background: var(--bg-panel);
  padding: 0 6px;
  font-family: var(--font-serif);
  letter-spacing: 1px;
}
.group-divider.flesh-to-skel::before {
  border-top-style: solid;
  border-top-color: var(--color-primary);
}
.group-divider.flesh-to-skel .divider-text { color: var(--color-primary-text); }

.step-node:hover .step-label { color: var(--text-primary); }
</style>
