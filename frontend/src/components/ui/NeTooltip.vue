<template>
  <div class="ne-tooltip-wrapper" @mouseenter="show = true" @mouseleave="show = false">
    <slot />
    <Teleport to="body">
      <Transition name="tooltip">
        <div v-if="show" class="ne-tooltip" :class="position" :style="tooltipStyle">
          {{ text }}
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
const props = defineProps<{ text: string; position?: 'top' | 'bottom' | 'left' | 'right' }>()
const show = ref(false)
const tooltipStyle = computed(() => ({}))
</script>

<style scoped>
.ne-tooltip-wrapper { position: relative; display: inline-flex; }
.ne-tooltip {
  position: fixed; padding: var(--space-1) var(--space-2);
  background: var(--bg-panel-tertiary); border: 1px solid var(--border-emphasis);
  border-radius: var(--radius-sm); font-size: var(--text-xs); color: var(--text-primary);
  white-space: nowrap; pointer-events: none; z-index: var(--z-tooltip);
  box-shadow: var(--shadow-md);
}
.tooltip-enter-active, .tooltip-leave-active { transition: opacity var(--transition-fast); }
.tooltip-enter-from, .tooltip-leave-to { opacity: 0; }
</style>
