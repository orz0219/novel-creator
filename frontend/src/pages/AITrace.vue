<template>
  <div class="trace-page">
    <div class="page-header">
      <h1 class="page-title">AI 追溯</h1>
      <span class="hint">每次 LLM 调用与校验的完整审计：prompt / 响应 / token / 延迟</span>
    </div>

    <!-- Generation Runs -->
    <section class="section">
      <h2 class="section-title">🤖 生成运行（generation_run）</h2>
      <div v-if="loadingGen" class="empty-state">加载中…</div>
      <div v-else-if="genRuns.length" class="run-list">
        <div v-for="run in genRuns" :key="run.id" class="run-card">
          <button class="run-header" @click="toggleGen(run.id)">
            <span class="run-id">#{{ run.id.slice(0, 8) }}</span>
            <span class="chip">{{ run.llm_model }}</span>
            <span class="chip subtle" v-if="run.provider">{{ run.provider }}</span>
            <span class="chip" v-if="run.latency_ms != null">{{ formatLatency(run.latency_ms) }}</span>
            <span class="chip" v-if="tokenTotal(run) != null">tokens {{ tokenTotal(run) }}</span>
            <span class="run-time">{{ formatDate(run.created_at) }}</span>
            <span class="toggle">{{ expandedGen === run.id ? '▾' : '▸' }}</span>
          </button>
          <div v-if="expandedGen === run.id" class="run-detail">
            <div class="detail-row">
              <span class="detail-label">任务 ID</span>
              <code class="mono">{{ run.task_id }}</code>
            </div>
            <div class="detail-row" v-if="run.context_snapshot_id">
              <span class="detail-label">上下文快照</span>
              <code class="mono">{{ run.context_snapshot_id }}</code>
            </div>
            <div class="detail-row" v-if="run.token_usage">
              <span class="detail-label">Token 明细</span>
              <code class="mono">{{ JSON.stringify(run.token_usage) }}</code>
            </div>
            <div class="detail-block">
              <span class="detail-label">Prompt</span>
              <pre class="pre">{{ run.prompt_sent || '（空）' }}</pre>
            </div>
            <div class="detail-block">
              <span class="detail-label">Response</span>
              <pre class="pre">{{ run.response_received || '（空）' }}</pre>
            </div>
          </div>
        </div>
      </div>
      <div v-else class="empty-state">暂无生成运行记录</div>
    </section>

    <!-- Validation Runs -->
    <section class="section">
      <h2 class="section-title">✅ 校验运行（validation_run）</h2>
      <div v-if="loadingVal" class="empty-state">加载中…</div>
      <div v-else-if="valRuns.length" class="run-list">
        <div v-for="run in valRuns" :key="run.id" class="run-card">
          <button class="run-header" @click="toggleVal(run.id)">
            <span class="run-id">#{{ run.id.slice(0, 8) }}</span>
            <span class="chip" :class="statusClass(run.status)">{{ run.status }}</span>
            <span class="chip subtle">校验 {{ run.changes_validated }} · 通过 {{ run.changes_approved }} · 驳回 {{ run.changes_rejected }}</span>
            <span class="chip warn" v-if="run.issues.length">{{ run.issues.length }} 个问题</span>
            <span class="run-time">{{ formatDate(run.started_at) }}</span>
            <span class="toggle">{{ expandedVal === run.id ? '▾' : '▸' }}</span>
          </button>
          <div v-if="expandedVal === run.id" class="run-detail">
            <div class="detail-row" v-if="run.completed_at">
              <span class="detail-label">完成时间</span>
              <span>{{ formatDate(run.completed_at) }}</span>
            </div>
            <div class="issue-list" v-if="run.issues.length">
              <div v-for="issue in run.issues" :key="issue.id" class="issue-item">
                <span class="chip" :class="severityClass(issue.severity)">{{ issue.severity }}</span>
                <span class="chip subtle">{{ issue.issue_type }}</span>
                <span class="issue-msg">{{ issue.message }}</span>
                <span class="issue-suggestion" v-if="issue.suggestion">建议：{{ issue.suggestion }}</span>
              </div>
            </div>
            <p v-else class="no-issue">本轮无校验问题。</p>
          </div>
        </div>
      </div>
      <div v-else class="empty-state">暂无校验运行记录</div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useProjectStore } from '@/stores/project'
import {
  traceApi,
  type GenerationRun,
  type ValidationRun,
} from '@/api/trace'

const route = useRoute()
const projectStore = useProjectStore()
const projectId = (route.params.id as string) || projectStore.currentProject?.id || ''

const genRuns = ref<GenerationRun[]>([])
const valRuns = ref<ValidationRun[]>([])
const loadingGen = ref(false)
const loadingVal = ref(false)
const expandedGen = ref<string | null>(null)
const expandedVal = ref<string | null>(null)

