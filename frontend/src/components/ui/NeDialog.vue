<template>
  <Teleport to="body">
    <Transition name="dialog">
      <div v-if="modelValue" class="dialog-overlay" @click.self="close">
        <div class="dialog-container" :class="size">
          <div class="dialog-header" v-if="title">
            <h3 class="dialog-title">{{ title }}</h3>
            <button class="dialog-close" @click="close">×</button>
          </div>
          <div class="dialog-body">
            <slot />
          </div>
          <div class="dialog-footer" v-if="$slots.footer">
            <slot name="footer" />
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
const props = defineProps<{
  modelValue: boolean
  title?: string
  size?: 'sm' | 'md' | 'lg'
}>()
const emit = defineEmits(['update:modelValue'])
function close() { emit('update:modelValue', false) }
</script>

<style scoped>
.dialog-overlay {
  position: fixed; inset: 0; background: var(--bg-overlay);
  display: flex; align-items: center; justify-content: center;
  z-index: var(--z-modal);
}
.dialog-container {
  background: var(--bg-panel); border: 1px solid var(--border-emphasis);
  border-radius: var(--radius-lg); box-shadow: var(--shadow-xl);
  max-height: 80vh; display: flex; flex-direction: column;
}
.dialog-container.sm { width: 360px; }
.dialog-container.md { width: 520px; }
.dialog-container.lg { width: 720px; }
.dialog-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: var(--space-4) var(--space-5); border-bottom: 1px solid var(--border-muted);
}
.dialog-title { font-size: var(--text-md); font-weight: 600; }
.dialog-close {
  border: none; background: transparent; color: var(--text-tertiary);
  font-size: var(--text-xl); cursor: pointer;
}
.dialog-close:hover { color: var(--text-primary); }
.dialog-body { padding: var(--space-5); overflow-y: auto; flex: 1; }
.dialog-footer {
  display: flex; justify-content: flex-end; gap: var(--space-2);
  padding: var(--space-3) var(--space-5); border-top: 1px solid var(--border-muted);
}
.dialog-enter-active, .dialog-leave-active { transition: opacity var(--transition-normal); }
.dialog-enter-from, .dialog-leave-to { opacity: 0; }
</style>
