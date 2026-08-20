<template>
  <div class="relationships-page">
    <div class="page-header">
      <h1 class="page-title">关系管理</h1>
      <button class="btn-primary" @click="openCreate">+ 新建关系</button>
    </div>
    <div v-if="worldStore.error" class="error-banner">{{ worldStore.error }}</div>
    <div class="rel-content">
      <div class="rel-filters">
        <button v-for="f in filters" :key="f.id" class="filter-btn" :class="{ active: activeFilter === f.id }" @click="activeFilter = f.id">{{ f.label }}</button>
      </div>
      <div class="rel-list">
        <div v-for="rel in filteredRelations" :key="rel.id" class="rel-card">
          <div class="rel-header">
            <span class="rel-source">{{ nameOf(rel.source_entity_id) }}</span>
            <span class="rel-arrow">→</span>
            <span class="rel-type" :class="rel.relation_type">{{ rel.relation_type }}</span>
            <span class="rel-arrow">→</span>
            <span class="rel-target">{{ nameOf(rel.target_entity_id) }}</span>
          </div>
          <div class="rel-desc" v-if="rel.description">{{ rel.description }}</div>
          <div class="rel-meta">
            <span class="rel-time">{{ formatDate(rel.updated_at) }}</span>
            <div class="rel-actions">
              <button class="action-btn" @click="openEdit(rel)">编辑</button>
              <button class="action-btn danger" @click="handleDelete(rel)">删除</button>
            </div>
          </div>
        </div>
        <div v-if="!filteredRelations.length" class="empty-state">暂无关系，点击「+ 新建关系」创建</div>
      </div>
    </div>

    <div v-if="showDialog" class="modal-mask" @click.self="showDialog = false">
      <div class="modal">
        <h3>{{ editing ? '编辑关系' : '新建关系' }}</h3>
        <label class="field">源实体
          <select v-model="form.source_entity_id">
            <option v-for="e in entityList" :key="e.id" :value="e.id">{{ e.name }}</option>
          </select>
        </label>
        <label class="field">目标实体
          <select v-model="form.target_entity_id">
            <option v-for="e in entityList" :key="e.id" :value="e.id">{{ e.name }}</option>
          </select>
        </label>
        <label class="field">关系类型
          <input v-model="form.relation_type" placeholder="如 ally / enemy / located / belongs" />
        </label>
        <label class="field">描述
          <textarea v-model="form.description" rows="3"></textarea>
        </label>
        <div class="modal-actions">
          <button class="action-btn" @click="showDialog = false">取消</button>
          <button class="btn-primary" @click="handleSave">保存</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useWorldStore } from '@/stores/world'
import type { Relation, Entity } from '@/types'

const route = useRoute()
const worldStore = useWorldStore()

const activeFilter = ref('all')
const showDialog = ref(false)
const editing = ref<Relation | null>(null)
const entityList = ref<Entity[]>([])
const form = ref({ source_entity_id: '', target_entity_id: '', relation_type: '', description: '' })

const nameMap = computed(() => {
  const m: Record<string, string> = {}
  for (const e of entityList.value) m[e.id] = e.name
  return m
})
function nameOf(id: string) { return nameMap.value[id] || id }

const filters = computed(() => {
  const types = Array.from(new Set(worldStore.relations.map(r => r.relation_type)))
  return [{ id: 'all', label: '全部' }, ...types.map(t => ({ id: t, label: t }))]
})
const filteredRelations = computed(() => {
  if (activeFilter.value === 'all') return worldStore.relations
  return worldStore.relations.filter(r => r.relation_type === activeFilter.value)
})

async function ensureWorld(): Promise<string | undefined> {
  const projectId = route.params.id as string
  if (!worldStore.currentWorld) await worldStore.fetchWorld(projectId)
  return worldStore.currentWorld?.id
}

onMounted(async () => {
  const worldId = await ensureWorld()
  if (!worldId) return
  const [ents] = await Promise.all([
    worldStore.fetchEntities(worldId),
    worldStore.fetchRelations(worldId),
  ])
  entityList.value = ents
})

function openCreate() {
  editing.value = null
  const first = entityList.value[0]?.id || ''
  form.value = { source_entity_id: first, target_entity_id: first, relation_type: '', description: '' }
  showDialog.value = true
}
function openEdit(rel: Relation) {
  editing.value = rel
  form.value = {
    source_entity_id: rel.source_entity_id,
    target_entity_id: rel.target_entity_id,
    relation_type: rel.relation_type,
    description: rel.description || '',
  }
  showDialog.value = true
}

