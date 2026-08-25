<template>
  <div class="app-layout">
    <main class="app-main"><RouterView /></main>
    <CommandPalette v-if="uiStore.commandPaletteOpen" @close="uiStore.closeCommandPalette()" />
    <Toast />
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { useUiStore } from '@/stores/ui'
import CommandPalette from '@/components/ui/CommandPalette.vue'
import Toast from '@/components/ui/Toast.vue'
const uiStore = useUiStore()
function handleKeydown(e: KeyboardEvent) {
  if ((e.metaKey || e.ctrlKey) && e.key === 'k') { e.preventDefault(); uiStore.openCommandPalette() }
  if (e.key === 'Escape' && uiStore.commandPaletteOpen) uiStore.closeCommandPalette()
}
onMounted(() => document.addEventListener('keydown', handleKeydown))
onUnmounted(() => document.removeEventListener('keydown', handleKeydown))
</script>

<style scoped>
.app-layout { display: flex; flex-direction: column; height: 100vh; width: 100vw; overflow: hidden; }
.app-main { flex: 1; overflow: hidden; min-height: 0; }
</style>
