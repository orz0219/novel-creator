<template>
  <div class="gf-panel">
    <!-- ① 一句话定位 -->
    <header class="gf-hero">
      <div class="hero-glow" aria-hidden="true"></div>
      <div class="hero-body">
        <div class="hero-meta">
          <span class="hero-badge">
            <Sparkles :size="13" />
            {{ profile?.gf_type || '金手指' }}
          </span>
          <span class="hero-version">v{{ finger.version }}</span>
        </div>
        <h1 class="hero-name">{{ finger.name }}</h1>
        <p v-if="profile?.one_liner" class="hero-line">{{ profile.one_liner }}</p>
        <p v-else class="hero-line is-missing">尚未写明「一句话作用」</p>
      </div>
      <div class="hero-actions">
        <button class="hero-btn" title="编辑" aria-label="编辑" @click="$emit('edit', finger)">
          <Pencil :size="14" />
        </button>
        <button
          class="hero-btn is-danger"
          title="删除"
          aria-label="删除"
          @click="$emit('delete', finger)"
        >
          <Trash2 :size="14" />
        </button>
      </div>
    </header>

    <!-- 档案未建立 / 结构不符：明确说清楚，不留白 -->
    <section v-if="read.kind === 'empty'" class="gf-notice is-guide">
      <div class="notice-head">
        <TriangleAlert :size="15" />
        <span>这份金手指还没有结构化档案</span>
      </div>
      <p class="notice-body">
        现在它只有上面的名称和原始描述文本，面板无法拆出「能干什么 / 代价是什么 / 什么时候会失效」。
        在对话里对 AI 说一句即可补全：
      </p>
      <code class="notice-code">请用 update_golden_finger 补全「{{ finger.name }}」的结构化档案</code>
    </section>

    <section v-else-if="read.kind === 'invalid'" class="gf-notice is-error">
      <div class="notice-head">
        <TriangleAlert :size="15" />
        <span>档案结构不符契约（共 {{ read.problems.length }} 处）</span>
      </div>
      <ul class="problem-list">
        <li v-for="(p, i) in read.problems" :key="i">{{ p }}</li>
      </ul>
    </section>

    <template v-if="profile">
      <!-- 档案完整度：一眼看出还缺什么 -->
      <div class="gf-progress">
        <div class="progress-bar">
          <div class="progress-fill" :style="{ width: progressPct + '%' }"></div>
        </div>
        <span class="progress-text">档案完整度 {{ completeness.filled }} / {{ completeness.total }}</span>
      </div>

      <!-- ② 核心机制 -->
      <section class="gf-section">
        <div class="section-head">
          <Zap class="section-icon" :size="15" />
          <h2 class="section-title">核心机制</h2>
          <span class="section-count">{{ profile.abilities.length }} 项</span>
        </div>
        <div v-if="profile.abilities.length" class="ability-grid">
          <article v-for="(ab, i) in profile.abilities" :key="i" class="ability-card">
            <span class="ability-num">{{ pad(i + 1) }}</span>
            <h3 class="ability-name">{{ ab.name }}</h3>
            <p class="ability-effect">{{ ab.effect }}</p>
            <dl v-if="ab.trigger || ab.limit" class="ability-meta">
              <div v-if="ab.trigger" class="meta-row">
                <dt>触发</dt>
                <dd>{{ ab.trigger }}</dd>
              </div>
              <div v-if="ab.limit" class="meta-row">
                <dt>边界</dt>
                <dd>{{ ab.limit }}</dd>
              </div>
            </dl>
          </article>
        </div>
        <p v-else class="section-empty">尚未填写核心机制</p>
      </section>

      <!-- ③ 代价与限制 -->
      <section class="gf-split">
        <div class="cost-block">
          <div class="block-head">
            <Scale :size="14" />
            <h2 class="block-title">代价 / 副作用</h2>
          </div>
          <p v-if="profile.side_effect" class="cost-text" :class="{ 'is-none': isNoCost }">
            {{ profile.side_effect }}
          </p>
          <p v-else class="block-empty">尚未填写</p>
        </div>

        <div class="constraint-block">
          <div class="block-head">
            <ShieldAlert :size="14" />
            <h2 class="block-title">硬约束</h2>
            <span class="block-count">{{ profile.constraints.length }}</span>
          </div>
          <ul v-if="profile.constraints.length" class="constraint-list">
            <li v-for="(c, i) in profile.constraints" :key="i">{{ c }}</li>
          </ul>
          <p v-else class="block-empty">尚未填写——写作时最容易在这里写崩</p>
        </div>
      </section>

      <!-- ④ 成长阶段 -->
      <section class="gf-section">
        <div class="section-head">
          <TrendingUp class="section-icon" :size="15" />
          <h2 class="section-title">成长阶段</h2>
          <span class="section-count">{{ profile.growth_stages.length }} 段</span>
        </div>
        <ol v-if="profile.growth_stages.length" class="stage-track">
          <li v-for="(s, i) in profile.growth_stages" :key="i" class="stage-item">
            <span class="stage-dot"></span>
            <div class="stage-body">
              <span class="stage-name">{{ s.stage }}</span>
              <p class="stage-unlocked">{{ s.unlocked }}</p>
              <p v-if="s.note" class="stage-note">{{ s.note }}</p>
            </div>
          </li>
        </ol>
        <p v-else class="section-empty">尚未划分成长阶段</p>
      </section>

      <!-- ⑤ 来源 -->
      <section v-if="profile.origin" class="gf-section">
        <div class="section-head">
          <BookOpen class="section-icon" :size="15" />
          <h2 class="section-title">来源</h2>
        </div>
        <p class="origin-text">{{ profile.origin }}</p>
      </section>
    </template>

    <!-- ⑥ 原始设定文本（保留完整信息，不丢） -->
    <details v-if="finger.summary || finger.description" class="gf-raw">
      <summary>
        <ChevronRight class="raw-chevron" :size="14" />
        原始设定文本
      </summary>
      <div class="raw-body">
        <p v-if="finger.summary" class="raw-summary">{{ finger.summary }}</p>
        <pre v-if="finger.description" class="raw-desc">{{ finger.description }}</pre>
      </div>
    </details>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { Entity } from '@/types'
