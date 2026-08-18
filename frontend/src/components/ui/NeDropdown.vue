<template>
  <div class="ne-dropdown" ref="dropdownRef">
    <div @click="toggle" class="dropdown-trigger">
      <slot name="trigger" />
    </div>
    <Transition name="dropdown">
      <div v-if="open" class="dropdown-menu" :class="align">
        <div
          v-for="item in items"
          :key="item.id"
          class="dropdown-item"
          :class="{ danger: item.danger, disabled: item.disabled }"
          @click="select(item)"
        >
          <span class="item-icon" v-if="item.icon">{{ item.icon }}</span>
          <span class="item-label">{{ item.label }}</span>
        </div>
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
const props = defineProps<{
  items: { id: string; label: string; icon?: string; danger?: boolean; disabled?: boolean }[]
  align?: 'left' | 'right'
}>()
const emit = defineEmits(['select'])
const open = ref(false)
const dropdownRef = ref<HTMLElement>()
function toggle() { open.value = !open.value }
function select(item: any) { emit('select', item); open.value = false }
function handleClickOutside(e: MouseEvent) {
  if (dropdownRef.value && !dropdownRef.value.contains(e.target as Node)) open.value = false
}
onMounted(() => document.addEventListener('click', handleClickOutside))
onUnmounted(() => document.removeEventListener('click', handleClickOutside))
</script>

<style scoped>
.ne-dropdown { position: relative; display: inline-flex; }
.dropdown-trigger { cursor: pointer; }
.dropdown-menu {
  position: absolute; top: 100%; min-width: 160px;
  background: var(--bg-panel); border: 1px solid var(--border-emphasis);
  border-radius: var(--radius-md); box-shadow: var(--shadow-lg);
  padding: var(--space-1); z-index: var(--z-dropdown);
  margin-top: var(--space-1);
}
.dropdown-menu.right { right: 0; }
.dropdown-item {
  display: flex; align-items: center; gap: var(--space-2);
  padding: var(--space-2) var(--space-3); border-radius: var(--radius-sm);
  font-size: var(--text-sm); color: var(--text-secondary); cursor: pointer;
  transition: all var(--transition-fast);
}
.dropdown-item:hover { background: var(--bg-hover); color: var(--text-primary); }
.dropdown-item.danger { color: var(--color-error); }
.dropdown-item.danger:hover { background: var(--color-error-subtle); }
.dropdown-item.disabled { opacity: 0.5; pointer-events: none; }
.item-icon { font-size: var(--text-sm); }
.dropdown-enter-active, .dropdown-leave-active { transition: all var(--transition-fast); }
.dropdown-enter-from, .dropdown-leave-to { opacity: 0; transform: translateY(-4px); }
</style>
