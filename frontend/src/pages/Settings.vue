<template>
  <div class="settings-page">
    <div class="page-header">
      <h1 class="page-title">设置</h1>
      <button class="save-btn" :disabled="saving" @click="save">
        {{ saving ? '保存中…' : '保存' }}
      </button>
    </div>
    <div v-if="savedAt" class="saved-hint">已保存 · {{ savedAt }}</div>
    <div class="settings-content">
      <div class="settings-section">
        <h3 class="section-title">项目设置</h3>
        <div class="setting-item">
          <span class="setting-label">项目名称</span>
          <input class="setting-input" v-model="form.projectName" />
        </div>
        <div class="setting-item">
          <span class="setting-label">语言</span>
          <select class="setting-select" v-model="form.language">
            <option value="zh-CN">zh-CN</option>
            <option value="en">en</option>
          </select>
        </div>
        <div class="setting-item">
          <span class="setting-label">默认模型</span>
          <select class="setting-select" v-model="form.defaultModel">
            <option value="mimo-v2.5">mimo-v2.5</option>
            <option value="mimo-v2">mimo-v2</option>
          </select>
        </div>
      </div>
      <div class="settings-section">
        <h3 class="section-title">编辑器设置</h3>
        <div class="setting-item">
          <span class="setting-label">字体大小</span>
          <input class="setting-input" type="number" v-model.number="form.fontSize" />
        </div>
        <div class="setting-item">
          <span class="setting-label">自动保存</span>
          <label class="toggle">
            <input type="checkbox" v-model="form.autoSave" />
            <span class="toggle-slider"></span>
          </label>
        </div>
      </div>
      <div class="settings-section">
        <h3 class="section-title">AI 设置</h3>
        <div class="setting-item">
          <span class="setting-label">默认写作风格</span>
          <input class="setting-input" v-model="form.writingStyle" />
        </div>
        <div class="setting-item">
          <span class="setting-label">自动验证</span>
          <label class="toggle">
            <input type="checkbox" v-model="form.autoValidate" />
            <span class="toggle-slider"></span>
          </label>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, onMounted } from 'vue'
import { settingsApi, type AppSettings } from '@/api'

const form = reactive<AppSettings>({
  projectName: '',
  language: 'zh-CN',
  defaultModel: 'mimo-v2.5',
  fontSize: 14,
  autoSave: true,
  writingStyle: '',
  autoValidate: true,
})

const saving = ref(false)
const savedAt = ref('')

onMounted(async () => {
  try {
    const s = await settingsApi.get()
    Object.assign(form, s)
  } catch {
    // 载入失败保留默认值。
  }
})

async function save() {
  saving.value = true
  try {
    await settingsApi.update({ ...form })
    savedAt.value = new Date().toLocaleTimeString()
  } finally {
    saving.value = false
  }
}
</script>

<style scoped>
.settings-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
.page-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: var(--space-6); }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }
.save-btn { padding: var(--space-2) var(--space-4); background: var(--color-primary); color: #fff; border: none; border-radius: var(--radius-sm); cursor: pointer; font-size: var(--text-sm); }
.save-btn:disabled { opacity: 0.6; cursor: default; }
.saved-hint { margin-bottom: var(--space-4); font-size: var(--text-xs); color: var(--text-secondary); }
.settings-content { max-width: 600px; }
.settings-section { margin-bottom: var(--space-8); }
.section-title { font-size: var(--text-md); font-weight: 600; margin-bottom: var(--space-4); padding-bottom: var(--space-2); border-bottom: 1px solid var(--border-muted); }
.setting-item { display: flex; align-items: center; justify-content: space-between; padding: var(--space-3) 0; }
.setting-label { font-size: var(--text-sm); color: var(--text-secondary); }
.setting-input, .setting-select { padding: var(--space-2) var(--space-3); background: var(--bg-base); border: 1px solid var(--border-default); border-radius: var(--radius-sm); color: var(--text-primary); font-size: var(--text-sm); }
.toggle { position: relative; display: inline-block; width: 40px; height: 22px; }
.toggle input { opacity: 0; width: 0; height: 0; }
.toggle-slider { position: absolute; cursor: pointer; inset: 0; background: var(--bg-panel-secondary); border-radius: 11px; transition: var(--transition-fast); }
.toggle-slider::before { content: ''; position: absolute; height: 16px; width: 16px; left: 3px; bottom: 3px; background: var(--text-tertiary); border-radius: 50%; transition: var(--transition-fast); }
.toggle input:checked + .toggle-slider { background: var(--color-primary); }
.toggle input:checked + .toggle-slider::before { transform: translateX(18px); background: white; }
</style>
