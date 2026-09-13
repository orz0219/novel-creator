<template>
  <div class="golden-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">金手指</h1>
        <span v-if="fingers.length" class="header-count">{{ fingers.length }} 个</span>
      </div>
      <button class="btn-primary" @click="openCreate">+ 新建金手指</button>
    </div>

    <div v-if="worldStore.error" class="error-banner">{{ worldStore.error }}</div>

    <!-- 只有一个金手指时不需要切换条：绝大多数书就是唯一一个 -->
    <nav v-if="fingers.length > 1" class="finger-tabs">
      <button
        v-for="f in fingers"
        :key="f.id"
        class="finger-tab"
        :class="{ 'is-active': current?.id === f.id }"
        @click="current = f"
      >
        {{ f.name }}
      </button>
    </nav>

    <GoldenFingerPanel
      v-if="current"
      :key="current.id"
      :finger="current"
      @edit="openEdit"
      @delete="handleDelete"
    />

    <div v-else class="empty-state">
      <Sparkles class="empty-icon" :size="40" />
      <span class="empty-text">
        还没有金手指。点击右上角新建，或直接在对话里让 AI 帮你设计——它会用
        <code>update_golden_finger</code> 把机制、代价、约束、成长阶段一起落库。
      </span>
    </div>

    <NeDialog v-model="showDialog" :title="dialogTitle" size="md">
      <form @submit.prevent="handleSubmit" class="entity-form">
        <div class="form-group">
          <label class="form-label">名称 *</label>
          <input v-model="form.name" class="form-input" placeholder="请输入金手指名称" required />
        </div>
        <div class="form-group">
          <label class="form-label">摘要</label>
          <input v-model="form.summary" class="form-input" placeholder="请输入金手指摘要" />
        </div>
        <div class="form-group">
          <label class="form-label">原始描述</label>
          <textarea
            v-model="form.description"
            class="form-textarea"
            placeholder="完整设定文本（会展示在面板底部的「原始设定文本」里）"
            rows="6"
          ></textarea>
        </div>

        <details class="advanced">
          <summary>高级：直接编辑结构化档案（JSON）</summary>
          <div class="form-group">
            <textarea
              v-model="attributesText"
              class="form-textarea attributes-editor"
              placeholder="{}"
              rows="10"
            ></textarea>
            <span class="form-help">
              对应面板上的机制 / 约束 / 成长阶段。一般不需要手改——让 AI 用
              <code>update_golden_finger</code> 填写更省事，手改需要严格符合结构。
            </span>
          </div>
        </details>

        <div v-if="error" class="form-error">{{ error }}</div>
      </form>
      <template #footer>
        <button class="btn-secondary" @click="closeDialog">取消</button>
        <button class="btn-primary" :disabled="submitting" @click="handleSubmit">
          {{ submitting ? '保存中...' : isEditing ? '更新' : '创建' }}
        </button>
      </template>
    </NeDialog>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useWorldStore } from '@/stores/world'
import { entityApi } from '@/api/world'
import GoldenFingerPanel from '@/components/goldenFinger/GoldenFingerPanel.vue'
import NeDialog from '@/components/ui/NeDialog.vue'
import type { Entity } from '@/types'
import { Sparkles } from 'lucide-vue-next'

const worldStore = useWorldStore()
const fingers = ref<Entity[]>([])
const current = ref<Entity | null>(null)

const showDialog = ref(false)
const editingEntity = ref<Entity | null>(null)

const attributesText = ref('{}')
const error = ref('')
const submitting = ref(false)

const isEditing = ref(false)
const dialogTitle = ref('新建金手指')

const form = ref({
  name: '',
  summary: '',
  description: '',
})

async function loadFingers() {
  const worldId = worldStore.currentWorld?.id
  if (!worldId) return
  worldStore.error = ''
  try {
    fingers.value = await worldStore.fetchEntities(worldId, 'golden_finger')
  } catch (e) {
    worldStore.error = `加载金手指失败：${(e as Error).message}`
    return
  }
  // 保持当前选中；若它已被删除或首次加载，则回落到第一个
  const keepId = current.value?.id
  current.value = fingers.value.find((f) => f.id === keepId) ?? fingers.value[0] ?? null
}

// ProjectLayout 异步解析 currentWorld；子页面 onMounted 可能早于 world 就绪，
// 用 watch 在 world 可用后再加载，兼顾深链直达与本页导航两种场景。
watch(() => worldStore.currentWorld?.id, (id) => { if (id) loadFingers() }, { immediate: true })

function openCreate() {
  resetForm()
  dialogTitle.value = '新建金手指'
  showDialog.value = true
}

function openEdit(entity: Entity) {
  editingEntity.value = entity
  isEditing.value = true
  dialogTitle.value = '编辑金手指'
  form.value = {
    name: entity.name || '',
    summary: entity.summary || '',
    description: entity.description || '',
  }
  attributesText.value = JSON.stringify((entity.attributes as Record<string, unknown>) ?? {}, null, 2)
  error.value = ''
  showDialog.value = true
}

function resetForm() {
  editingEntity.value = null
  isEditing.value = false
  form.value = { name: '', summary: '', description: '' }
  attributesText.value = '{}'
  error.value = ''
}

