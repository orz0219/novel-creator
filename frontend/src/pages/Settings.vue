<template>
  <div class="settings-page">
    <div class="page-header">
      <div class="header-left">
        <button class="back-btn" type="button" @click="goBack">
          <ArrowLeft :size="16" />
          <span>{{ backLabel }}</span>
        </button>
        <h1 class="page-title">设置</h1>
      </div>
      <button class="save-btn" :disabled="saving" @click="save">
        {{ saving ? '保存中…' : '保存' }}
      </button>
    </div>
    <div v-if="savedAt" class="saved-hint">已保存 · {{ savedAt }}</div>
    <div v-if="loadError" class="error-hint">载入设置失败：{{ loadError }}</div>
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
      </div>

      <div class="settings-section">
        <h3 class="section-title">AI 模型</h3>
        <p class="section-hint">
          保存后立即对「创作引导对话 / AI 生成 / 文本抽取」生效，无需重启后端服务。
        </p>
        <div class="setting-item">
          <span class="setting-label">供应商</span>
          <select class="setting-select wide" v-model="form.aiProvider">
            <option v-for="p in AI_PROVIDERS" :key="p.id" :value="p.id">
              {{ p.label }}
            </option>
          </select>
        </div>
        <p v-if="currentPreset" class="section-hint">{{ currentPreset.hint }}</p>
        <div class="setting-item">
          <span class="setting-label">接口地址</span>
          <input
            class="setting-input wide"
            v-model="form.aiBaseUrl"
            placeholder="https://opencode.ai/zen/go/v1"
          />
        </div>
        <p v-if="providerApplied" class="section-hint">{{ providerApplied }}</p>
        <div class="setting-item">
          <span class="setting-label">API Key</span>
          <input
            class="setting-input wide"
            :type="showKey ? 'text' : 'password'"
            v-model="form.aiApiKey"
            :placeholder="`${currentPreset?.label ?? '当前供应商'} 的密钥`"
          />
          <button class="ghost-btn" type="button" @click="showKey = !showKey">
            {{ showKey ? '隐藏' : '显示' }}
          </button>
        </div>
        <p v-if="keyNotice" class="section-hint">{{ keyNotice }}</p>
        <div class="setting-item">
          <span class="setting-label">模型名</span>
          <select
            class="setting-select wide"
            v-model="form.defaultModel"
            :disabled="!modelChoices.length"
          >
            <option value="" disabled>{{ modelPlaceholder }}</option>
            <option v-for="m in modelChoices" :key="m" :value="m">{{ m }}</option>
          </select>
          <button
            class="ghost-btn"
            type="button"
            :disabled="loadingModels"
            @click="loadModels"
          >
            {{ loadingModels ? '获取中…' : '获取模型' }}
          </button>
        </div>
        <p v-if="modelsError" class="error-hint">模型列表获取失败：{{ modelsError }}</p>
        <p v-else-if="!form.aiApiKey" class="section-hint">
          请先填写 {{ currentPreset?.label ?? '当前供应商' }} 的 API Key，再点「获取模型」。
        </p>
        <p v-else-if="modelOptions.length" class="section-hint">
          已获取 {{ modelOptions.length }} 个可用模型，只可从下拉中选择。
        </p>
        <div class="setting-item">
          <span class="setting-label">上下文上限</span>
          <input
            class="setting-input wide"
            type="number"
            min="1"
            step="1000"
            v-model.number="currentModelLimit"
            placeholder="128000"
          />
          <span class="field-unit">tokens</span>
        </div>
        <p class="section-hint">
          已按所选模型自动带入<strong>真实上下文上限</strong>（来自内置模型目录；
          该网关自身不提供此数据，各模型上限差异很大，从 20 万到 105 万不等）。
          <template v-if="isLimitOverridden">当前为你的自定义覆盖值。</template>
          <template v-else>如需调整可直接修改，保存后会记为该模型的覆盖值。</template>
          对话页据此显示「已用 / 上限」并在接近上限时提醒。
        </p>
        <div class="setting-item">
          <span class="setting-label">单次输出上限</span>
          <input
            class="setting-input wide"
            type="number"
            min="256"
            step="512"
            v-model.number="form.maxOutputTokens"
            placeholder="22000"
          />
          <span class="field-unit">tokens</span>
        </div>
        <p class="section-hint">
          默认 22000。中文长文本 + 工具调用 JSON 容易撞到输出上限；如果模型网关本身
          不支持 22K 输出，请以模型/网关的真实上限为准。
        </p>
        <div class="ai-actions">
          <button class="test-btn" type="button" :disabled="testing" @click="runTest">
            {{ testing ? '测试中…' : '测试连接' }}
          </button>
          <span
            v-if="testResult"
            class="test-result"
            :class="testResult.ok ? 'ok' : 'fail'"
          >
            <template v-if="testResult.ok">
              连接成功 · {{ testResult.model }} · {{ testResult.latency_ms }}ms
            </template>
            <template v-else>连接失败：{{ testResult.error }}</template>
          </span>
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
        <h3 class="section-title">写作偏好</h3>
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
import { computed, reactive, ref, onMounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ArrowLeft } from 'lucide-vue-next'
import {
  settingsApi,
  type AppSettings,
  type TestConnectionResult,
  type ModelCatalog,
} from '@/api'
import {
  AI_PROVIDERS,
  findProvider,
  providerIdForBaseUrl,
  switchProviderKey,
} from '@/api/aiProviders'

