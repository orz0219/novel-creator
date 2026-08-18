<template>
  <div class="items-page">
    <div class="page-header">
      <h1 class="page-title">物品</h1>
      <button class="btn-primary">+ 新建物品</button>
    </div>
    <div class="empty-state">
      <span class="empty-icon">📦</span>
      <span class="empty-text">暂无物品，点击上方按钮创建</span>
    </div>
  </div>
</template>

<style scoped>
.items-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-6); }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }
.btn-primary { padding: var(--space-2) var(--space-4); background: var(--color-primary); border: none; color: white; border-radius: var(--radius-sm); font-size: var(--text-sm); cursor: pointer; }
.btn-primary:hover { background: var(--color-primary-hover); }
.empty-state { display: flex; flex-direction: column; align-items: center; justify-content: center; padding: var(--space-16); color: var(--text-tertiary); }
.empty-icon { font-size: 48px; margin-bottom: var(--space-4); }
.empty-text { font-size: var(--text-sm); }
</style>