async function handleSave() {
  const worldId = worldStore.currentWorld?.id
  if (!worldId) return
  const data = { ...form.value }
  if (editing.value) {
    // 后端无 PUT，编辑以「删后建」实现
    await worldStore.deleteRelation(editing.value.id)
  }
  await worldStore.createRelation(worldId, data)
  showDialog.value = false
  editing.value = null
}

async function handleDelete(rel: Relation) {
  if (!confirm('确认删除该关系？')) return
  await worldStore.deleteRelation(rel.id)
}

function formatDate(d?: string) {
  if (!d) return ''
  return new Date(d).toLocaleDateString('zh-CN', { month: 'short', day: 'numeric' })
}
</script>

<style scoped>
.relationships-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-6); }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }
.btn-primary { padding: var(--space-2) var(--space-4); background: var(--color-primary); border: none; color: white; border-radius: var(--radius-sm); font-size: var(--text-sm); cursor: pointer; }
.btn-primary:hover { background: var(--color-primary-hover); }
.rel-content { }
.rel-filters { display: flex; gap: var(--space-2); margin-bottom: var(--space-4); flex-wrap: wrap; }
.filter-btn { padding: var(--space-1) var(--space-3); border: 1px solid var(--border-default); background: transparent; color: var(--text-secondary); border-radius: var(--radius-sm); font-size: var(--text-xs); cursor: pointer; }
.filter-btn.active { background: var(--color-primary-subtle); border-color: var(--color-primary); color: var(--color-primary-text); }
.rel-list { display: flex; flex-direction: column; gap: var(--space-3); }
.rel-card { padding: var(--space-4); border: 1px solid var(--border-default); border-radius: var(--radius-md); background: var(--bg-panel); }
.rel-header { display: flex; align-items: center; gap: var(--space-2); margin-bottom: var(--space-2); }
.rel-source, .rel-target { font-weight: 500; }
.rel-arrow { color: var(--text-tertiary); }
.rel-type { font-size: var(--text-xs); padding: 2px 8px; border-radius: 10px; background: var(--color-info-subtle); color: var(--color-info); }
.rel-desc { font-size: var(--text-sm); color: var(--text-secondary); margin-bottom: var(--space-2); }
.rel-meta { display: flex; justify-content: space-between; align-items: center; }
.rel-time { font-size: var(--text-xs); color: var(--text-tertiary); }
.rel-actions { display: flex; gap: var(--space-2); }
.action-btn { padding: var(--space-1) var(--space-2); border: 1px solid var(--border-default); background: transparent; color: var(--text-secondary); border-radius: var(--radius-sm); font-size: var(--text-xs); cursor: pointer; }
.action-btn:hover { background: var(--bg-hover); }
.action-btn.danger { color: var(--color-error); border-color: var(--color-error); }
.action-btn.danger:hover { background: var(--color-error-subtle); }
.empty-state { padding: var(--space-8); text-align: center; color: var(--text-tertiary); font-size: var(--text-sm); }
.error-banner { padding: var(--space-3) var(--space-4); background: var(--color-error-subtle); color: var(--color-error); border-radius: var(--radius-sm); margin-bottom: var(--space-4); font-size: var(--text-sm); }
.modal-mask { position: fixed; inset: 0; background: rgba(0,0,0,0.4); display: flex; align-items: center; justify-content: center; z-index: 100; }
.modal { background: var(--bg-panel); padding: var(--space-6); border-radius: var(--radius-md); width: 420px; max-width: 90vw; display: flex; flex-direction: column; gap: var(--space-3); }
.modal h3 { font-size: var(--text-lg); margin-bottom: var(--space-2); }
.field { display: flex; flex-direction: column; gap: var(--space-1); font-size: var(--text-sm); color: var(--text-secondary); }
.field select, .field input, .field textarea { padding: var(--space-2); background: var(--bg-base); border: 1px solid var(--border-default); border-radius: var(--radius-sm); color: var(--text-primary); font-size: var(--text-sm); }
.modal-actions { display: flex; justify-content: flex-end; gap: var(--space-2); margin-top: var(--space-2); }
</style>
