<template>
  <div class="ne-input-wrapper" :class="{ focused, error }">
    <label class="input-label" v-if="label">{{ label }}</label>
    <input
      ref="inputRef"
      class="ne-input"
      :type="type"
      :value="modelValue"
      :placeholder="placeholder"
      :disabled="disabled"
      @input="$emit('update:modelValue', ($event.target as HTMLInputElement).value)"
      @focus="focused = true"
      @blur="focused = false"
    />
    <span class="input-error" v-if="error">{{ error }}</span>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
defineProps<{
  modelValue?: string
  type?: string
  placeholder?: string
  label?: string
  disabled?: boolean
  error?: string
}>()
defineEmits(['update:modelValue'])
const focused = ref(false)
const inputRef = ref<HTMLInputElement>()
</script>

<style scoped>
.ne-input-wrapper { display: flex; flex-direction: column; gap: var(--space-1); }
.input-label { font-size: var(--text-xs); font-weight: 500; color: var(--text-secondary); }
.ne-input {
  padding: var(--space-2) var(--space-3);
  background: var(--bg-base);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  font-family: inherit;
  outline: none;
  transition: border-color var(--transition-fast);
}
.ne-input:focus { border-color: var(--color-primary); }
.ne-input:disabled { opacity: 0.5; cursor: not-allowed; }
.ne-input-wrapper.error .ne-input { border-color: var(--color-error); }
.input-error { font-size: var(--text-xs); color: var(--color-error); }
</style>
