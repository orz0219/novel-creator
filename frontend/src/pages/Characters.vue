<template>
  <div class="characters-page">
    <div class="page-header">
      <h1 class="page-title">人物</h1>
      <button class="btn-primary" @click="showCreateDialog = true">+ 新建人物</button>
    </div>
    <div v-if="worldStore.error" class="error-banner">{{ worldStore.error }}</div>
    <div v-if="worldStore.characters.length" class="entity-grid">
      <EntityCard
        v-for="char in worldStore.characters"
        :key="char.id"
        :entity="char"
        type="Character"
        @click="openEdit(char)"
      />
    </div>
    <div v-else class="empty-state">
      <span class="empty-icon">👤</span>
      <span class="empty-text">暂无人物，点击上方按钮创建</span>
    </div>

    <!-- Create/Edit Dialog -->
    <EntityDialog
      v-model="showDialog"
      :entity-type="'人物'"
      :edit-data="editingEntity"
      @submit="handleSubmit"
    />
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useRoute } from 'vue-router'
import { useWorldStore } from '@/stores/world'
import EntityCard from '@/components/ui/EntityCard.vue'
import EntityDialog from '@/components/ui/EntityDialog.vue'
import type { Entity } from '@/types'

const route = useRoute()
const worldStore = useWorldStore()

const showCreateDialog = ref(false)
const showDialog = ref(false)
const editingEntity = ref<Entity | null>(null)

function openEdit(entity: Entity) {
  editingEntity.value = entity
  showDialog.value = true
}

async function handleSubmit(data: { name: string; summary?: string; description?: string }) {
  const worldId = worldStore.currentWorld?.id
  if (!worldId) {
    worldStore.error = '未找到世界数据'
    return
  }

  if (editingEntity.value) {
    await worldStore.updateCharacter(editingEntity.value.id, data)
  } else {
    await worldStore.createCharacter(worldId, data)
  }
  editingEntity.value = null
}

// Watch for create dialog opening
import { watch } from 'vue'
watch(showCreateDialog, (v) => {
  if (v) {
    editingEntity.value = null
    showDialog.value = true
    showCreateDialog.value = false
  }
})
</script>

<style scoped>
.characters-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
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
.entity-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: var(--space-4); }
.empty-state { display: flex; flex-direction: column; align-items: center; justify-content: center; padding: var(--space-16); color: var(--text-tertiary); }
.empty-icon { font-size: 48px; margin-bottom: var(--space-4); }
.empty-text { font-size: var(--text-sm); }
.error-banner { padding: var(--space-3) var(--space-4); background: var(--color-error-subtle); color: var(--color-error); border-radius: var(--radius-sm); margin-bottom: var(--space-4); font-size: var(--text-sm); }
</style>
