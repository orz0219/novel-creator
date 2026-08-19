<template>
  <div class="rules-page">
    <div class="page-header">
      <h1 class="page-title">世界规则</h1>
      <button class="btn-primary" @click="editingRule = null; ruleForm = { rule_content: '', rule_level: 'RULE-2', enforcement: 'Allow' }; showDialog = true">+ 添加规则</button>
    </div>
    <div class="rules-content">
      <div class="rule-section">
        <h3 class="section-title">世界规则</h3>
        <div v-if="rules.length" class="rule-list">
          <div v-for="rule in rules" :key="rule.id" class="rule-item">
            <span class="rule-severity" :class="rule.enforcement === 'Reject' ? 'error' : 'info'">
              {{ rule.enforcement === 'Reject' ? 'ERROR' : 'INFO' }}
            </span>
            <span class="rule-text">{{ rule.rule_content }}</span>
            <div class="rule-actions">
              <button class="action-btn" @click="openEdit(rule)">编辑</button>
              <button class="action-btn danger" @click="handleDelete(rule.id)">删除</button>
            </div>
          </div>
        </div>
        <div v-else class="empty-hint">暂无规则，点击上方按钮添加</div>
      </div>
      <div class="rule-section">
        <h3 class="section-title">世界设定</h3>
        <div class="setting-content">
          <div class="setting-item">
            <span class="setting-key">世界名称</span>
            <span class="setting-value">{{ worldStore.currentWorld?.name || '未知' }}</span>
          </div>
          <div class="setting-item">
            <span class="setting-key">描述</span>
            <span class="setting-value">{{ worldStore.currentWorld?.description || '暂无' }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Create/Edit Dialog -->
    <NeDialog v-model="showDialog" :title="editingRule ? '编辑规则' : '添加规则'" size="md">
      <form @submit.prevent="handleSubmit" class="entity-form">
        <div class="form-group">
          <label class="form-label">规则内容 *</label>
          <textarea v-model="ruleForm.rule_content" class="form-textarea" placeholder="请输入规则内容" rows="3" required></textarea>
        </div>
        <div class="form-group">
          <label class="form-label">规则级别</label>
          <select v-model="ruleForm.rule_level" class="form-select">
            <option value="RULE-0">RULE-0 (宪法)</option>
            <option value="RULE-1">RULE-1 (核心)</option>
            <option value="RULE-2">RULE-2 (一般)</option>
            <option value="RULE-3">RULE-3 (建议)</option>
          </select>
        </div>
        <div class="form-group">
          <label class="form-label">执行级别</label>
          <select v-model="ruleForm.enforcement" class="form-select">
            <option value="Reject">拒绝 (ERROR)</option>
            <option value="Allow">允许 (INFO)</option>
            <option value="RequireApproval">需要审批</option>
          </select>
        </div>
      </form>
      <template #footer>
        <button class="btn-secondary" @click="showDialog = false">取消</button>
        <button class="btn-primary" @click="handleSubmit">
          {{ editingRule ? '更新' : '创建' }}
        </button>
      </template>
    </NeDialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useWorldStore } from '@/stores/world'
import { rulesApi, type CanonRule } from '@/api/rules'
import NeDialog from '@/components/ui/NeDialog.vue'

const worldStore = useWorldStore()
const rules = ref<CanonRule[]>([])

const showDialog = ref(false)
const editingRule = ref<CanonRule | null>(null)
const ruleForm = ref({
  rule_content: '',
  rule_level: 'RULE-2',
  enforcement: 'Allow',
})

async function loadRules() {
  const worldId = worldStore.currentWorld?.id
  if (!worldId) return
  try {
    rules.value = await rulesApi.list(worldId)
  } catch {
    rules.value = []
  }
}

onMounted(loadRules)

function openEdit(rule: CanonRule) {
  editingRule.value = rule
  ruleForm.value = {
    rule_content: rule.rule_content,
    rule_level: rule.rule_level,
    enforcement: rule.enforcement,
  }
  showDialog.value = true
}

async function handleSubmit() {
  if (!ruleForm.value.rule_content.trim()) return

  const worldId = worldStore.currentWorld?.id
  if (!worldId) return

  if (editingRule.value) {
    await rulesApi.update(editingRule.value.id, {
      rule_content: ruleForm.value.rule_content,
      rule_level: ruleForm.value.rule_level,
      enforcement: ruleForm.value.enforcement,
    })
  } else {
    await rulesApi.create(worldId, {
      rule_content: ruleForm.value.rule_content,
      rule_level: ruleForm.value.rule_level,
      enforcement: ruleForm.value.enforcement,
    })
  }
  editingRule.value = null
  showDialog.value = false
  await loadRules()
}

async function handleDelete(id: string) {
  await rulesApi.delete(id)
  await loadRules()
}
</script>

<style scoped>
.rules-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-6); }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }
.btn-primary { padding: var(--space-2) var(--space-4); background: var(--color-primary); border: none; color: white; border-radius: var(--radius-sm); font-size: var(--text-sm); cursor: pointer; }
.btn-primary:hover { background: var(--color-primary-hover); }
.rules-content { max-width: 700px; }
.rule-section { margin-bottom: var(--space-8); }
.section-title { font-size: var(--text-md); font-weight: 600; margin-bottom: var(--space-4); padding-bottom: var(--space-2); border-bottom: 1px solid var(--border-muted); }
.rule-item { display: flex; align-items: center; gap: var(--space-3); padding: var(--space-3) 0; border-bottom: 1px solid var(--border-muted); }
.rule-severity { font-size: 10px; padding: 2px 6px; border-radius: 3px; text-transform: uppercase; }
.rule-severity.error { background: var(--color-error-subtle); color: var(--color-error); }
.rule-severity.info { background: var(--color-info-subtle); color: var(--color-info); }
.rule-text { flex: 1; font-size: var(--text-sm); }
.rule-actions { display: flex; gap: var(--space-2); }
.action-btn { padding: var(--space-1) var(--space-2); border: 1px solid var(--border-default); background: transparent; color: var(--text-secondary); border-radius: var(--radius-sm); font-size: var(--text-xs); cursor: pointer; }
.action-btn:hover { background: var(--bg-hover); }
.action-btn.danger { color: var(--color-error); }
.empty-hint { padding: var(--space-4); color: var(--text-tertiary); font-size: var(--text-sm); }
.setting-item { display: flex; justify-content: space-between; padding: var(--space-3) 0; border-bottom: 1px solid var(--border-muted); }
.setting-key { font-size: var(--text-sm); color: var(--text-tertiary); }
.setting-value { font-size: var(--text-sm); color: var(--text-primary); }
.entity-form { display: flex; flex-direction: column; gap: var(--space-4); }
.form-group { display: flex; flex-direction: column; gap: var(--space-1); }
.form-label { font-size: var(--text-sm); font-weight: 500; color: var(--text-secondary); }
.form-textarea {
  padding: var(--space-2) var(--space-3);
  background: var(--bg-base);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  outline: none;
  resize: vertical;
  font-family: inherit;
}
.form-textarea:focus { border-color: var(--color-primary); }
.form-select {
  padding: var(--space-2) var(--space-3);
  background: var(--bg-base);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  outline: none;
}
.form-select:focus { border-color: var(--color-primary); }
.btn-secondary {
  padding: var(--space-2) var(--space-4);
  background: transparent;
  border: 1px solid var(--border-default);
  color: var(--text-secondary);
  border-radius: var(--radius-sm);
  font-size: var(--text-sm);
  cursor: pointer;
}
.btn-secondary:hover { border-color: var(--border-emphasis); color: var(--text-primary); }
</style>
