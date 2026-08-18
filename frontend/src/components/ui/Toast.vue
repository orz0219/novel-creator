<template>
  <div class="toast-container">
    <TransitionGroup name="toast">
      <div
        v-for="toast in uiStore.toasts"
        :key="toast.id"
        class="toast"
        :class="toast.type"
      >
        <span class="toast-title">{{ toast.title }}</span>
        <span class="toast-message" v-if="toast.message">{{ toast.message }}</span>
        <button class="toast-close" @click="uiStore.removeToast(toast.id)">×</button>
      </div>
    </TransitionGroup>
  </div>
</template>

<script setup lang="ts">
import { useUiStore } from '@/stores/ui'
const uiStore = useUiStore()
</script>

<style scoped>
.toast-container {
  position: fixed;
  bottom: var(--space-4);
  right: var(--space-4);
  z-index: var(--z-toast);
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.toast {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-3) var(--space-4);
  background: var(--bg-panel);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-lg);
  min-width: 280px;
}

.toast.success { border-left: 3px solid var(--color-success); }
.toast.error { border-left: 3px solid var(--color-error); }
.toast.warning { border-left: 3px solid var(--color-warning); }
.toast.info { border-left: 3px solid var(--color-info); }

.toast-title { font-size: var(--text-sm); font-weight: 500; }
.toast-message { font-size: var(--text-xs); color: var(--text-secondary); }
.toast-close {
  margin-left: auto;
  border: none;
  background: transparent;
  color: var(--text-tertiary);
  cursor: pointer;
  font-size: var(--text-lg);
}

.toast-enter-active,
.toast-leave-active {
  transition: all var(--transition-normal);
}
.toast-enter-from { opacity: 0; transform: translateX(100px); }
.toast-leave-to { opacity: 0; transform: translateX(100px); }
</style>
