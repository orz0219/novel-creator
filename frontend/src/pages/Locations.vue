<template>
  <div class="locations-page">
    <div class="page-header">
      <h1 class="page-title">地点</h1>
      <button class="btn-primary">+ 新建地点</button>
    </div>
    <div class="entity-grid">
      <EntityCard
        v-for="loc in worldStore.locations"
        :key="loc.id"
        :entity="loc"
        type="Location"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { useWorldStore } from '@/stores/world'
import EntityCard from '@/components/ui/EntityCard.vue'
const worldStore = useWorldStore()
worldStore.loadMockData()
</script>

<style scoped>
.locations-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-6); }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }
.btn-primary { padding: var(--space-2) var(--space-4); background: var(--color-primary); border: none; color: white; border-radius: var(--radius-sm); font-size: var(--text-sm); cursor: pointer; }
.btn-primary:hover { background: var(--color-primary-hover); }
.entity-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: var(--space-4); }
</style>