function closeDialog() {
  showDialog.value = false
  resetForm()
}

async function handleSubmit() {
  const worldId = worldStore.currentWorld?.id
  if (!worldId) {
    error.value = '未找到世界数据'
    return
  }
  if (!form.value.name.trim()) {
    error.value = '请输入名称'
    return
  }

  let parsed: Record<string, unknown> = {}
  try {
    parsed = JSON.parse(attributesText.value || '{}') as Record<string, unknown>
  } catch (e) {
    error.value = `结构化档案 JSON 格式错误：${(e as Error).message}`
    return
  }

  submitting.value = true
  error.value = ''
  try {
    if (editingEntity.value) {
      await entityApi.update(editingEntity.value.id, {
        name: form.value.name.trim(),
        summary: form.value.summary.trim() || undefined,
        description: form.value.description.trim() || undefined,
        attributes: parsed,
      })
    } else {
      await worldStore.createEntity(worldId, {
        name: form.value.name.trim(),
        summary: form.value.summary.trim() || undefined,
        description: form.value.description.trim() || undefined,
      })
    }
    showDialog.value = false
    resetForm()
    await loadFingers()
  } catch (e) {
    error.value = (e as Error).message || '操作失败'
  } finally {
    submitting.value = false
  }
}

async function handleDelete(entity: Entity) {
  if (!confirm(`确认删除「${entity.name}」？此操作不可撤销。`)) return
  worldStore.error = ''
  try {
    await worldStore.deleteEntity(entity.id)
  } catch (e) {
    worldStore.error = `删除金手指失败：${(e as Error).message}`
    return
  }
  // 删掉的正是当前选中项时，loadFingers 会自动回落到下一个
  current.value = null
  await loadFingers()
}
</script>

<style scoped>
.golden-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-5); }
.header-left { display: flex; align-items: baseline; gap: var(--space-3); }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }
.header-count { font-size: var(--text-xs); color: var(--text-tertiary); }
.btn-primary { padding: var(--space-2) var(--space-4); background: var(--color-primary); border: none; color: white; border-radius: var(--radius-sm); font-size: var(--text-sm); cursor: pointer; }
.btn-primary:hover { background: var(--color-primary-hover); }
.btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
.btn-secondary { padding: var(--space-2) var(--space-4); background: transparent; border: 1px solid var(--border-default); color: var(--text-secondary); border-radius: var(--radius-sm); font-size: var(--text-sm); cursor: pointer; }
.btn-secondary:hover { border-color: var(--border-emphasis); color: var(--text-primary); }

.error-banner { padding: var(--space-3) var(--space-4); background: var(--color-error-subtle); color: var(--color-error); border-radius: var(--radius-sm); margin-bottom: var(--space-4); font-size: var(--text-sm); }

.finger-tabs { display: flex; flex-wrap: wrap; gap: var(--space-2); margin-bottom: var(--space-5); }
.finger-tab {
  padding: var(--space-1) var(--space-3);
  border: 1px solid var(--border-default);
  border-radius: 999px;
  background: var(--bg-panel);
  color: var(--text-secondary);
  font-size: var(--text-sm);
  cursor: pointer;
  transition: all var(--transition-fast);
}
.finger-tab:hover { border-color: var(--border-emphasis); color: var(--text-primary); }
.finger-tab.is-active {
  border-color: var(--color-primary);
  background: var(--color-primary-subtle);
  color: var(--color-primary-text);
}

.empty-state { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: var(--space-4); padding: var(--space-16); color: var(--text-tertiary); text-align: center; }
.empty-icon { color: var(--text-tertiary); }
.empty-text { font-size: var(--text-sm); line-height: var(--leading-relaxed); max-width: 460px; }
.empty-text code { padding: 1px 5px; border-radius: 3px; background: var(--bg-panel-tertiary); font-family: var(--font-mono); font-size: var(--text-xs); color: var(--text-secondary); }

.entity-form { display: flex; flex-direction: column; gap: var(--space-4); }
.form-group { display: flex; flex-direction: column; gap: var(--space-1); }
.form-label { font-size: var(--text-sm); font-weight: 500; color: var(--text-secondary); }
.form-input {
  padding: var(--space-2) var(--space-3);
  background: var(--bg-base);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  outline: none;
  transition: border-color var(--transition-fast);
}
.form-input:focus { border-color: var(--color-primary); }
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
  transition: border-color var(--transition-fast);
}
.form-textarea:focus { border-color: var(--color-primary); }
.attributes-editor { font-family: var(--font-mono); }
.form-help { font-size: var(--text-xs); color: var(--text-tertiary); line-height: var(--leading-normal); }
.form-help code { font-family: var(--font-mono); }
.form-error { color: var(--color-error); font-size: var(--text-xs); padding: var(--space-2); background: var(--color-error-subtle); border-radius: var(--radius-sm); }

.advanced { border: 1px solid var(--border-default); border-radius: var(--radius-sm); padding: var(--space-2) var(--space-3); }
.advanced summary { cursor: pointer; font-size: var(--text-sm); color: var(--text-tertiary); list-style: none; }
.advanced summary::-webkit-details-marker { display: none; }
.advanced summary:hover { color: var(--text-secondary); }
.advanced[open] summary { margin-bottom: var(--space-3); }
</style>
