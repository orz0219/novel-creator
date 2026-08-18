<template>
  <div class="ne-select-wrapper">
    <label class="select-label" v-if="label">{{ label }}</label>
    <select
      class="ne-select"
      :value="modelValue"
      :disabled="disabled"
      @change="$emit('update:modelValue', ($event.target as HTMLSelectElement).value)"
    >
      <option value="" disabled>{{ placeholder }}</option>
      <option v-for="opt in options" :key="opt.value" :value="opt.value">{{ opt.label }}</option>
    </select>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  modelValue?: string
  options: { value: string; label: string }[]
  placeholder?: string
  label?: string
  disabled?: boolean
}>()
defineEmits(['update:modelValue'])
</script>

<style scoped>
.ne-select-wrapper { display: flex; flex-direction: column; gap: var(--space-1); }
.select-label { font-size: var(--text-xs); font-weight: 500; color: var(--text-secondary); }
.ne-select {
  padding: var(--space-2) var(--space-3);
  background: var(--bg-base);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  font-family: inherit;
  outline: none;
  cursor: pointer;
}
.ne-select:focus { border-color: var(--color-primary); }
</style>
