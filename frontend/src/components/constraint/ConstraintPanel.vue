<template>
  <div class="constraint-panel">
    <div class="panel-header">
      <span class="panel-title">世界约束 (Constraints)</span>
      <button class="add-btn" @click="openCreate">+ 添加规则</button>
    </div>

    <div v-if="loading" class="state-hint">加载中…</div>
    <div v-else-if="!rules.length" class="state-hint">暂无约束规则</div>

    <div v-else class="constraint-list">
      <div v-for="rule in rules" :key="rule.id" class="constraint-item">
        <div v-if="editingId === rule.id" class="rule-form">
          <textarea v-model="form.rule_content" class="form-textarea" rows="2" placeholder="规则内容" required></textarea>
          <div class="form-row">
            <select v-model="form.rule_level" class="form-select">
              <option value="RULE-0">RULE-0</option>
              <option value="RULE-1">RULE-1</option>
              <option value="RULE-2">RULE-2</option>
              <option value="RULE-3">RULE-3</option>
            </select>
            <select v-model="form.enforcement" class="form-select">
              <option value="Must">Must</option>
              <option value="Should">Should</option>
              <option value="May">May</option>
              <option value="MustNot">MustNot</option>
            </select>
          </div>
          <input v-model="form.affected_scope" class="form-input" placeholder="影响范围（可选）" />
          <div class="form-actions">
            <button class="action-btn" @click="saveEdit(rule.id)">保存</button>
            <button class="action-btn" @click="cancelEdit">取消</button>
          </div>
        </div>
        <div v-else class="rule-view">
          <span class="c-badge" :class="'ef-' + rule.enforcement">{{ rule.enforcement }}</span>
          <span class="c-level">{{ rule.rule_level }}</span>
          <span class="c-text">{{ rule.rule_content }}</span>
          <span v-if="rule.affected_scope" class="c-scope">@{{ rule.affected_scope }}</span>
          <div class="rule-actions">
            <button class="action-btn" @click="openEdit(rule)">编辑</button>
            <button class="action-btn danger" @click="handleDelete(rule.id)">删除</button>
          </div>
        </div>
      </div>
    </div>

    <!-- Create form -->
    <div v-if="showCreate" class="rule-form create-form">
      <textarea v-model="form.rule_content" class="form-textarea" rows="2" placeholder="规则内容 *" required></textarea>
      <div class="form-row">
        <select v-model="form.rule_level" class="form-select">
          <option value="RULE-0">RULE-0</option>
          <option value="RULE-1">RULE-1</option>
          <option value="RULE-2">RULE-2</option>
          <option value="RULE-3">RULE-3</option>
        </select>
        <select v-model="form.enforcement" class="form-select">
          <option value="Must">Must</option>
          <option value="Should">Should</option>
          <option value="May">May</option>
          <option value="MustNot">MustNot</option>
        </select>
      </div>
      <input v-model="form.affected_scope" class="form-input" placeholder="影响范围（可选）" />
      <div class="form-actions">
        <button class="action-btn primary" :disabled="!form.rule_content.trim()" @click="createRule">创建</button>
        <button class="action-btn" @click="cancelCreate">取消</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import { useWorldStore } from '@/stores/world'
import { rulesApi, type CanonRule } from '@/api/rules'

const props = defineProps<{ worldId?: string }>()

const route = useRoute()
const worldStore = useWorldStore()
const projectId = route.params.id as string
const worldId = computed(() => props.worldId ?? worldStore.currentWorld?.id ?? '')

const rules = ref<CanonRule[]>([])
const loading = ref(false)
const showCreate = ref(false)
const editingId = ref<string | null>(null)

const emptyForm = () => ({
  rule_content: '',
  rule_level: 'RULE-2',
  enforcement: 'Must',
  affected_scope: '',
})
const form = ref(emptyForm())

async function loadRules() {
  if (!worldId.value) return
  loading.value = true
  try {
    rules.value = await rulesApi.list(worldId.value)
  } catch {
    rules.value = []
  } finally {
    loading.value = false
  }
}

function openCreate() {
  form.value = emptyForm()
  showCreate.value = true
}

function cancelCreate() {
  showCreate.value = false
  form.value = emptyForm()
}

