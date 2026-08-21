<template>
  <div class="world-page">
    <div class="page-header">
      <h1 class="page-title">世界总览</h1>
      <button v-if="worldStore.currentWorld" class="btn-primary" @click="openEdit" :disabled="saving">
        编辑
      </button>
    </div>

    <div v-if="worldStore.error" class="error-banner">{{ worldStore.error }}</div>

    <div v-if="worldStore.loading" class="empty-state">
      <span class="empty-icon">⏳</span>
      <span class="empty-text">加载中…</span>
    </div>

    <template v-else-if="worldStore.currentWorld">
      <div class="world-detail">
        <div class="world-detail-header">
          <h2 class="world-name">{{ worldStore.currentWorld.name }}</h2>
          <span class="world-badge" :class="worldStore.currentWorld.is_main ? 'main' : 'sub'">
            {{ worldStore.currentWorld.is_main ? '主世界' : '子世界' }}
          </span>
        </div>

        <div class="detail-rows">
          <div class="detail-row">
            <span class="detail-label">世界 ID</span>
            <span class="detail-value">{{ worldStore.currentWorld.id }}</span>
          </div>
          <div class="detail-row">
            <span class="detail-label">项目 ID</span>
            <span class="detail-value">{{ worldStore.currentWorld.project_id }}</span>
          </div>
          <div class="detail-row">
            <span class="detail-label">是否主世界</span>
            <span class="detail-value">{{ worldStore.currentWorld.is_main ? '是' : '否' }}</span>
          </div>
          <div v-if="worldStore.currentWorld.description" class="detail-row">
            <span class="detail-label">描述</span>
            <span class="detail-value">{{ worldStore.currentWorld.description }}</span>
          </div>
          <div v-if="worldStore.currentWorld.world_rules" class="detail-row">
            <span class="detail-label">世界规则</span>
            <span class="detail-value">{{ worldStore.currentWorld.world_rules }}</span>
          </div>
          <div v-if="configKeys.length" class="detail-row">
            <span class="detail-label">配置</span>
            <span class="detail-value config-value">{{ configPreview }}</span>
          </div>
        </div>
      </div>

      <div class="world-stats">
        <div class="stat-card" @click="$router.push('/project/' + route.params.id + '/world/characters')">
          <span class="stat-icon">👤</span>
          <span class="stat-value">{{ worldStore.characters.length }}</span>
          <span class="stat-label">人物</span>
        </div>
        <div class="stat-card" @click="$router.push('/project/' + route.params.id + '/world/locations')">
          <span class="stat-icon">📍</span>
          <span class="stat-value">{{ worldStore.locations.length }}</span>
          <span class="stat-label">地点</span>
        </div>
        <div class="stat-card" @click="$router.push('/project/' + route.params.id + '/world/factions')">
          <span class="stat-icon">⚔️</span>
          <span class="stat-value">{{ worldStore.factions.length }}</span>
          <span class="stat-label">势力</span>
        </div>
        <div class="stat-card" @click="$router.push('/project/' + route.params.id + '/world/timeline')">
          <span class="stat-icon">📅</span>
          <span class="stat-value">{{ worldStore.events.length }}</span>
          <span class="stat-label">事件</span>
        </div>
      </div>

      <div class="world-grid">
        <!-- Characters -->
        <div class="panel">
          <div class="panel-header">
            <h3 class="panel-title">人物</h3>
            <router-link :to="`/project/${route.params.id}/world/characters`" class="panel-link">查看全部 →</router-link>
          </div>
          <div class="entity-list">
            <div v-for="char in worldStore.characters" :key="char.id" class="entity-row">
              <span class="entity-name">{{ char.name }}</span>
              <span class="entity-summary">{{ char.summary }}</span>
              <span class="entity-meta">{{ char.attributes?.identity }}</span>
            </div>
          </div>
        </div>

        <!-- Locations -->
        <div class="panel">
          <div class="panel-header">
            <h3 class="panel-title">地点</h3>
            <router-link :to="`/project/${route.params.id}/world/locations`" class="panel-link">查看全部 →</router-link>
          </div>
          <div class="entity-list">
            <div v-for="loc in worldStore.locations" :key="loc.id" class="entity-row">
              <span class="entity-name">{{ loc.name }}</span>
              <span class="entity-summary">{{ loc.summary }}</span>
              <span class="entity-meta">{{ loc.attributes?.type }}</span>
            </div>
          </div>
        </div>

        <!-- Factions -->
        <div class="panel">
          <div class="panel-header">
            <h3 class="panel-title">势力</h3>
            <router-link :to="`/project/${route.params.id}/world/factions`" class="panel-link">查看全部 →</router-link>
          </div>
          <div class="entity-list">
            <div v-for="fac in worldStore.factions" :key="fac.id" class="entity-row">
              <span class="entity-name">{{ fac.name }}</span>
              <span class="entity-summary">{{ fac.summary }}</span>
              <span class="entity-meta">{{ fac.attributes?.leader }}</span>
            </div>
          </div>
        </div>

        <!-- Facts -->
        <div class="panel">
          <div class="panel-header">
            <h3 class="panel-title">世界事实</h3>
          </div>
          <div class="fact-list">
            <div v-for="fact in worldStore.facts" :key="fact.id" class="fact-row">
              <span class="fact-certainty" :class="fact.certainty.toLowerCase()">{{ fact.certainty }}</span>
              <span class="fact-content">{{ fact.content }}</span>
            </div>
          </div>
        </div>
      </div>
    </template>

    <div v-else class="empty-state">
      <span class="empty-icon">🌍</span>
      <span class="empty-text">暂无世界数据</span>
    </div>

    <!-- Edit Dialog -->
    <div v-if="showEdit" class="dialog-overlay" @click.self="closeEdit">
      <div class="dialog">
        <div class="dialog-header">
          <h3 class="dialog-title">编辑世界</h3>
          <button class="dialog-close" @click="closeEdit">×</button>
        </div>
        <div class="dialog-body">
          <label class="field">
            <span class="field-label">名称</span>
            <input v-model="editForm.name" class="field-input" type="text" />
          </label>
          <label class="field">
            <span class="field-label">描述</span>
            <textarea v-model="editForm.description" class="field-input" rows="3"></textarea>
          </label>
          <label class="field">
            <span class="field-label">世界规则</span>
            <textarea v-model="editForm.world_rules" class="field-input" rows="4"></textarea>
          </label>
          <label class="field field-inline">
            <input v-model="editForm.is_main" type="checkbox" />
            <span class="field-label">主世界</span>
          </label>
        </div>
        <div class="dialog-footer">
          <button class="btn-secondary" @click="closeEdit" :disabled="saving">取消</button>
          <button class="btn-primary" @click="save" :disabled="saving">
            {{ saving ? '保存中…' : '保存' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useRoute } from 'vue-router'
import { useWorldStore } from '@/stores/world'
import { worldApi } from '@/api/world'
import { ref, onMounted, computed } from 'vue'

const route = useRoute()
const worldStore = useWorldStore()
const projectId = route.params.id as string
const worldId = computed(() => worldStore.currentWorld?.id ?? '')

const showEdit = ref(false)
const saving = ref(false)
const editForm = ref({
  name: '',
  description: '',
  world_rules: '',
  is_main: false,
})

const configKeys = computed(() =>
  worldStore.currentWorld ? Object.keys(worldStore.currentWorld.config ?? {}) : []
)
const configPreview = computed(() => {
  const config = worldStore.currentWorld?.config
  if (!config) return ''
  return JSON.stringify(config, null, 2)
})

function openEdit() {
  const w = worldStore.currentWorld
  if (!w) return
  editForm.value = {
    name: w.name,
    description: w.description ?? '',
    world_rules: w.world_rules ?? '',
    is_main: w.is_main,
  }
  showEdit.value = true
}

function closeEdit() {
  if (saving.value) return
  showEdit.value = false
}

async function save() {
  if (!worldId.value) return
  saving.value = true
  try {
    await worldApi.update(worldId.value, {
      name: editForm.value.name,
      description: editForm.value.description || undefined,
      world_rules: editForm.value.world_rules || undefined,
      is_main: editForm.value.is_main,
    })
    await worldStore.fetchWorld(projectId)
    showEdit.value = false
  } catch (e: any) {
    worldStore.error = e?.message ?? '保存失败'
  } finally {
    saving.value = false
  }
}

onMounted(async () => {
  if (!worldStore.currentWorld) await worldStore.fetchWorld(projectId)
})
</script>

<style scoped>
.world-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-6); }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }

