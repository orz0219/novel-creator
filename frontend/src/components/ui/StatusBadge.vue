<template>
  <span class="status-badge" :class="status">
    {{ label }}
  </span>
</template>

<script setup lang="ts">
defineProps<{
  status: string
  label: string
}>()
</script>

<style scoped>
.status-badge {
  display: inline-flex;
  align-items: center;
  padding: 2px 8px;
  border-radius: 10px;
  font-size: var(--text-xs);
  font-weight: 500;
}

.status-badge.completed { background: var(--color-success-subtle); color: var(--color-success); }
.status-badge.inprogress, .status-badge.in_progress, .status-badge.active { background: var(--color-accent-subtle); color: var(--color-accent); }
.status-badge.planned, .status-badge.draft { background: var(--bg-panel-secondary); color: var(--text-tertiary); }
.status-badge.paused { background: var(--color-warning-subtle); color: var(--color-warning); }
.status-badge.archived, .status-badge.abandoned { background: var(--bg-panel-secondary); color: var(--text-disabled); }
.status-badge.approved { background: var(--color-success-subtle); color: var(--color-success); }
.status-badge.rejected { background: var(--color-error-subtle); color: var(--color-error); }
.status-badge.pending { background: var(--color-warning-subtle); color: var(--color-warning); }
</style>