import {
  readGoldenFinger,
  profileCompleteness,
  type GoldenFingerProfile,
} from '@/utils/goldenFinger'
import {
  Sparkles,
  Pencil,
  Trash2,
  TriangleAlert,
  Zap,
  Scale,
  ShieldAlert,
  TrendingUp,
  BookOpen,
  ChevronRight,
} from 'lucide-vue-next'

const props = defineProps<{ finger: Entity }>()
defineEmits<{ edit: [entity: Entity]; delete: [entity: Entity] }>()

const read = computed(() => readGoldenFinger(props.finger.attributes))
const profile = computed<GoldenFingerProfile | null>(() =>
  read.value.kind === 'ready' ? read.value.profile : null,
)

const completeness = computed(() =>
  profile.value ? profileCompleteness(profile.value) : { filled: 0, total: 7 },
)
const progressPct = computed(() =>
  completeness.value.total === 0
    ? 0
    : Math.round((completeness.value.filled / completeness.value.total) * 100),
)

/** 作者常写「无」表示没有副作用——那是设定，不是缺失，值得单独突出。 */
const isNoCost = computed(() => {
  const t = profile.value?.side_effect?.trim() ?? ''
  return t === '无' || t === 'none' || t === 'None' || t === '不适用'
})

function pad(n: number): string {
  return n.toString().padStart(2, '0')
}
</script>

<style scoped>
.gf-panel {
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
  padding-bottom: var(--space-8);
}

/* ---------- ① Hero ---------- */
.gf-hero {
  position: relative;
  overflow: hidden;
  padding: var(--space-6);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-lg);
  background:
    linear-gradient(135deg, var(--color-primary-subtle) 0%, transparent 55%),
    var(--bg-panel);
}
.hero-glow {
  position: absolute;
  top: -140px;
  right: -100px;
  width: 340px;
  height: 340px;
  background: radial-gradient(circle, var(--color-primary-subtle) 0%, transparent 68%);
  pointer-events: none;
}
.hero-body {
  position: relative;
}
.hero-meta {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  margin-bottom: var(--space-3);
}
.hero-badge {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 3px var(--space-3);
  border: 1px solid var(--color-primary);
  border-radius: 999px;
  background: var(--color-primary-subtle);
  color: var(--color-primary-text);
  font-size: var(--text-xs);
  letter-spacing: 0.04em;
}
.hero-version {
  font-size: var(--text-xs);
  color: var(--text-tertiary);
  font-family: var(--font-mono);
}
.hero-name {
  margin: 0 0 var(--space-2);
  font-family: var(--font-serif);
  font-size: var(--text-3xl);
  font-weight: 700;
  letter-spacing: 0.01em;
  color: var(--text-primary);
}
.hero-line {
  margin: 0;
  font-size: var(--text-lg);
  line-height: var(--leading-relaxed);
  color: var(--color-primary-text);
}
.hero-line.is-missing {
  color: var(--text-tertiary);
  font-size: var(--text-md);
}
.hero-actions {
  position: absolute;
  top: 0;
  right: 0;
  display: flex;
  gap: var(--space-1);
}
/* 纯图标按钮：默认低调（无边框透明），hover 才浮现，避免抢走名称与一句话作用的视觉重心 */
.hero-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  padding: 0;
  border: 1px solid transparent;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-tertiary);
  cursor: pointer;
  transition: all var(--transition-fast);
}
.hero-btn:hover {
  border-color: var(--border-default);
  background: var(--bg-hover);
  color: var(--text-primary);
}
.hero-btn.is-danger:hover {
  border-color: var(--color-error);
  background: var(--color-error-subtle);
  color: var(--color-error);
}

