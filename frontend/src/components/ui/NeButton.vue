<template>
  <button
    class="ne-button"
    :class="[variant, size, { disabled, loading }]"
    :disabled="disabled || loading"
    @click="$emit('click', $event)"
  >
    <span class="btn-spinner" v-if="loading"></span>
    <slot />
  </button>
</template>

<script setup lang="ts">
defineProps<{
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger'
  size?: 'sm' | 'md' | 'lg'
  disabled?: boolean
  loading?: boolean
}>()
defineEmits(['click'])
</script>

<style scoped>
.ne-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  border: 1px solid var(--border-default);
  background: var(--bg-panel);
  color: var(--text-secondary);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--transition-fast);
  font-family: inherit;
  white-space: nowrap;
}
.ne-button:hover:not(.disabled) { background: var(--bg-hover); color: var(--text-primary); border-color: var(--border-emphasis); }
.ne-button.sm { padding: var(--space-1) var(--space-2); font-size: var(--text-xs); }
.ne-button.md { padding: var(--space-2) var(--space-3); font-size: var(--text-sm); }
.ne-button.lg { padding: var(--space-3) var(--space-5); font-size: var(--text-md); }
.ne-button.primary { background: var(--color-primary); border-color: var(--color-primary); color: white; }
.ne-button.primary:hover:not(.disabled) { background: var(--color-primary-hover); }
.ne-button.secondary { border-color: var(--border-emphasis); }
.ne-button.ghost { border-color: transparent; background: transparent; }
.ne-button.ghost:hover:not(.disabled) { background: var(--bg-hover); }
.ne-button.danger { border-color: var(--color-error); color: var(--color-error); }
.ne-button.danger:hover:not(.disabled) { background: var(--color-error-subtle); }
.ne-button.disabled { opacity: 0.5; cursor: not-allowed; }
.btn-spinner {
  width: 14px; height: 14px; border: 2px solid currentColor; border-right-color: transparent;
  border-radius: 50%; animation: spin 0.6s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }
</style>
