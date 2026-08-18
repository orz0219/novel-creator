<template>
  <div class="ne-textarea-wrapper">
    <label class="textarea-label" v-if="label">{{ label }}</label>
    <textarea
      class="ne-textarea"
      :value="modelValue"
      :placeholder="placeholder"
      :rows="rows"
      :disabled="disabled"
      @input="$emit('update:modelValue', ($event.target as HTMLTextAreaElement).value)"
    ></textarea>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  modelValue?: string
  placeholder?: string
  label?: string
  rows?: number
  disabled?: boolean
}>()
defineEmits(['update:modelValue'])
</script>

<style scoped>
.ne-textarea-wrapper { display: flex; flex-direction: column; gap: var(--space-1); }
.textarea-label { font-size: var(--text-xs); font-weight: 500; color: var(--text-secondary); }
.ne-textarea {
  padding: var(--space-2) var(--space-3);
  background: var(--bg-base);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  font-family: inherit;
  outline: none;
  resize: vertical;
  transition: border-color var(--transition-fast);
}
.ne-textarea:focus { border-color: var(--color-primary); }
</style>