/* ---------- 提示块 ---------- */
.gf-notice {
  padding: var(--space-4);
  border-radius: var(--radius-md);
  border: 1px solid var(--border-default);
  background: var(--bg-panel);
}
.gf-notice.is-guide {
  border-color: var(--color-accent);
  background: var(--color-accent-subtle);
}
.gf-notice.is-error {
  border-color: var(--color-warning);
  background: var(--color-warning-subtle);
}
.notice-head {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin-bottom: var(--space-2);
  font-size: var(--text-md);
  font-weight: 600;
}
.is-guide .notice-head {
  color: var(--color-accent);
}
.is-error .notice-head {
  color: var(--color-warning);
}
.notice-body {
  margin: 0 0 var(--space-3);
  font-size: var(--text-sm);
  line-height: var(--leading-relaxed);
  color: var(--text-secondary);
}
.notice-code {
  display: block;
  padding: var(--space-2) var(--space-3);
  border-radius: var(--radius-sm);
  background: var(--bg-base);
  color: var(--text-primary);
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  user-select: all;
}
.problem-list {
  margin: 0;
  padding-left: var(--space-5);
  font-size: var(--text-sm);
  color: var(--text-secondary);
  line-height: var(--leading-relaxed);
}

/* ---------- 完整度 ---------- */
.gf-progress {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}
.progress-bar {
  flex: 1;
  height: 3px;
  border-radius: 999px;
  background: var(--bg-panel-tertiary);
  overflow: hidden;
}
.progress-fill {
  height: 100%;
  border-radius: 999px;
  background: linear-gradient(90deg, var(--color-primary), var(--color-primary-hover));
  transition: width var(--transition-normal);
}
.progress-text {
  font-size: var(--text-xs);
  color: var(--text-tertiary);
  white-space: nowrap;
}