const route = useRoute()
const router = useRouter()

/**
 * 返回目标：进入设置页时由来源页通过 `?from=` 带上（侧栏 / 命令面板）。
 * 直接打开设置页（如刷新、书签）时没有来源，则回到首页。
 */
const backTarget = computed(() => (route.query.from as string) || '/')
const backLabel = computed(() => (route.query.from ? '返回' : '返回首页'))

function goBack() {
  router.push(backTarget.value)
}

const form = reactive<AppSettings>({
  projectName: '',
  language: 'zh-CN',
  defaultModel: 'mimo-v2.5',
  // 供应商只决定「接口地址」这一项，后端不读该字段
  aiProvider: 'opencode',
  aiBaseUrl: 'https://opencode.ai/zen/go/v1',
  aiApiKey: '',
  // 各供应商各自的 key（切换时按 id 存取，两个 key 完全分离）
  aiApiKeys: {},
  contextLimit: 128000,
  maxOutputTokens: 22000,
  fontSize: 14,
  autoSave: true,
  writingStyle: '',
  autoValidate: true,
})

// ---------- 供应商 → 接口地址 / API Key 联动 ----------
// 选供应商只做两件事：带入它的接口地址、切到它自己的 API Key。
// 选「自定义」时**不动**接口地址（否则用户手填的地址会被抹掉）。

/** 当前选中供应商的预设；id 不在表中（旧数据）时为 null。 */
const currentPreset = computed(() => findProvider(form.aiProvider))

/** 最近一次自动带入地址的说明文字；没带入过则为空。 */
const providerApplied = ref('')

/** 密钥随供应商切换的说明（不显示 key 内容）。 */
const keyNotice = ref('')

/** 载入设置期间置位：此时由 initializeProvider 自己调 applyProvider，避免重复切换。 */
const initializing = ref(true)

/**
 * 切换供应商：先把当前 key 存回「原供应商」名下，再载入「新供应商」自己的 key。
 *
 * 这是两个 key 分离的关键——`form.aiApiKey` 是当前生效的那一个（后端只读它），
 * 所以在切换的瞬间必须先备份再替换，否则新 key 会覆盖掉旧供应商的 key。
 */
function applyProvider(nextId: string) {
  // 先备份旧供应商的 key，再换成新供应商自己的 key（账本运算见 switchProviderKey）
  const { keys, key } = switchProviderKey(
    form.aiApiKeys,
    form.aiProvider,
    nextId,
    form.aiApiKey,
  )
  form.aiApiKeys = keys
  form.aiProvider = nextId

  const preset = findProvider(nextId)
  if (preset && preset.baseUrl !== '') {
    form.aiBaseUrl = preset.baseUrl
    providerApplied.value = `已自动带入 ${preset.label} 的接口地址。`
  } else {
    providerApplied.value = '自定义供应商：接口地址请自行填写，已保留当前值。'
  }

  form.aiApiKey = key
  keyNotice.value = key
    ? `已载入 ${preset?.label ?? nextId} 自己的 API Key（与其他供应商互不影响）。`
    : `${preset?.label ?? nextId} 还没有填过 API Key，请填写后保存。`
}

/** 把当前输入框里的 key 存回指定供应商名下；清空则删掉该条目。 */
function backupKey(providerId: string) {
  const keys = form.aiApiKeys ?? {}
  const current = form.aiApiKey?.trim() ?? ''
  if (current) {
    keys[providerId] = current
  } else {
    delete keys[providerId]
  }
  form.aiApiKeys = keys
}

watch(
  () => form.aiProvider,
  (id) => {
    // aiProvider 在类型上是可选的；下拉改变时必定有值，为空则无从联动
    if (!id || initializing.value) return
    applyProvider(id)
  },
)