.btn-primary {
  padding: var(--space-2) var(--space-4);
  background: var(--color-primary);
  border: none;
  color: white;
  border-radius: var(--radius-sm);
  font-size: var(--text-sm);
  cursor: pointer;
}
.btn-primary:hover { background: var(--color-primary-hover); }
.btn-primary:disabled { opacity: 0.6; cursor: not-allowed; }
.btn-secondary {
  padding: var(--space-2) var(--space-4);
  background: var(--bg-panel);
  border: 1px solid var(--border-default);
  color: var(--text-secondary);
  border-radius: var(--radius-sm);
  font-size: var(--text-sm);
  cursor: pointer;
}
.btn-secondary:hover { border-color: var(--color-primary); }
.btn-secondary:disabled { opacity: 0.6; cursor: not-allowed; }

.error-banner { padding: var(--space-3) var(--space-4); background: var(--color-error-subtle); color: var(--color-error); border-radius: var(--radius-sm); margin-bottom: var(--space-4); font-size: var(--text-sm); }

.empty-state { display: flex; flex-direction: column; align-items: center; justify-content: center; padding: var(--space-16); color: var(--text-tertiary); }
.empty-icon { font-size: 48px; margin-bottom: var(--space-4); }
.empty-text { font-size: var(--text-sm); }

