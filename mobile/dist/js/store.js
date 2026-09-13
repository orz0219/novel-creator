// 应用状态。仅保存「上次打开的项目/会话」这类轻量偏好到 localStorage；
// 业务数据一律以手机本地 SQLite（引擎）为唯一真源，不做本地缓存副本，避免出现两份真相。

const LS_KEY = 'novel.mobile.prefs'

function loadPrefs() {
  try {
    return JSON.parse(localStorage.getItem(LS_KEY) || '{}')
  } catch (e) {
    console.warn('[store] 偏好读取失败，按空处理', e)
    return {}
  }
}

function savePrefs(p) {
  try {
    localStorage.setItem(LS_KEY, JSON.stringify(p))
  } catch (e) {
    console.warn('[store] 偏好保存失败', e)
  }
}

export const prefs = loadPrefs()

export function setPref(key, value) {
  prefs[key] = value
  savePrefs(prefs)
}

export const state = {
  tab: 'agent',

  /// 首次启动引导（未配置 API Key 时展示）
  onboarding: false,
  onboardingData: { base_url: '', model: '' },

  /// 全屏详情页状态
  detail: null,

  /// 是否在设置页（设置不是底部 Tab，需独立标志）
  showingSettings: false,

  /// AI 文本抽取页（不是底部 Tab，用独立标志）
  showingExtract: false,
  extract: { text: '', running: false, result: null, error: null },

  /// 各 Tab 的搜索词（按 scope 分开：world / story）
  ///
  /// 放在这里而不是渲染时临时读输入框：搜索输入只做「局部重渲染」——
  /// 列表容器换掉、输入框不动，否则中文输入法的候选状态会被打断。
  search: { world: '', story: '' },

  engine: { ok: false, checking: true, error: null },

  projects: [],
  project: null,
  world: null,
  sessions: [],
  session: null,

  // 当前会话的消息（内存态，真源在数据库）
  messages: [],

  streaming: false,
  abortController: null,
  thinking: false,
  error: null,
  usage: null,

  tools: [],

  // 各 Tab 的数据（按需加载）
  /// 世界 / 故事 Tab 的数据缓存。
  ///
  /// 注意：这里的键必须与 `WORLD_SECTIONS` 的 key 一一对应。
  /// 漏了键的后果很隐蔽——`loadWorldAll()` 用 `if (c[key] === null)` 判断
  /// 「是否还没加载」，而漏掉的键是 `undefined`，判断不成立 => 永远不去请求，
  /// 界面上那个分类就一直停在「加载中…」，点它还会报「本地缓存里找不到这条数据」。
  /// （items / rules / relations 就是这么漏掉的。）
  cache: {
    characters: null,
    locations: null,
    factions: null,
    items: null,
    rules: null,
    relations: null,
    nodes: null,
    storylines: null,
    foreshadows: null,
    proposals: null,
    snapshots: null,
    events: null,
  },
}

export function clearCache() {
  for (const k of Object.keys(state.cache)) state.cache[k] = null
}

export function resetProjectData() {
  state.sessions = []
  state.session = null
  state.messages = []
  state.usage = null
  clearCache()
}