function openEdit(rule: CanonRule) {
  editingId.value = rule.id
  form.value = {
    rule_content: rule.rule_content,
    rule_level: rule.rule_level,
    enforcement: rule.enforcement,
    affected_scope: rule.affected_scope,
  }
}

function cancelEdit() {
  editingId.value = null
  form.value = emptyForm()
}

async function createRule() {
  if (!form.value.rule_content.trim() || !worldId.value) return
  await rulesApi.create(worldId.value, {
    rule_content: form.value.rule_content.trim(),
    rule_level: form.value.rule_level,
    enforcement: form.value.enforcement,
    affected_scope: form.value.affected_scope || undefined,
  })
  cancelCreate()
  await loadRules()
}

async function saveEdit(id: string) {
  if (!form.value.rule_content.trim()) return
  await rulesApi.update(id, {
    rule_content: form.value.rule_content.trim(),
    rule_level: form.value.rule_level,
    enforcement: form.value.enforcement,
    affected_scope: form.value.affected_scope || undefined,
  })
  cancelEdit()
  await loadRules()
}

async function handleDelete(id: string) {
  await rulesApi.delete(id)
  await loadRules()
}

onMounted(async () => {
  if (!worldStore.currentWorld) await worldStore.fetchWorld(projectId)
  if (worldId.value) await loadRules()
})
</script>

<style scoped>
.constraint-panel { display: flex; flex-direction: column; }
.panel-header { display: flex; justify-content: space-between; align-items: center; padding: var(--space-3) var(--space-4); border-bottom: 1px solid var(--border-muted); }
.panel-title { font-size: var(--text-sm); font-weight: 600; }
.add-btn { padding: var(--space-1) var(--space-2); border: 1px solid var(--border-default); background: transparent; color: var(--text-secondary); border-radius: var(--radius-sm); font-size: var(--text-xs); cursor: pointer; }
.add-btn:hover { background: var(--bg-hover); }
.state-hint { padding: var(--space-4); color: var(--text-tertiary); font-size: var(--text-sm); }
.constraint-list { padding: var(--space-2); }
.constraint-item { padding: var(--space-2); border-bottom: 1px solid var(--border-muted); }
.rule-view { display: flex; align-items: center; gap: var(--space-2); flex-wrap: wrap; }
.c-badge { font-size: 10px; padding: 2px 6px; border-radius: 3px; text-transform: uppercase; flex-shrink: 0; background: var(--color-info-subtle); color: var(--color-info); }
.c-badge.ef-Must { background: var(--color-error-subtle); color: var(--color-error); }
.c-badge.ef-Should { background: var(--color-warning-subtle); color: var(--color-warning); }
.c-badge.ef-May { background: var(--color-info-subtle); color: var(--color-info); }
.c-badge.ef-MustNot { background: var(--color-error-subtle); color: var(--color-error); }
.c-level { font-size: var(--text-xs); color: var(--text-tertiary); flex-shrink: 0; }
.c-text { flex: 1; font-size: var(--text-sm); color: var(--text-secondary); min-width: 120px; }
.c-scope { font-size: var(--text-xs); color: var(--text-tertiary); }
.rule-actions { display: flex; gap: var(--space-2); margin-left: auto; }
.action-btn { padding: var(--space-1) var(--space-2); border: 1px solid var(--border-default); background: transparent; color: var(--text-secondary); border-radius: var(--radius-sm); font-size: var(--text-xs); cursor: pointer; }
.action-btn:hover { background: var(--bg-hover); }
.action-btn.danger { color: var(--color-error); }
.action-btn.primary { background: var(--color-primary); color: white; border-color: var(--color-primary); }
.action-btn.primary:hover { background: var(--color-primary-hover); }
.action-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.rule-form { display: flex; flex-direction: column; gap: var(--space-2); padding: var(--space-2); background: var(--bg-hover); border-radius: var(--radius-sm); }
.create-form { margin: var(--space-2); }
.form-row { display: flex; gap: var(--space-2); }
.form-textarea, .form-input, .form-select {
  padding: var(--space-2) var(--space-3);
  background: var(--bg-base);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  outline: none;
  font-family: inherit;
}
.form-textarea { resize: vertical; }
.form-row .form-select { flex: 1; }
.form-textarea:focus, .form-input:focus, .form-select:focus { border-color: var(--color-primary); }
.form-actions { display: flex; gap: var(--space-2); justify-content: flex-end; }
</style>
