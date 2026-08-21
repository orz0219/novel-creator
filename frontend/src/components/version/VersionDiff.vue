<template>
  <div class="version-diff">
    <div class="panel-header">
      <span class="panel-title">版本对比</span>
    </div>

    <div v-if="loadingEntities" class="state-box">加载实体中…</div>
    <div v-else-if="!entities.length" class="state-box">暂无实体</div>

    <div v-else class="picker">
      <select v-model="selectedEntityId" class="entity-select" @change="onEntityChange">
        <option v-for="e in entities" :key="e.id" :value="e.id">
          {{ e.name }}{{ e.entity_type_id ? ' (' + e.entity_type_id + ')' : '' }}
        </option>
      </select>
    </div>

    <div v-if="selectedEntityId">
      <div v-if="loadingVersions" class="state-box">加载版本中…</div>
      <div v-else-if="!versions.length" class="state-box">该实体暂无版本</div>

      <template v-else>
        <div class="version-selectors">
          <select v-model.number="fromVersion" class="version-select" @change="onVersionChange">
            <option v-for="v in versions" :key="v.id" :value="v.version">v{{ v.version }} - {{ v.description }}</option>
          </select>
          <span class="arrow">→</span>
          <select v-model.number="toVersion" class="version-select" @change="onVersionChange">
            <option v-for="v in versions" :key="v.id" :value="v.version">v{{ v.version }} - {{ v.description }}</option>
          </select>
        </div>

        <div v-if="loadingDiff" class="state-box">对比中…</div>
        <div v-else-if="!changeFields.length" class="state-box">暂无差异</div>

        <div v-else class="diff-content">
          <div v-for="field in changeFields" :key="field" class="diff-item">
            <div class="diff-field">{{ field }}</div>
            <div class="diff-old" v-if="changes[field].old !== undefined && changes[field].old !== null">
              <span class="diff-label">旧值:</span>
              <span class="diff-value removed">{{ changes[field].old }}</span>
            </div>
            <div class="diff-new" v-if="changes[field].new !== undefined && changes[field].new !== null">
              <span class="diff-label">新值:</span>
              <span class="diff-value added">{{ changes[field].new }}</span>
            </div>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref, computed } from 'vue'
import { useRoute } from 'vue-router'
import { historyApi, type VersionEntry } from '@/api/history'
import { useWorldStore } from '@/stores/world'

const route = useRoute()
const projectId = route.params.id as string
const worldStore = useWorldStore()

const entities = computed(() => worldStore.entities)

const selectedEntityId = ref<string>('')
const versions = ref<VersionEntry[]>([])
const changes = ref<Record<string, { old: unknown; new: unknown }>>({})

const loadingEntities = ref(false)
const loadingVersions = ref(false)
const loadingDiff = ref(false)

const fromVersion = ref<number | null>(null)
const toVersion = ref<number | null>(null)

const changeFields = computed(() => Object.keys(changes.value))

async function ensureWorld() {
  if (!worldStore.currentWorld) {
    await worldStore.fetchWorld(projectId)
  }
  const worldId = worldStore.currentWorld?.id
  if (worldId && !worldStore.entities.length) {
    worldStore.entities = await worldStore.fetchEntities(worldId)
  }
}

async function onEntityChange() {
  versions.value = []
  changes.value = {}
  fromVersion.value = null
  toVersion.value = null
  const entityId = selectedEntityId.value
  if (!entityId) return
  loadingVersions.value = true
  try {
    versions.value = await historyApi.getVersions(entityId)
    if (versions.value.length >= 2) {
      fromVersion.value = versions.value[versions.value.length - 2].version
      toVersion.value = versions.value[versions.value.length - 1].version
      await runCompare()
    } else if (versions.value.length === 1) {
      fromVersion.value = versions.value[0].version
      toVersion.value = versions.value[0].version
    }
  } catch (e: any) {
    versions.value = []
  } finally {
    loadingVersions.value = false
  }
}

async function onVersionChange() {
  await runCompare()
}

async function runCompare() {
  const entityId = selectedEntityId.value
  if (!entityId || fromVersion.value === null || toVersion.value === null) return
  loadingDiff.value = true
  try {
    changes.value = await historyApi.compareVersions(entityId, fromVersion.value, toVersion.value)
  } catch (e: any) {
    changes.value = {}
  } finally {
    loadingDiff.value = false
  }
}

onMounted(async () => {
  loadingEntities.value = true
  try {
    await ensureWorld()
    if (entities.value.length) {
      selectedEntityId.value = entities.value[0].id
      await onEntityChange()
    }
  } finally {
    loadingEntities.value = false
  }
})
</script>

<style scoped>
.version-diff { display: flex; flex-direction: column; }
.panel-header { padding: var(--space-3) var(--space-4); border-bottom: 1px solid var(--border-muted); }
.panel-title { font-size: var(--text-sm); font-weight: 600; display: block; margin-bottom: var(--space-2); }
.state-box { padding: var(--space-6) var(--space-4); text-align: center; color: var(--text-tertiary); font-size: var(--text-sm); }
.picker { padding: var(--space-3) var(--space-4); }
.entity-select { width: 100%; padding: var(--space-2); background: var(--bg-base); border: 1px solid var(--border-default); border-radius: var(--radius-sm); color: var(--text-primary); font-size: var(--text-sm); }
.version-selectors { display: flex; align-items: center; gap: var(--space-2); padding: var(--space-3) var(--space-4); }
.version-select { flex: 1; padding: var(--space-1) var(--space-2); background: var(--bg-base); border: 1px solid var(--border-default); border-radius: var(--radius-sm); color: var(--text-primary); font-size: var(--text-xs); }
.arrow { color: var(--text-tertiary); }
.diff-content { padding: var(--space-3) var(--space-4); }
.diff-item { margin-bottom: var(--space-3); padding: var(--space-3); border: 1px solid var(--border-muted); border-radius: var(--radius-sm); }
.diff-field { font-size: var(--text-sm); font-weight: 600; margin-bottom: var(--space-2); font-family: var(--font-mono); }
.diff-old, .diff-new { display: flex; gap: var(--space-2); font-size: var(--text-sm); margin-bottom: var(--space-1); }
.diff-label { color: var(--text-tertiary); min-width: 40px; }
.diff-value { font-family: var(--font-mono); padding: 2px 6px; border-radius: 3px; }
.diff-value.removed { background: var(--color-error-subtle); color: var(--color-error); text-decoration: line-through; }
.diff-value.added { background: var(--color-success-subtle); color: var(--color-success); }
</style>