.world-detail {
  padding: var(--space-5) var(--space-6);
  background: var(--bg-panel);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  margin-bottom: var(--space-6);
}
.world-detail-header { display: flex; align-items: center; gap: var(--space-3); margin-bottom: var(--space-4); }
.world-name { font-size: var(--text-xl); font-weight: 700; font-family: var(--font-serif); }
.world-badge { font-size: var(--text-xs); padding: 2px 10px; border-radius: 999px; }
.world-badge.main { background: var(--color-primary-subtle); color: var(--color-primary-text); }
.world-badge.sub { background: var(--color-info-subtle); color: var(--color-info); }

.detail-rows { display: flex; flex-direction: column; gap: var(--space-2); }
.detail-row { display: flex; gap: var(--space-4); font-size: var(--text-sm); line-height: 1.6; }
.detail-label { flex: 0 0 90px; color: var(--text-tertiary); }
.detail-value { flex: 1; color: var(--text-primary); white-space: pre-wrap; }
.config-value { font-family: var(--font-mono, monospace); font-size: var(--text-xs); }

.world-stats {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: var(--space-3);
  margin-bottom: var(--space-6);
}

.stat-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-5);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  background: var(--bg-panel);
  cursor: pointer;
  transition: all var(--transition-fast);
}
.stat-card:hover { border-color: var(--color-primary); background: var(--color-primary-subtle); }
.stat-icon { font-size: 24px; }
.stat-value { font-size: var(--text-2xl); font-weight: 700; }
.stat-label { font-size: var(--text-xs); color: var(--text-tertiary); }

.world-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: var(--space-4);
}

.panel { border: 1px solid var(--border-default); border-radius: var(--radius-md); background: var(--bg-panel); overflow: hidden; }
.panel-header { display: flex; align-items: center; justify-content: space-between; padding: var(--space-3) var(--space-4); border-bottom: 1px solid var(--border-muted); }
.panel-title { font-size: var(--text-md); font-weight: 600; }
.panel-link { font-size: var(--text-xs); color: var(--color-primary-text); text-decoration: none; }

.entity-list { padding: var(--space-2) var(--space-4); }
.entity-row { display: flex; align-items: center; gap: var(--space-3); padding: var(--space-2) 0; border-bottom: 1px solid var(--border-muted); }
.entity-name { font-weight: 500; min-width: 80px; }
.entity-summary { font-size: var(--text-sm); color: var(--text-secondary); flex: 1; }
.entity-meta { font-size: var(--text-xs); color: var(--text-tertiary); }

.fact-list { padding: var(--space-3) var(--space-4); }
.fact-row { display: flex; gap: var(--space-3); padding: var(--space-2) 0; border-bottom: 1px solid var(--border-muted); font-size: var(--text-sm); }
.fact-certainty { font-size: 10px; padding: 2px 6px; border-radius: 3px; flex-shrink: 0; }
.fact-certainty.confirmed { background: var(--color-success-subtle); color: var(--color-success); }
.fact-certainty.likely { background: var(--color-info-subtle); color: var(--color-info); }
.fact-certainty.rumor { background: var(--color-warning-subtle); color: var(--color-warning); }
.fact-content { color: var(--text-secondary); }

.dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}
.dialog {
  width: 520px;
  max-width: 92vw;
  max-height: 90vh;
  overflow-y: auto;
  background: var(--color-surface);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-md, 0 8px 24px rgba(0, 0, 0, 0.2));
}
.dialog-header { display: flex; align-items: center; justify-content: space-between; padding: var(--space-4); border-bottom: 1px solid var(--border-muted); }
.dialog-title { font-size: var(--text-md); font-weight: 600; }
.dialog-close { background: none; border: none; font-size: 22px; line-height: 1; color: var(--text-tertiary); cursor: pointer; }
.dialog-body { padding: var(--space-4); display: flex; flex-direction: column; gap: var(--space-4); }
.field { display: flex; flex-direction: column; gap: var(--space-2); }
.field-inline { flex-direction: row; align-items: center; gap: var(--space-2); }
.field-label { font-size: var(--text-sm); color: var(--text-secondary); }
.field-input {
  padding: var(--space-2) var(--space-3);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm);
  background: var(--bg-panel);
  color: var(--text-primary);
  font-size: var(--text-sm);
  font-family: inherit;
  resize: vertical;
}
.field-input:focus { outline: none; border-color: var(--color-primary); }
.dialog-footer { display: flex; justify-content: flex-end; gap: var(--space-3); padding: var(--space-4); border-top: 1px solid var(--border-muted); }
</style>