const saving = ref(false)
const savedAt = ref('')
const loadError = ref('')
const showKey = ref(false)
const testing = ref(false)
const testResult = ref<TestConnectionResult | null>(null)
/** 从网关拉到的真实模型列表（模型名下拉的候选）。 */
const modelOptions = ref<string[]>([])
const loadingModels = ref(false)
const modelsError = ref('')

/**
 * 下拉实际选项：拉取到的模型 + 当前已保存的模型名。
 * 把当前值并入是为了让已保存的模型在列表刷新前也能正确显示与选中。
 */
const modelChoices = computed(() => {
  const list = [...modelOptions.value]
  const current = form.defaultModel?.trim()
  if (current && !list.includes(current)) list.unshift(current)
  return list
})

const modelPlaceholder = computed(() =>
  modelOptions.value.length ? '请选择模型' : '请先点「刷新列表」获取模型',
)

// ---------- 上下文上限（按模型区分） ----------
// 网关不返回上下文长度，因此后端内置了一份真实目录；
// 下面这个输入框显示「当前模型」的上限，改动即写入该模型的覆盖值。

const catalog = ref<ModelCatalog>({ default_limit: 128000, context_limits: {} })
/** 当前模型生效的上限（显示在输入框里）。 */
const currentModelLimit = ref<number | null>(null)

/** 当前模型在目录中的已知上限（没有覆盖时就是它）。 */
const catalogLimitForModel = computed(() => {
  const model = (form.defaultModel || '').trim()
  return catalog.value.context_limits[model] ?? catalog.value.default_limit
})

/** 当前值是否来自用户覆盖（而非内置目录）。 */
const isLimitOverridden = computed(() => {
  const model = (form.defaultModel || '').trim()
  return model !== '' && form.contextLimits?.[model] !== undefined
})

/** 把「当前模型」的上限带入输入框：优先用户覆盖，其次内置目录。 */
function syncModelLimit() {
  const model = (form.defaultModel || '').trim()
  const override = model ? form.contextLimits?.[model] : undefined
  currentModelLimit.value = override ?? catalogLimitForModel.value
}

watch(() => form.defaultModel, syncModelLimit)

onMounted(async () => {
  try {
    const [s, c] = await Promise.all([
      settingsApi.get(),
      settingsApi.getModelCatalog(),
    ])
    Object.assign(form, s)
    catalog.value = c
  } catch (e) {
    loadError.value = (e as Error).message
    // 设置读不出来时表单仍是默认值：此时必须放开供应商联动，
    // 否则用户切换供应商不会带入接口地址，且页面上看不出原因。
    initializing.value = false
    return
  }
  initializeProvider()
  syncModelLimit()
  // 没填 API Key 就不拉：否则必然报一个「获取模型列表失败」，属于噪音
  if (form.aiApiKey) {
    await loadModels()
  }
})

/**
 * 回显供应商下拉并载入它自己的 key：优先用已保存的 aiProvider；
 * 没有该字段（老数据）时按已保存的接口地址反查。
 * 反查不到就落到「自定义」——不猜、不改动用户已填的地址。
 */
function initializeProvider() {
  const saved = findProvider(form.aiProvider)
  const id = saved ? saved.id : providerIdForBaseUrl(form.aiBaseUrl)
  // 表单是从库里 Object.assign 来的，这里同步走一遍，顺便把 key 载入表单
  applyProvider(id)
  initializing.value = false
}

async function save() {
  saving.value = true
  savedAt.value = ''
  try {
    // 只有与内置目录不一致时才写入按模型的覆盖，避免凭空堆一批冗余条目；
    // 恢复成目录值时顺手清掉旧覆盖。
    const model = (form.defaultModel || '').trim()
    const limits = { ...(form.contextLimits || {}) }
    if (model) {
      if (currentModelLimit.value && currentModelLimit.value !== catalogLimitForModel.value) {
        limits[model] = currentModelLimit.value
      } else {
        delete limits[model]
      }
    }
    // 保存前把当前 key 归到它所属的供应商名下，保证两个 key 各存一份
    backupKey(form.aiProvider ?? 'custom')
    const payload = { ...form, contextLimits: limits }
    await settingsApi.update(payload)
    Object.assign(form, payload)
    savedAt.value = new Date().toLocaleTimeString()
  } finally {
    saving.value = false
  }
}

/** 拉取网关真实可用的模型列表（OpenAI 兼容 GET /models）。 */
async function loadModels() {
  if (!form.aiApiKey) {
    modelsError.value = '请先填写 API Key，再点「获取模型」。'
    return
  }
  loadingModels.value = true
  modelsError.value = ''
  try {
    const result = await settingsApi.listModels({
      base_url: form.aiBaseUrl,
      api_key: form.aiApiKey,
    })
    if (result.ok) {
      modelOptions.value = result.models ?? []
    } else {
      modelOptions.value = []
      modelsError.value = result.error ?? '未知错误'
    }
  } catch (e) {
    modelOptions.value = []
    modelsError.value = (e as Error).message
  } finally {
    loadingModels.value = false
  }
}

