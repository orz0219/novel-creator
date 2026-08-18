<template>
  <div class="history-page">
    <div class="page-header">
      <h1 class="page-title">历史记录</h1>
    </div>
    <div class="history-content">
      <EventLog />
      <VersionDiff />
    </div>
  </div>
</template>

<script setup lang="ts">
import EventLog from '@/components/event/EventLog.vue'
import VersionDiff from '@/components/version/VersionDiff.vue'
</script>

<style scoped>
.history-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
.page-header { margin-bottom: var(--space-6); }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }
.history-content { display: grid; grid-template-columns: 1fr 1fr; gap: var(--space-4); }
</style>