onMounted(async () => {
  loadingGen.value = true
  loadingVal.value = true
  try {
    genRuns.value = await traceApi.listGenerationRuns(projectId).catch(() => [])
  } finally {
    loadingGen.value = false
  }
  try {
    valRuns.value = await traceApi.listValidationRuns(projectId).catch(() => [])
  } finally {
    loadingVal.value = false
  }
})

function toggleGen(id: string) {
  expandedGen.value = expandedGen.value === id ? null : id
}
function toggleVal(id: string) {
  expandedVal.value = expandedVal.value === id ? null : id
}

function tokenTotal(run: GenerationRun): number | null {
  const t: any = run.token_usage
  if (!t || typeof t !== 'object') return null
  if (typeof t.total_tokens === 'number') return t.total_tokens
  const sum = Object.values(t).reduce((a: number, v) => a + (typeof v === 'number' ? v : 0), 0)
  return sum > 0 ? sum : null
}

function formatLatency(ms: number): string {
  return ms >= 1000 ? `${(ms / 1000).toFixed(1)}s` : `${ms}ms`
}

function formatDate(dateStr: string): string {
  try {
    const d = new Date(dateStr)
    return `${d.getMonth() + 1}月${d.getDate()}日 ${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`
  } catch {
    return dateStr
  }
}

function statusClass(status: string): string {
  if (status === 'Passed' || status === 'Completed') return 'ok'
  if (status === 'Failed') return 'danger'
  return 'subtle'
}

function severityClass(severity: string): string {
  const s = severity.toLowerCase()
  if (s.includes('error') || s.includes('critical')) return 'danger'
  if (s.includes('warn')) return 'warn'
  return 'subtle'
}
</script>

<style scoped>
.trace-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
.page-header { margin-bottom: var(--space-6); }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }
.hint { display: block; margin-top: var(--space-1); font-size: var(--text-xs); color: var(--text-tertiary); }
.section { margin-bottom: var(--space-6); }
.section-title { font-size: var(--text-md); font-weight: 600; margin-bottom: var(--space-3); }
.run-list { display: flex; flex-direction: column; gap: var(--space-2); }
.run-card { border: 1px solid var(--border-default); border-radius: var(--radius-sm); background: var(--bg-panel); overflow: hidden; }
.run-header { display: flex; align-items: center; gap: var(--space-2); width: 100%; padding: var(--space-3) var(--space-4); background: transparent; border: none; cursor: pointer; text-align: left; color: inherit; }
.run-header:hover { background: var(--bg-hover); }
.run-id { font-family: var(--font-mono); font-size: var(--text-sm); color: var(--text-tertiary); }
.run-time { margin-left: auto; font-size: var(--text-xs); color: var(--text-tertiary); }
.toggle { color: var(--text-tertiary); }
.chip { font-size: 11px; padding: 2px 8px; border-radius: 10px; background: var(--bg-panel-secondary); color: var(--text-secondary); white-space: nowrap; }
.chip.subtle { color: var(--text-tertiary); }
.chip.ok { background: var(--color-success-subtle, rgba(34,197,94,.15)); color: var(--color-success, #16a34a); }
.chip.warn { background: rgba(234,179,8,.15); color: #a16207; }
.chip.danger { background: var(--color-error-subtle); color: var(--color-error); }
.run-detail { padding: var(--space-3) var(--space-4); border-top: 1px solid var(--border-muted); }
.detail-row { display: flex; gap: var(--space-3); align-items: baseline; margin-bottom: var(--space-2); font-size: var(--text-sm); }
.detail-label { flex: 0 0 88px; color: var(--text-tertiary); font-size: var(--text-xs); }
.mono { word-break: break-all; }
.detail-block { margin-top: var(--space-3); }
.pre { margin: var(--space-1) 0 0; padding: var(--space-3); background: var(--bg-panel-secondary); border-radius: var(--radius-sm); font-family: var(--font-mono); font-size: var(--text-xs); max-height: 280px; overflow-y: auto; white-space: pre-wrap; word-break: break-all; }
.issue-list { display: flex; flex-direction: column; gap: var(--space-2); margin-top: var(--space-2); }
.issue-item { display: flex; align-items: baseline; gap: var(--space-2); flex-wrap: wrap; font-size: var(--text-sm); }
.issue-msg { color: var(--text-primary); }
.issue-suggestion { color: var(--text-tertiary); font-size: var(--text-xs); }
.no-issue { font-size: var(--text-sm); color: var(--text-tertiary); }
.empty-state { padding: var(--space-8); text-align: center; color: var(--text-tertiary); font-size: var(--text-sm); }
</style>