/** 用当前表单里（尚未保存的）参数真发一次最小请求，验证网关是否可达。 */
async function runTest() {
  testing.value = true
  testResult.value = null
  try {
    testResult.value = await settingsApi.testConnection({
      base_url: form.aiBaseUrl,
      api_key: form.aiApiKey,
      model: form.defaultModel,
    })
    // 连通即拉取可用模型列表，省掉用户再去点一次「获取模型」
    if (testResult.value.ok) {
      await loadModels()
    }
  } catch (e) {
    testResult.value = { ok: false, error: (e as Error).message }
  } finally {
    testing.value = false
  }
}
</script>

<style scoped>
.settings-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
.page-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: var(--space-6); }
.header-left { display: flex; align-items: center; gap: var(--space-3); }
.back-btn { display: inline-flex; align-items: center; gap: var(--space-1); padding: var(--space-2) var(--space-3); background: transparent; color: var(--text-secondary); border: 1px solid var(--border-default); border-radius: var(--radius-sm); cursor: pointer; font-size: var(--text-sm); transition: var(--transition-fast); }
.back-btn:hover { color: var(--color-primary-text); border-color: var(--border-primary); }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }
.save-btn { padding: var(--space-2) var(--space-4); background: var(--color-primary); color: #fff; border: none; border-radius: var(--radius-sm); cursor: pointer; font-size: var(--text-sm); }
.save-btn:disabled { opacity: 0.6; cursor: default; }
.saved-hint { margin-bottom: var(--space-4); font-size: var(--text-xs); color: var(--text-secondary); }
.error-hint { margin-bottom: var(--space-4); font-size: var(--text-xs); color: var(--color-error); }
.settings-content { max-width: 600px; }
.settings-section { margin-bottom: var(--space-8); }
.section-title { font-size: var(--text-md); font-weight: 600; margin-bottom: var(--space-4); padding-bottom: var(--space-2); border-bottom: 1px solid var(--border-muted); }
.section-hint { font-size: var(--text-xs); color: var(--text-tertiary); margin-bottom: var(--space-3); line-height: 1.6; }
.setting-item { display: flex; align-items: center; justify-content: space-between; gap: var(--space-3); padding: var(--space-3) 0; }
.setting-label { font-size: var(--text-sm); color: var(--text-secondary); flex: 0 0 auto; }
.setting-input, .setting-select { padding: var(--space-2) var(--space-3); background: var(--bg-base); border: 1px solid var(--border-default); border-radius: var(--radius-sm); color: var(--text-primary); font-size: var(--text-sm); }
.setting-input.wide { flex: 1 1 auto; min-width: 0; font-family: var(--font-mono); }
.setting-select.wide { flex: 1 1 auto; min-width: 0; }
.field-unit { flex: 0 0 auto; font-size: var(--text-xs); color: var(--text-tertiary); font-family: var(--font-mono); }
.toggle { position: relative; display: inline-block; width: 40px; height: 22px; }
.toggle input { opacity: 0; width: 0; height: 0; }
.toggle-slider { position: absolute; cursor: pointer; inset: 0; background: var(--bg-panel-secondary); border-radius: 11px; transition: var(--transition-fast); }
.toggle-slider::before { content: ''; position: absolute; height: 16px; width: 16px; left: 3px; bottom: 3px; background: var(--text-tertiary); border-radius: 50%; transition: var(--transition-fast); }
.toggle input:checked + .toggle-slider { background: var(--color-primary); }
.toggle input:checked + .toggle-slider::before { transform: translateX(18px); background: white; }
.ai-actions { display: flex; align-items: center; gap: var(--space-3); padding-top: var(--space-3); }
.test-btn { padding: var(--space-2) var(--space-4); background: transparent; color: var(--text-primary); border: 1px solid var(--border-default); border-radius: var(--radius-sm); cursor: pointer; font-size: var(--text-sm); }
.test-btn:hover:not(:disabled) { border-color: var(--border-primary); color: var(--color-primary-text); }
.test-btn:disabled { opacity: 0.6; cursor: default; }
.ghost-btn { flex: 0 0 auto; padding: var(--space-1) var(--space-2); background: transparent; color: var(--text-tertiary); border: none; cursor: pointer; font-size: var(--text-xs); }
.ghost-btn:hover { color: var(--color-primary-text); }
.test-result { font-size: var(--text-xs); line-height: 1.5; }
.test-result.ok { color: var(--color-success); }
.test-result.fail { color: var(--color-error); }
</style>