/* ---------- 通用 section ---------- */
.gf-section {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}
.section-head {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
.section-icon {
  color: var(--color-primary);
  flex-shrink: 0;
}
.section-title {
  margin: 0;
  font-size: var(--text-lg);
  font-weight: 600;
  color: var(--text-primary);
}
.section-count {
  margin-left: auto;
  font-size: var(--text-xs);
  color: var(--text-tertiary);
}
.section-empty {
  margin: 0;
  padding: var(--space-4);
  border: 1px dashed var(--border-default);
  border-radius: var(--radius-md);
  color: var(--text-tertiary);
  font-size: var(--text-sm);
  text-align: center;
}

/* ---------- ② 核心机制 ---------- */
.ability-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: var(--space-3);
}
.ability-card {
  position: relative;
  padding: var(--space-4);
  padding-right: var(--space-10);
  border: 1px solid var(--border-default);
  border-left: 2px solid var(--color-primary);
  border-radius: var(--radius-md);
  background: var(--bg-panel);
  transition: transform var(--transition-fast), border-color var(--transition-fast);
}
.ability-card:hover {
  transform: translateY(-2px);
  border-color: var(--border-emphasis);
  border-left-color: var(--color-primary-hover);
}
.ability-num {
  position: absolute;
  top: var(--space-2);
  right: var(--space-3);
  font-family: var(--font-mono);
  font-size: var(--text-2xl);
  font-weight: 700;
  line-height: 1;
  color: var(--color-primary);
  opacity: 0.16;
  user-select: none;
}
.ability-name {
  margin: 0 0 var(--space-2);
  font-size: var(--text-md);
  font-weight: 600;
  color: var(--text-primary);
}
.ability-effect {
  margin: 0;
  font-size: var(--text-sm);
  line-height: var(--leading-relaxed);
  color: var(--text-secondary);
}
.ability-meta {
  margin: var(--space-3) 0 0;
  padding-top: var(--space-3);
  border-top: 1px solid var(--border-muted);
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.meta-row {
  display: flex;
  gap: var(--space-2);
  font-size: var(--text-xs);
  line-height: var(--leading-normal);
}
.meta-row dt {
  flex-shrink: 0;
  padding: 1px 5px;
  border-radius: 3px;
  background: var(--bg-panel-tertiary);
  color: var(--text-tertiary);
}
.meta-row dd {
  margin: 0;
  color: var(--text-secondary);
}

/* ---------- ③ 代价 / 约束 ---------- */
.gf-split {
  display: grid;
  grid-template-columns: minmax(200px, 1fr) minmax(260px, 1.6fr);
  gap: var(--space-3);
}
@media (max-width: 720px) {
  .gf-split {
    grid-template-columns: 1fr;
  }
}
.cost-block,
.constraint-block {
  padding: var(--space-4);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  background: var(--bg-panel);
}
.block-head {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin-bottom: var(--space-3);
  color: var(--text-tertiary);
}
.block-title {
  margin: 0;
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--text-secondary);
}
.block-count {
  margin-left: auto;
  font-size: var(--text-xs);
  color: var(--text-tertiary);
}
.cost-text {
  margin: 0;
  font-size: var(--text-md);
  line-height: var(--leading-relaxed);
  color: var(--text-primary);
}
/* 设定就是「无副作用」时给一个明确的肯定色，而不是让它看起来像没填 */
.cost-text.is-none {
  color: var(--color-success);
  font-weight: 600;
}
.block-empty {
  margin: 0;
  font-size: var(--text-sm);
  color: var(--text-tertiary);
}
.constraint-list {
  margin: 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.constraint-list li {
  position: relative;
  padding: var(--space-2) var(--space-3);
  padding-left: var(--space-5);
  border-left: 2px solid var(--color-warning);
  border-radius: var(--radius-sm);
  background: var(--color-warning-subtle);
  font-size: var(--text-sm);
  line-height: var(--leading-relaxed);
  color: var(--text-primary);
}
.constraint-list li::before {
  content: '';
  position: absolute;
  left: 7px;
  top: calc(var(--space-2) + 6px);
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: var(--color-warning);
}

/* ---------- ④ 成长阶段 ---------- */
.stage-track {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(190px, 1fr));
  gap: var(--space-4);
  margin: 0;
  padding: 0;
  list-style: none;
}
.stage-item {
  position: relative;
  padding-top: var(--space-5);
}
/* 阶段之间的横向连线 */
.stage-item::before {
  content: '';
  position: absolute;
  top: 5px;
  left: 0;
  right: 0;
  height: 1px;
  background: var(--border-default);
}
.stage-item:first-child::before {
  left: 50%;
}
.stage-item:last-child::before {
  right: 50%;
}
.stage-dot {
  position: absolute;
  top: 0;
  left: 50%;
  transform: translateX(-50%);
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--color-primary);
  box-shadow: 0 0 0 3px var(--color-primary-subtle);
}
.stage-body {
  text-align: center;
}
.stage-name {
  display: inline-block;
  margin-bottom: var(--space-2);
  padding: 2px var(--space-3);
  border-radius: 999px;
  background: var(--bg-panel-tertiary);
  color: var(--text-primary);
  font-size: var(--text-sm);
  font-weight: 600;
}
.stage-unlocked {
  margin: 0;
  font-size: var(--text-sm);
  line-height: var(--leading-relaxed);
  color: var(--text-secondary);
}
.stage-note {
  margin: var(--space-2) 0 0;
  font-size: var(--text-xs);
  line-height: var(--leading-normal);
  color: var(--text-tertiary);
}

/* ---------- ⑤ 来源 ---------- */
.origin-text {
  margin: 0;
  padding: var(--space-4);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  background: var(--bg-panel);
  font-size: var(--text-sm);
  line-height: var(--leading-relaxed);
  color: var(--text-secondary);
}

/* ---------- ⑥ 原始文本 ---------- */
.gf-raw {
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  background: var(--bg-panel);
}
.gf-raw summary {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  cursor: pointer;
  font-size: var(--text-sm);
  color: var(--text-secondary);
  list-style: none;
}
.gf-raw summary::-webkit-details-marker {
  display: none;
}
.raw-chevron {
  transition: transform var(--transition-fast);
  color: var(--text-tertiary);
}
.gf-raw[open] .raw-chevron {
  transform: rotate(90deg);
}
.raw-body {
  padding: 0 var(--space-4) var(--space-4);
}
.raw-summary {
  margin: 0 0 var(--space-3);
  font-size: var(--text-sm);
  line-height: var(--leading-relaxed);
  color: var(--text-secondary);
}
.raw-desc {
  margin: 0;
  padding: var(--space-3);
  border-radius: var(--radius-sm);
  background: var(--bg-base);
  font-family: var(--font-sans);
  font-size: var(--text-sm);
  line-height: var(--leading-relaxed);
  color: var(--text-tertiary);
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
