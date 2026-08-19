<template>
  <div class="items-page">
    <div class="page-header">
      <h1 class="page-title">物品</h1>
      <button class="btn-primary" @click="showCreateDialog = true">+ 新建物品</button>
    </div>
    <div v-if="items.length" class="entity-grid">
      <EntityCard
        v-for="item in items"
        :key="item.id"
        :entity="item"
        type="Item"
        @click="openEdit(item)"
      />
    </div>
    <div v-else class="empty-state">
      <span class="empty-icon">📦</span>
      <span class="empty-text">暂无物品，点击上方按钮创建</span>
    </div>

    <EntityDialog
      v-model="showDialog"
      :entity-type="'物品'"
      :edit-data="editingEntity"
      @submit="handleSubmit"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useWorldStore } from '@/stores/world'
import EntityCard from '@/components/ui/EntityCard.vue'
import EntityDialog from '@/components/ui/EntityDialog.vue'
import type { Entity } from '@/types'

const route = useRoute()
const worldStore = useWorldStore()
const items = ref<Entity[]>([])

const showCreateDialog = ref(false)
const showDialog = ref(false)
const editingEntity = ref<Entity | null>(null)

async function loadItems() {
  const worldId = worldStore.currentWorld?.id
  if (!worldId) return
  items.value = await worldStore.fetchEntities(worldId, 'Item')
}

onMounted(loadItems)

function openEdit(entity: Entity) {
  editingEntity.value = entity
  showDialog.value = true
}

async function handleSubmit(data: { name: string; summary?: string; description?: string }) {
  const worldId = worldStore.currentWorld?.id
  if (!worldId) return

  if (editingEntity.value) {
    await worldStore.updateCharacter(editingEntity.value.id, data) // generic entity update
  } else {
    await worldStore.createEntity(worldId, data)
  }
  editingEntity.value = null
  await loadItems()
}

watch(showCreateDialog, (v) => {
  if (v) {
    editingEntity.value = null
    showDialog.value = true
    showCreateDialog.value = false
  }
})
</script>

<style scoped>
.items-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-6); }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }
.btn-primary { padding: var(--space-2) var(--space-4); background: var(--color-primary); border: none; color: white; border-radius: var(--radius-sm); font-size: var(--text-sm); cursor: pointer; }
.btn-primary:hover { background: var(--color-primary-hover); }
.entity-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: var(--space-4); }
.empty-state { display: flex; flex-direction: column; align-items: center; justify-content: center; padding: var(--space-16); color: var(--text-tertiary); }
.empty-icon { font-size: 48px; margin-bottom: var(--space-4); }
.empty-text { font-size: var(--text-sm); }
</style>
