// Novel Engine 手机版主逻辑
//
// 界面结构：顶栏（项目 + 引擎状态）+ 主视图 + 输入区 + 底部 4 Tab
// 对话为主界面；工具调用内联成卡片，点开看详情。

import { api, streamChat } from './api.js'
import { isTauri, pickImportFiles, takeSchemaRebuildNotice, reportDiagnostic, backupDatabase, setKeepAlive } from './tauri.js'
import { fetchRemoteProjects, fetchRemoteSettings, importFromComputer, normalizeHost } from './api.js'
import { state, prefs, setPref, resetProjectData, clearCache } from './store.js'
import { renderMarkdown } from './markdown.js'

const $ = (id) => document.getElementById(id)

/** 复制图标（线性风格，与顶栏那几个按钮一致；用内联 SVG，不引图标字体） */
const COPY_ICON = '<svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>'
const els = {
  view: $('view'),
  projectName: $('project-name'),
  engineDot: $('engine-dot'),
  engineText: $('engine-text'),
  composer: $('composer'),
  input: $('input'),
  send: $('btn-send'),
  composerError: $('composer-error'),
  drawer: $('drawer'),
  drawerMask: $('drawer-mask'),
  drawerBody: $('drawer-body'),
  toast: $('toast'),
}

// ---------------------------------------------------------------- 基础工具

function esc(s) {
  return String(s ?? '').replace(/[&<>"']/g, (c) => (
    { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]
  ))
}

/**
 * 把后端的时间戳格式化成手机上看得清的短格式：`09-12 21:57`。
 *
 * 后端给的是 RFC3339（如 `2026-09-12T13:57:40.456048Z`），原样显示会占掉
 * 一整行还看不清重点。认不出来的值（例如叙事时间「第三年春」）原样返回。
 */
function fmtTime(v) {
  const raw = String(v ?? '').trim()
  if (!raw) return ''
  const d = new Date(raw)
  if (Number.isNaN(d.getTime())) return raw
  const p = (n) => String(n).padStart(2, '0')
  return `${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`
}

let toastTimer = null
function toast(msg, ms = 2200) {
  els.toast.textContent = msg
  els.toast.hidden = false
  clearTimeout(toastTimer)
  toastTimer = setTimeout(() => { els.toast.hidden = true }, ms)
}

/**
 * 轻量 markdown 渲染。
 *
 * AI 回复里常有标题、列表、引用，只按段落处理会挤成一坨，阅读体验很差。
 * 这里按「行 → 块」两级处理：
 *   - 块级：标题 / 无序列表 / 有序列表 / 引用 / 分隔线 / 段落
 *   - 行内：粗体 / 斜体 / 行内代码
 * 全部先做 HTML 转义，避免内容里的尖括号被当成标签。
 */
function renderInline(raw) {
  return esc(raw)
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
    .replace(/(^|[^*])\*([^*\n]+)\*(?!\*)/g, '$1<em>$2</em>')
}

/**
 * 把 Markdown 渲染成 HTML。
 *
 * 实现搬到了 `markdown.js`（用与电脑端相同的 marked + DOMPurify）：
 * 原先这里是手写的轻量渲染器，只认标题 / 列表 / 引用 / 行内样式，
 * 不认 AI 回复里常见的**代码块**与**表格**，那些就原样显示成一堆符号。
 */
function renderText(src) {
  return renderMarkdown(src)
}

function scrollToBottom() {
  // 用 rAF 等一帧，避免在连续渲染时反复触发布局计算
  requestAnimationFrame(() => {
    els.view.scrollTop = els.view.scrollHeight
  })
}

// ---------------------------------------------------------------- 引擎状态

async function checkEngine() {
  state.engine.checking = true
  renderEngineStatus()
  try {
    const t = await api.health()
    state.engine.ok = t.trim() === 'OK'
    state.engine.error = state.engine.ok ? null : `引擎返回: ${t}`
  } catch (e) {
    state.engine.ok = false
    state.engine.error = e.message
  }
  state.engine.checking = false
  renderEngineStatus()
  return state.engine.ok
}

function renderEngineStatus() {
  const e = state.engine
  els.engineDot.className = 'dot' + (e.checking ? ' busy' : e.ok ? ' ok' : ' bad')
  els.engineText.textContent = e.checking ? '正在连接引擎…' : e.ok ? '本地引擎已就绪' : '引擎未连接'
}

// ---------------------------------------------------------------- 项目

async function loadProjects() {
  state.projects = await api.listProjects()
  // 恢复上次打开的项目；否则用第一个；再否则 null
  const lastId = prefs.lastProjectId
  let p = state.projects.find((x) => x.id === lastId) || state.projects[0] || null
  if (p) await selectProject(p.id, { silent: true })
}

async function selectProject(id, { silent = false } = {}) {
  state.project = state.projects.find((x) => x.id === id) || await api.getProject(id)
  setPref('lastProjectId', id)
  els.projectName.textContent = state.project?.name || '未命名项目'
  resetProjectData()
  if (!silent) toast(`已切换到「${state.project?.name || ''}」`)

  // 载入主世界
  try {
    state.world = await api.getWorld(id)
  } catch (e) {
    state.world = null
    console.warn('[app] 主世界读取失败', e)
  }

  // 载入会话列表，若无会话则自动建一个（手机端希望打开就能聊）
  try {
    state.sessions = await api.listSessions(id)
    if (state.sessions.length === 0) {
      const r = await api.createSession(id)
      state.sessions = await api.listSessions(id)
      state.session = state.sessions.find((s) => s.id === r.session_id) || state.sessions[0]
    } else {
      const lastSid = prefs.lastSessionId
      state.session = state.sessions.find((s) => s.id === lastSid) || state.sessions[0]
    }
    if (state.session) {
      setPref('lastSessionId', state.session.id)
      await loadMessages(state.session.id)
    }
  } catch (e) {
    state.error = e.message
  }
  render()
}

async function createProject(name) {
  const p = await api.createProject(name, null)
  state.projects = await api.listProjects()
  await selectProject(p.id, { silent: true })
  toast('项目已创建')
  return p
}

async function loadMessages(sessionId) {
  const s = await api.getSession(sessionId)
  state.messages = (s.messages || []).map((m) => ({
    id: m.id,
    role: m.role === 'user' ? 'user' : 'ai',
    content: m.content,
    tools: [],
  }))
}

/**
 * 从手机数据库重新拉取当前会话，覆盖内存里的消息。
 *
 * 用途：App 退到后台时 Android 会限制它的执行，正在进行的 SSE 会被掐断，
 * 前端只收到半截回复；而引擎（在同一进程里）若已把完整回复写入数据库，
 * 回到前台刷新一次就能把内容补齐，用户不会看到残缺的对话。
 *
 * 返回是否发生了更新（供调用方决定要不要重绘）。
 */
async function refreshSessionFromDb() {
  const sid = state.session?.id
  if (!sid) return false
  try {
    const s = await api.getSession(sid)
    const fresh = (s.messages || []).map((m) => ({
      id: m.id,
      role: m.role === 'user' ? 'user' : 'ai',
      content: m.content,
      tools: [],
    }))
    // 只比「条数 + 内容」：流式期间消息 id 是前端临时生成的，
    // 拿它比较会导致内容没变也判定为变化，白白重绘一次。
    const changed =
      fresh.length !== state.messages.length ||
      fresh.some((m, i) => m.content !== (state.messages[i]?.content ?? ''))
    if (changed) state.messages = fresh
    return changed
  } catch (e) {
    // 刷新失败不该打扰用户，留下痕迹即可
    console.warn('[app] 会话刷新失败', e)
    return false
  }
}

async function switchSession(sessionId) {
  state.session = state.sessions.find((s) => s.id === sessionId)
  if (!state.session) return
  setPref('lastSessionId', sessionId)
  await loadMessages(sessionId)
  closeDrawer()
  state.tab = 'agent'
  render()
}

async function newSession() {
  if (!state.project) { toast('请先选择项目'); return }
  const r = await api.createSession(state.project.id)
  state.sessions = await api.listSessions(state.project.id)
  state.session = state.sessions.find((s) => s.id === r.session_id) || state.sessions[0]
  setPref('lastSessionId', state.session.id)
  state.messages = []
  state.usage = null
  closeDrawer()
  render()
  toast('已新建会话')
}

// ---------------------------------------------------------------- 发送消息

async function send(text) {
  const msg = (text ?? els.input.value).trim()
  if (!msg || state.streaming) return

  if (!state.session) {
    try {
      const r = await api.createSession(state.project?.id)
      state.sessions = await api.listSessions(state.project.id)
      state.session = state.sessions.find((s) => s.id === r.session_id) || state.sessions[0]
    } catch (e) {
      state.error = `无法创建会话: ${e.message}`
      render()
      return
    }
  }

  els.input.value = ''
  autoGrow()
  state.error = null
  state.streaming = true
  state.thinking = true
  state.messages.push({ id: 'local-' + Date.now(), role: 'user', content: msg, tools: [] })
  const aiMsg = { id: 'ai-' + Date.now(), role: 'ai', content: '', tools: [] }
  state.messages.push(aiMsg)
  render()

  // 生成期间保持屏幕常亮（避免用户等待时切走导致中断）。
  //
  // 故意不 await：常亮只是辅助手段，而 `navigator.wakeLock.request()` 在某些环境下
  // 会一直 pending（实测 headless Chromium 就是），一旦 await 就把整条发送流程卡死。
  // 申请失败也不影响正文生成（函数内部已 catch）。
  acquireWakeLock()
  // 光是不息屏还不够：用户切到别的应用时 Android 会冻结本进程，引擎跟着停、生成就断。
  // 前台服务让进程保持存活（通知栏会出现「正在生成」）。同样不 await——
  // 它只是保命手段，失败不该挡住生成本身。
  setKeepAlive(true).catch((e) => console.warn('[app] 前台服务启动失败', e))

  const ctrl = new AbortController()
  state.abortController = ctrl

  try {
    await streamChat(state.session.id, msg, {
      onStatus: () => { state.thinking = true; render() },
      onToken: (t) => {
        state.thinking = false
        aiMsg.content += t
        updateStreamingBubble(aiMsg)
      },
      onTool: (payload) => {
        aiMsg.tools.push(payload)
        render()
      },
      onQuestion: (payload) => {
        aiMsg.tools.push({ name: 'ask_question', input: payload })
        render()
      },
      onUsage: (u) => { state.usage = u },
      onError: (e) => { state.error = e },
      onDone: () => {},
    }, ctrl.signal)
  } catch (e) {
    if (e.name === 'AbortError') {
      toast('已停止生成')
    } else {
      // 常见于「App 切到后台」导致连接被系统掐断：先尝试从数据库补齐完整回复
      const filled = await refreshSessionFromDb()
      // 有了前台服务，正常切后台不会再断；这里更可能是网络波动，
      // 所以文案指向网络，而不是让用户「别切走」。
      state.error = filled
        ? null
        : `连接中断（${e.message}）。网络恢复后重发即可；若回复其实已生成，切回本页会自动补齐。`
      if (filled) toast('已从本地补齐完整回复')
    }
  } finally {
    state.streaming = false
    state.thinking = false
    state.abortController = null
    await releaseWakeLock()
    // 生成结束（成功或失败）就撤掉前台服务，别让通知一直挂着
    setKeepAlive(false).catch((e) => console.warn('[app] 前台服务停止失败', e))
    render()
  }
}

function stopStreaming() {
  state.abortController?.abort()
}

/** 流式过程中只更新最后一条气泡，避免整页重绘导致滚动跳动 */
function updateStreamingBubble(aiMsg) {
  const node = els.view.querySelector(`[data-msg-id="${aiMsg.id}"] .bubble`)
  if (node) {
    node.innerHTML = renderText(aiMsg.content)
    scrollToBottom()
  } else {
    render()
  }
}

// ---------------------------------------------------------------- 屏幕常亮
//
// 生成一条回复往往要等几十秒，用户这时容易切走去干别的；而 App 一退到后台，
// Android 就会冻结进程、对话随之中断（详见实施记录「后台限制」一节）。
// 生成期间保持屏幕常亮，能显著减少「等待时切走」的情况。
//
// 用 WebView 的 Screen Wake Lock API；不支持时静默跳过（不影响功能）。
let wakeLock = null

async function acquireWakeLock() {
  try {
    if (!('wakeLock' in navigator)) return
    if (wakeLock) return
    wakeLock = await navigator.wakeLock.request('screen')
    wakeLock.addEventListener('release', () => { wakeLock = null })
    console.log('[app] 已开启屏幕常亮')
  } catch (e) {
    // 系统可能拒绝（低电量等），这不该影响对话
    console.warn('[app] 屏幕常亮申请失败', e)
    wakeLock = null
  }
}

async function releaseWakeLock() {
  try {
    if (wakeLock) {
      await wakeLock.release()
      wakeLock = null
    }
  } catch (e) {
    console.warn('[app] 释放屏幕常亮失败', e)
  }
}

// ---------------------------------------------------------------- 渲染

function render() {
  // 顶部
  els.projectName.textContent = state.project?.name || '选择项目'
  renderEngineStatus()

  // Tab 高亮
  document.querySelectorAll('.tab').forEach((t) => {
    t.classList.toggle('active', t.dataset.tab === state.tab)
  })

  // 输入区只在对话页显示
  els.composer.hidden = state.tab !== 'agent'

  // 错误条
  if (state.error) {
    els.composerError.textContent = state.error
    els.composerError.hidden = false
  } else {
    els.composerError.hidden = true
  }

  if (state.onboarding) {
    els.composer.hidden = true
    els.view.innerHTML = renderOnboarding()
    bindOnboarding()
    return
  }

  // 设置页不是底部 Tab 之一，用独立标志判定；
  // 若把它当作 tab 值塞进 renderers 映射，点底部 Tab 时会取到 undefined 而崩。
  if (state.showingSettings) {
    renderSettings()
    return
  }

  // AI 文本抽取页：也不是底部 Tab，同样用独立标志
  if (state.showingExtract) {
    els.composer.hidden = true
    renderExtract()
    return
  }

  if (state.detail) {
    els.view.innerHTML = renderDetail()
    bindViewEvents()
    return
  }

  if (!state.project) {
    els.view.innerHTML = renderNoProject()
    bindViewEvents()
    return
  }

  const renderers = {
    agent: renderAgent,
    world: renderWorld,
    story: renderStory,
    more: renderMore,
  }
  // 兜底：tab 值异常时回到对话页，而不是把 "undefined" 写进页面
  const view = renderers[state.tab]
  if (!view) {
    console.warn('[app] 未知的 tab:', state.tab, '→ 回退到对话页')
    state.tab = 'agent'
  }
  els.view.innerHTML = renderers[state.tab]()
  bindViewEvents()
  bindSearchInputs()
  if (state.tab === 'agent') scrollToBottom()
}

/** 首次启动引导：让用户填自己的 AI 网关配置 */
function renderOnboarding() {
  return `
    <div class="empty" style="padding-top:24px">
      <div class="seal">笔</div>
      <h3>欢迎使用 Novel Engine</h3>
      <p>这台手机自带完整引擎，数据全部存在本机。<br>填一次 AI 网关信息即可开始创作。</p>
    </div>

    <div class="field">
      <label>接口地址</label>
      <input id="ob-base" type="url" placeholder="https://opencode.ai/zen/go/v1"
             value="${esc(state.onboardingData.base_url || '')}" />
      <div class="hint">OpenAI 兼容端点前缀，不含 /chat/completions</div>
    </div>
    <div class="field">
      <label>API Key</label>
      <input id="ob-key" type="password" placeholder="sk-..." />
      <div class="hint">只保存在本机数据库，用于直连 AI 服务商</div>
    </div>
    <div class="field">
      <label>模型</label>
      <input id="ob-model" type="text" placeholder="deepseek-chat"
             value="${esc(state.onboardingData.model || '')}" />
    </div>

    <button class="btn-primary" id="ob-save">开始使用</button>
    <button class="btn-secondary" id="ob-skip">稍后再说</button>
    <div id="ob-msg" class="hint" style="margin-top:10px"></div>
  `
}

function bindOnboarding() {
  const msg = $('ob-msg')

  $('ob-save').addEventListener('click', async () => {
    const base_url = $('ob-base').value.trim()
    const api_key = $('ob-key').value.trim()
    const model = $('ob-model').value.trim()

    // 明确校验，不静默放过
    if (!base_url) { msg.textContent = '请填写 API 地址'; return }
    if (!api_key) { msg.textContent = '请填写 API Key'; return }
    if (!model) { msg.textContent = '请填写模型名称'; return }

    msg.textContent = '正在保存…'
    try {
      // 字段名与电脑端设置页一致（引擎读的是 aiBaseUrl / aiApiKey / defaultModel）
      await api.saveSettings({
        aiBaseUrl: base_url,
        aiApiKey: api_key,
        defaultModel: model,
      })
      state.onboarding = false
      toast('设置已保存，开始创作吧')
      const ok = await checkEngine()
      if (ok) await loadProjects()
      render()
      updateSendBtn()
    } catch (e) {
      msg.textContent = `保存失败：${e.message}`
    }
  })

  $('ob-skip').addEventListener('click', async () => {
    state.onboarding = false
    toast('可随时在「更多 → AI 设置」里配置')
    render()
    updateSendBtn()
  })
}

/** 详情页可以「就地编辑」的实体类型 → 调哪个接口更新基础信息 */
const ENTITY_UPDATERS = {
  characters: (id, data) => api.updateCharacter(id, data),
  locations: (id, data) => api.updateLocation(id, data),
  factions: (id, data) => api.updateFaction(id, data),
  items: (id, data) => api.updateEntityGeneric(id, data),
}

/**
 * 详情页就地编辑：基础信息 / 档案 / 当前状态。
 *
 * 三种编辑态的字段清单由后端实现决定（见 `crates/db/src/application_ports.rs`）：
 *   - 基础信息：`PUT /{resource}/{id}`，**部分更新**（不传的字段保持原值）
 *   - 档案 / 当前状态：`PUT .../profile|state`，**整行覆盖**——所以保存时必须
 *     回传完整对象，漏掉字段等于把它清空。这里用「原对象 + 表单改动」合并。
 */
function renderDetailEditor(d) {
  const e = d.editing
  if (!e) return ''
  const fields = editFieldSpec(d.kind, e.what)
  let html = `<div class="section-title">${esc(e.title)}</div>`
  for (const f of fields) {
    const val = e.form[f.key] ?? ''
    html += `<div class="field">
      <label>${esc(f.label)}</label>
      ${f.multiline
        ? `<textarea id="ed-${f.key}" class="edit-area" rows="${f.rows || 4}" placeholder="${esc(f.placeholder || '')}">${esc(val)}</textarea>`
        : `<input id="ed-${f.key}" type="text" value="${esc(val)}" placeholder="${esc(f.placeholder || '')}" />`}
      ${f.hint ? `<div class="hint">${esc(f.hint)}</div>` : ''}
    </div>`
  }
  html += `
    <button class="btn-primary" data-act="save-detail-edit">保存</button>
    <button class="btn-secondary" data-act="cancel-detail-edit">取消</button>
    <div class="hint" id="ed-msg"></div>`
  return html
}

/** 各编辑态的字段清单 */
function editFieldSpec(kind, what) {
  if (what === 'entity') {
    return [
      { key: 'name', label: '名称', placeholder: '必填' },
      { key: 'summary', label: '摘要', multiline: true, rows: 2 },
      { key: 'description', label: '描述', multiline: true, rows: 6 },
    ]
  }
  if (what === 'state') {
    return [
      { key: 'location', label: '位置' },
      { key: 'physical_state', label: '身体状态', multiline: true, rows: 3 },
      { key: 'mental_state', label: '心理状态', multiline: true, rows: 3 },
      { key: 'resource_state', label: '资源状态', multiline: true, rows: 3 },
      { key: 'social_state', label: '社交状态', multiline: true, rows: 3 },
    ]
  }
  // 档案：按实体类型给不同字段
  if (kind === 'characters') {
    return [
      { key: 'name', label: '真名' },
      { key: 'identity', label: '身份' },
      { key: 'age_range', label: '年龄', placeholder: '如「青年」「中年」', hint: '可用中文；认不出的写法后端会明确报错' },
      { key: 'gender', label: '性别', placeholder: '如「男」「女」' },
      { key: 'role_in_story', label: '剧情作用', placeholder: '如「主角」「反派」' },
      { key: 'appearance', label: '外貌', multiline: true, rows: 4 },
      { key: 'background_origin', label: '出身', multiline: true, rows: 4 },
      { key: 'core_personality', label: '核心性格', multiline: true, rows: 4 },
      { key: 'values', label: '价值观', multiline: true, rows: 3 },
      { key: 'aliases', label: '别名', placeholder: '多个用逗号分隔', hint: '逗号分隔，超过一个会存成别名列表' },
    ]
  }
  if (kind === 'locations') {
    return [
      { key: 'location_type', label: '地点类型' },
      { key: 'size', label: '规模' },
      { key: 'climate', label: '气候' },
      { key: 'geography', label: '地理', multiline: true, rows: 4 },
      { key: 'appearance', label: '外观', multiline: true, rows: 4 },
      { key: 'population', label: '人口', multiline: true, rows: 3 },
      { key: 'economy', label: '经济', multiline: true, rows: 3 },
      { key: 'rules', label: '规则', multiline: true, rows: 3 },
      { key: 'history', label: '历史', multiline: true, rows: 4 },
      { key: 'narrative_usage', label: '叙事用途', multiline: true, rows: 3 },
    ]
  }
  if (kind === 'factions') {
    return [
      { key: 'leader', label: '首领' },
      { key: 'goals', label: '目标', multiline: true, rows: 3 },
      { key: 'values', label: '价值观', multiline: true, rows: 3 },
      { key: 'resources', label: '资源', multiline: true, rows: 3 },
      { key: 'territory', label: '领地', multiline: true, rows: 3 },
      { key: 'members', label: '成员', multiline: true, rows: 3 },
      { key: 'enemies', label: '敌对', multiline: true, rows: 2 },
      { key: 'allies', label: '盟友', multiline: true, rows: 2 },
      { key: 'internal_conflicts', label: '内部矛盾', multiline: true, rows: 3 },
      { key: 'secrets', label: '秘密', multiline: true, rows: 3 },
      { key: 'modus_operandi', label: '行事风格', multiline: true, rows: 3 },
    ]
  }
  return []
}

/** 进入某一种编辑态（把当前值拷成草稿；档案/状态用整行对象做底，避免保存时丢字段） */
function startDetailEdit(what) {
  const d = state.detail
  if (!d) return
  const it = d.data || {}

  if (what === 'entity') {
    d.editing = {
      what,
      title: '编辑基础信息',
      form: {
        name: it.name || '',
        summary: it.summary || '',
        description: it.description || '',
      },
    }
  } else if (what === 'state') {
    const st = d.status && typeof d.status === 'object' ? d.status : {}
    d.editing = {
      what,
      title: '编辑当前状态',
      form: {
        location: st.location || '',
        physical_state: st.physical_state || '',
        mental_state: st.mental_state || '',
        resource_state: st.resource_state || '',
        social_state: st.social_state || '',
      },
    }
  } else {
    const p = d.profile && typeof d.profile === 'object' ? d.profile : {}
    const form = {}
    for (const f of editFieldSpec(d.kind, 'profile')) {
      const v = p[f.key]
      // 别名是数组，编辑框里用逗号分隔的文本
      form[f.key] = f.key === 'aliases'
        ? (Array.isArray(v) ? v.join('，') : (v ?? ''))
        : (v ?? '')
    }
    d.editing = { what, title: '编辑档案', form }
  }
  render()
}

/** 退出编辑态（不保存） */
function cancelDetailEdit() {
  if (!state.detail) return
  state.detail.editing = null
  render()
}

/** 保存详情页编辑 */
async function saveDetailEdit() {
  const d = state.detail
  if (!d?.editing) return
  const { what, form } = d.editing
  const msg = $('ed-msg')

  // 从表单读回（用户可能改了又改，以输入框为准）
  for (const f of editFieldSpec(d.kind, what === 'entity' ? 'entity' : what)) {
    form[f.key] = $(`ed-${f.key}`)?.value ?? ''
  }
  if (what === 'entity' && !String(form.name || '').trim()) {
    msg.textContent = '名称不能为空'
    return
  }

  msg.textContent = '正在保存…'
  try {
    if (what === 'entity') {
      const updater = ENTITY_UPDATERS[d.kind]
      if (!updater) throw new Error(`「${d.kind}」不支持编辑基础信息`)
      const updated = await updater(d.id, {
        name: form.name.trim(),
        summary: form.summary,
        description: form.description,
      })
      if (updated && typeof updated === 'object') d.data = { ...d.data, ...updated }
      d.title = d.data.name || form.name.trim()
    } else if (what === 'state') {
      const st = d.status && typeof d.status === 'object' ? d.status : {}
      // 整行覆盖：以原对象为底，只替换表单里的字段
      const payload = {
        ...st,
        location: form.location,
        physical_state: form.physical_state,
        mental_state: form.mental_state,
        resource_state: form.resource_state,
        social_state: form.social_state,
      }
      d.status = await api.updateCharacterState(d.id, payload)
    } else {
      const p = d.profile && typeof d.profile === 'object' ? d.profile : {}
      const payload = { ...p }
      for (const f of editFieldSpec(d.kind, 'profile')) {
        if (f.key === 'aliases') {
          const list = String(form.aliases || '')
            .split(/[,，]/)
            .map((x) => x.trim())
            .filter(Boolean)
          payload.aliases = list
        } else {
          payload[f.key] = form[f.key]
        }
      }
      // 整行覆盖前先确认后端支持的入参形态（characters 会被严格校验枚举）
      if (d.kind === 'characters') d.profile = await api.updateCharacterProfile(d.id, payload)
      else if (d.kind === 'locations') d.profile = await api.updateLocationProfile(d.id, payload)
      else if (d.kind === 'factions') d.profile = await api.updateFactionProfile(d.id, payload)
      else throw new Error(`「${d.kind}」不支持编辑档案`)
    }

    d.editing = null
    // 同步列表缓存（名称/摘要变了，返回列表要能看到）
    syncDetailIntoCache(d)
    toast('已保存')
    render()
  } catch (e) {
    console.error('[app] 保存详情失败', e)
    msg.textContent = `保存失败：${e.message}`
  }
}

/** 把详情页改动的可见字段同步进列表缓存，避免返回列表还是旧值 */
function syncDetailIntoCache(d) {
  const list = state.cache[d.kind]
  if (!Array.isArray(list) || !d.data) return
  const i = list.findIndex((x) => x.id === d.id)
  if (i >= 0) list[i] = { ...list[i], name: d.data.name, summary: d.data.summary, description: d.data.description }
}

/**
 * 关系图：以当前条目为中心，把它直接相关的条目画成一圈。
 *
 * 为什么是这种「中心 + 一环」的画法而不是力导向图：
 * 手机屏幕小，全局关系网画出来会糊成一团；而用户真正想看的是
 * 「这个人和谁有关系」——把一跳关系画清楚比画全更有用。
 * 点某个节点可以直接跳到那个条目的详情，于是可以一环一环逛下去。
 *
 * 数据来自详情页已经加载好的 `d.relations`（`/characters/{id}/relationships`），
 * 不额外请求。
 */
function renderRelationGraph(d) {
  const rels = (d.relations || []).filter((r) => r && r.otherName)
  if (rels.length === 0) {
    return '<div class="hint">这个条目还没有关系记录。建过关系之后这里会画出来。</div>'
  }

  // 节点太多时一圈会挤，超过 12 个只画前 12 个（并说明还有多少）
  const shown = rels.slice(0, 12)
  const size = 340
  const cx = size / 2
  const cy = size / 2
  const radius = 118

  let svg = `<svg viewBox="0 0 ${size} ${size}" class="rel-graph" role="img" aria-label="关系图">`
  // 先画线，再画节点，保证节点压在线上面
  shown.forEach((rel, i) => {
    const angle = (i / shown.length) * Math.PI * 2 - Math.PI / 2
    const x = cx + radius * Math.cos(angle)
    const y = cy + radius * Math.sin(angle)
    const mx = (cx + x) / 2
    const my = (cy + y) / 2
    // 实线 = 当前条目指向对方；虚线 = 对方指向当前条目
    const edgeCls = rel.outgoing === false ? 'rel-edge rel-edge-in' : 'rel-edge'
    svg += `<line x1="${cx}" y1="${cy}" x2="${x.toFixed(1)}" y2="${y.toFixed(1)}" class="${edgeCls}" />`
    svg += `<text x="${mx.toFixed(1)}" y="${(my - 4).toFixed(1)}" class="rel-edge-label">${esc(clipLabel(rel.type, 6))}</text>`
  })
  shown.forEach((rel, i) => {
    const angle = (i / shown.length) * Math.PI * 2 - Math.PI / 2
    const x = cx + radius * Math.cos(angle)
    const y = cy + radius * Math.sin(angle)
    const cls = rel.otherId ? 'rel-node rel-node-tap' : 'rel-node'
    svg += `<g${rel.otherId ? ` data-act="graph-open" data-id="${esc(rel.otherId)}"` : ''}>`
    svg += `<circle cx="${x.toFixed(1)}" cy="${y.toFixed(1)}" r="30" class="${cls}" />`
    svg += `<text x="${x.toFixed(1)}" y="${(y + 4).toFixed(1)}" class="rel-node-label">${esc(clipLabel(rel.otherName, 4))}</text>`
    svg += '</g>'
  })
  svg += `<circle cx="${cx}" cy="${cy}" r="38" class="rel-center" />`
  svg += `<text x="${cx}" y="${(cy + 4).toFixed(1)}" class="rel-center-label">${esc(clipLabel(d.title || '当前', 4))}</text>`
  svg += '</svg>'

  let html = svg
  if (rels.length > shown.length) {
    html += `<div class="hint">关系较多，图中只画了 ${shown.length} 条（共 ${rels.length} 条）。</div>`
  }
  html += '<div class="hint">实线表示「它指向对方」，虚线表示「对方指向它」；点外圈的名字可以跳过去继续看。</div>'
  return html
}

/** 图上文字要短：超长就截断加省略号（SVG 里没法优雅换行） */
function clipLabel(s, n) {
  const t = String(s || '').trim()
  return t.length > n ? t.slice(0, n) + '…' : t
}

/**
 * 历史版本区块。
 *
 * 数据来自服务端 `entity_snapshot`：每次实体被改动**之前**留的档。
 * 在此之前这三个接口是占位桩（永远返回一条编出来的 Initial），
 * 所以这里也顺手把「没有任何改动记录」的情况说清楚。
 */
function renderHistoryBlock(d) {
  if (!d.history) return ''
  if (d.historyError) {
    return `<div class="section-title">历史版本</div>
      <div class="hint status-fail">读取失败：${esc(d.historyError)}</div>`
  }
  const versions = d.history || []
  const profiles = d.profileHistory || []
  const total = versions.length + profiles.length

  let html = `<div class="section-title">历史版本（${total}）</div>`
  if (total === 0) {
    return html + '<div class="hint">还没有改动记录。之后再改这个条目，这里就会留下痕迹。</div>'
  }

  // 两类记录（实体字段 / 档案）合并后按时间倒序——用户只关心「最近改了什么」
  const merged = [
    ...versions.map((v) => ({
      at: v.created_at,
      title: `第 ${v.version} 版${v === versions[versions.length - 1] ? '（当前）' : ''}`,
      sub: versionActorLabel(v.actor),
      changes: v.changes || {},
    })),
    ...profiles.map((p) => ({
      at: p.created_at,
      title: '档案改动',
      sub: `${profileKindLabel(p.kind)} · ${versionActorLabel(p.actor)}`,
      changes: p.changes || {},
    })),
  ].sort((a, b) => String(b.at).localeCompare(String(a.at)))

  html += '<div class="list">'
  for (const item of merged) {
    const keys = Object.keys(item.changes)
    html += `<div class="list-item">
      <div class="li-main">
        <div class="li-title">${esc(item.title)}</div>
        <div class="li-sub">${esc(fmtTime(item.at))} · ${esc(item.sub)}${keys.length ? ' · 改了 ' + esc(keys.map(historyFieldLabel).join('、')) : ''}</div>
      </div>
    </div>`
    // 逐字段展示"改前 → 改后"
    for (const k of keys) {
      const c = item.changes[k] || {}
      html += `<div class="detail-note">${esc(historyFieldLabel(k))}：${esc(clip(c.old))} → ${esc(clip(c.new))}</div>`
    }
  }
  html += '</div>'
  html += '<div class="hint">记录的是每次改动前后的对比（基本信息与档案都包含）。</div>'
  return html
}

/** 档案类别（kind）的中文说明 */
function profileKindLabel(kind) {
  const map = { character: '角色档案 / 状态', location: '地点档案', faction: '势力档案' }
  return map[String(kind || '')] || '档案'
}

/** 历史里「谁改的」的中文标注（认不出来就原样显示，不猜） */
function versionActorLabel(actor) {
  const raw = String(actor || '').trim()
  if (!raw || raw === 'unknown') return '未知来源（早期数据未记录）'
  // 后端写的是 MutationSource::as_str()：user / ai / system
  // （'agent' 是早先那版硬编码留下的历史值，一并认掉，免得老记录显示成英文）
  const map = { user: '手动编辑', ai: 'AI / 提案', agent: 'AI / 提案', system: '系统' }
  return map[raw] || raw
}

/** 历史里字段名的中文说明（实体字段 + 各类档案字段） */
function historyFieldLabel(k) {
  const map = {
    // 实体本身
    name: '名称', summary: '摘要', description: '描述', status: '状态', attributes: '属性',
    // 角色档案
    aliases: '别名', age_range: '年龄', gender: '性别', identity: '身份', appearance: '外貌',
    background_origin: '出身', social_position: '社会地位', core_personality: '核心性格',
    values: '价值观', role_in_story: '剧情作用', narrative_necessity: '叙事必要性', extra: '附加信息',
    // 角色当前状态
    location: '位置', physical_state: '身体状态', mental_state: '心理状态',
    resource_state: '资源状态', social_state: '社交状态', flags: '状态标记',
    // 地点档案
    geography: '地理', population: '人口', economy: '经济', rules: '规则', history: '历史',
    narrative_usage: '叙事用途', location_type: '地点类型', size: '规模', climate: '气候',
    era: '纪元', accessibility: '可达性',
    // 势力档案
    goals: '目标', leader: '首领', resources: '资源', territory: '领地', members: '成员',
    enemies: '敌对', allies: '盟友', internal_conflicts: '内部矛盾', secrets: '秘密',
    modus_operandi: '行事风格',
  }
  // 认不出来的字段原样显示，不猜
  return map[k] || k
}

function clip(v) {
  if (v === null || v === undefined || v === '') return '（空）'
  const s = typeof v === 'object' ? JSON.stringify(v) : String(v)
  return s.length > 40 ? s.slice(0, 40) + '…' : s
}

/** 拉取并展示某实体的历史版本 */
async function loadHistory() {
  const d = state.detail
  if (!d?.id) return
  d.historyLoading = true
  render()
  try {
    // 两条历史来源：实体字段（entity_snapshot）与档案改动（entity_profile_snapshot）
    const [versions, profiles] = await Promise.all([
      api.entityVersions(d.id),
      api.entityProfileHistory(d.id),
    ])
    d.history = versions
    d.profileHistory = profiles
  } catch (e) {
    console.error('[app] 读取历史版本失败', e)
    d.historyError = e.message
  } finally {
    d.historyLoading = false
    render()
  }
}

/** 详情页顶部的编辑入口按钮 */
function renderEditButtons(d) {
  if (!d.data) return ''
  const isEntity = !!ENTITY_UPDATERS[d.kind]
  let html = '<div class="edit-actions">'
  if (isEntity) {
    html += '<button class="mini-btn" data-act="edit-detail" data-what="entity">编辑基础信息</button>'
    html += '<button class="mini-btn" data-act="load-history">历史版本</button>'
    html += '<button class="mini-btn" data-act="toggle-graph">关系图</button>'
  }
  if (d.kind === 'characters' || d.kind === 'locations' || d.kind === 'factions') {
    html += '<button class="mini-btn" data-act="edit-detail" data-what="profile">编辑档案</button>'
  }
  if (d.kind === 'characters') {
    html += '<button class="mini-btn" data-act="edit-detail" data-what="state">编辑当前状态</button>'
  }
  html += '</div>'
  return html
}

function renderDetail() {
  const d = state.detail
  const back = `<div class="list"><div class="list-item" data-act="close-detail">
      <div class="li-main"><div class="li-title">‹ 返回</div></div></div></div>`

  if (d.loading) {
    return back + '<div style="text-align:center;padding:30px"><span class="spinner"></span></div>'
  }
  if (d.error) {
    return back + `<div class="empty"><p>读取失败：${esc(d.error)}</p></div>`
  }

  const it = d.data || {}
  const title = d.title || it.name || it.title || '详情'
  let html = back + `<div class="detail-title">${esc(title)}</div>`

  // ---- 编辑态：整页换成表单（就地改，不跳页）----
  if (d.editing) {
    return html + renderDetailEditor(d)
  }

  // ---- 叙事节点：正文单独成块（可读排版）+ 编辑入口 ----
  if (d.kind === 'nodes') {
    html += renderNodeEditor(d, it)
  }

  // ---- 实体：编辑入口 + 历史版本 ----
  html += renderEditButtons(d)
  // 关系图按需展开，避免详情页默认太长
  if (d.showGraph) {
    html += '<div class="section-title">关系图</div>'
    html += d.relationsLoaded
      ? renderRelationGraph(d)
      : '<div class="hint">关系数据还在读取，稍等一下再点开。</div>'
  }
  html += renderHistoryBlock(d)

  // ---- 基本信息 ----
  const primary = [
    ['类型', detailTypeLabel(d.kind, it)],
    ['摘要', it.summary],
    // 正文已经单独成块显示，这里不再混进「描述」
    ['描述', it.description || (d.kind === 'nodes' ? null : it.content)],
    ['状态', nodeStatusLabel(it.status)],
    // 规则
    ['规则内容', it.rule_content],
    ['适用范围', it.affected_scope],
    ['强制程度', it.enforcement],
    // 关系
    ['关系类型', it.relation_type],
  ]
  html += renderFields('基本信息', primary)

  // ---- 人物档案 ----
  if (d.profile && typeof d.profile === 'object') {
    const p = d.profile
    html += renderFields('档案', [
      ['姓名', p.name],
      ['年龄', p.age_range],
      ['性别', p.gender],
      ['身份', p.identity],
      ['外貌', p.appearance],
      ['出身', p.background_origin],
      ['社会地位', p.social_position],
      ['核心性格', p.core_personality],
      ['剧情作用', p.role_in_story],
      ['地理', p.geography],
      ['人口', p.population],
      ['经济', p.economy],
      ['目标', p.goals],
      ['首领', p.leader],
    ])
  }

  // ---- 角色当前状态（位置/身体/心理/资源/社交/标记）----
  if (d.kind === 'characters' && d.status) {
    const st = d.status
    html += renderFields('当前状态', [
      ['位置', st.location],
      ['身体', st.physical_state],
      ['心理', st.mental_state],
      ['资源', st.resource_state],
      ['社交', st.social_state],
    ])
    const flags = Array.isArray(st.flags) ? st.flags : []
    if (flags.length) {
      html += `<div class="section-title">状态标记（${flags.length}）</div><div class="chips">`
      for (const f of flags) {
        html += `<div class="chip">${esc(typeof f === 'string' ? f : JSON.stringify(f))}</div>`
      }
      html += '</div>'
    }
    if (st.extra && typeof st.extra === 'object' && Object.keys(st.extra).length) {
      const pairs = Object.entries(st.extra).map(([k, v]) => [k, typeof v === 'object' ? JSON.stringify(v) : v])
      html += renderFields('附加状态', pairs)
    }
  }

  // ---- 角色已知信息 ----
  if (d.kind === 'characters' && Array.isArray(d.knowledge) && d.knowledge.length) {
    html += `<div class="section-title">已知信息（${d.knowledge.length}）</div><div class="list">`
    for (const k of d.knowledge) {
      const content = k.content || k.fact || k.description || JSON.stringify(k)
      const level = k.knowledge_level || k.level || ''
      html += `<div class="list-item"><div class="li-main">
        <div class="li-title">${esc(String(content).slice(0, 120))}</div>
        ${level ? `<div class="li-sub">${esc(level)}</div>` : ''}
      </div></div>`
    }
    html += '</div>'
  }

  // ---- 人物关系 ----
  if (d.kind === 'characters') {
    if (d.relations === undefined) {
      html += '<div class="section-title">人物关系</div><div class="hint">读取中…</div>'
    } else if (d.relations.length === 0) {
      html += '<div class="section-title">人物关系</div><div class="hint">暂无关系记录</div>'
    } else {
      html += '<div class="section-title">人物关系（' + d.relations.length + '）</div>'
      html += '<div class="list">'
      for (const r of d.relations) {
        const other = r.otherName || '未知对象'
        const type = r.type || '关联'
        html += `<div class="list-item">
          <div class="li-main">
            <div class="li-title">${esc(other)}</div>
            <div class="li-sub">${esc(type)}</div>
          </div>
          <span class="li-badge">${esc(type)}</span>
        </div>`
        if (r.description) {
          html += `<div class="detail-note">${esc(r.description)}</div>`
        }
      }
      html += '</div>'
    }
  }

  // ---- 地点：相关人物 / 事件 ----
  if (d.kind === 'locations') {
    if (d.entities && d.entities.length) {
      html += `<div class="section-title">相关人物（${d.entities.length}）</div><div class="chips">`
      for (const e of d.entities) {
        html += `<div class="chip" data-act="open-entity" data-id="${esc(e.id || '')}">${esc(e.name || '未命名')}</div>`
      }
      html += '</div>'
    }
    if (d.events && d.events.length) {
      html += `<div class="section-title">相关事件（${d.events.length}）</div><div class="list">`
      for (const e of d.events) {
        html += `<div class="list-item"><div class="li-main">
          <div class="li-title">${esc(e.name || e.title || '未命名')}</div>
          <div class="li-sub">${esc((e.description || '').slice(0, 80))}</div>
        </div></div>`
      }
      html += '</div>'
    }
  }

  // ---- 完整数据（排查用） ----
  html += `
    <div class="section-title">完整数据</div>
    <div class="list"><div class="list-item" data-act="toggle-raw">
      <div class="li-main"><div class="li-title">展开原始 JSON</div></div>
      <span class="li-arrow">›</span></div></div>
    <div class="tool-card" id="raw-json" style="display:none">
      <div class="tool-result" style="display:block">${esc(JSON.stringify(d.data, null, 2))}</div>
    </div>`

  return html
}

/** 叙事节点状态的中文名（后端 `NarrativeNodeStatus` 枚举，认不出来就原样显示） */
function nodeStatusLabel(status) {
  const raw = String(status || '').trim()
  const map = {
    Draft: '草稿',
    Planned: '已规划',
    InProgress: '进行中',
    Completed: '已完成',
    Archived: '已归档',
  }
  return map[raw] || raw
}

/** 可选的节点状态（与后端枚举一一对应） */
const NODE_STATUSES = ['Draft', 'Planned', 'InProgress', 'Completed', 'Archived']

/**
 * 叙事节点的正文块 + 就地编辑表单。
 *
 * 手机上写小说主要靠「改片段」，所以正文必须能直接编辑（与电脑端 Story 页一致）。
 * 编辑不跳页：切成表单 → 保存 → 回读 → 同步列表缓存。
 */
function renderNodeEditor(d, it) {
  if (d.editing) {
    const e = d.editing
    return `
      <div class="section-title">编辑节点</div>
      <div class="field">
        <label>标题</label>
        <input id="ne-title" type="text" value="${esc(e.title)}" />
      </div>
      <div class="field">
        <label>状态</label>
        <select id="ne-status">
          ${NODE_STATUSES.map((s) =>
            `<option value="${s}"${s === e.status ? ' selected' : ''}>${nodeStatusLabel(s)}</option>`).join('')}
        </select>
      </div>
      <div class="field">
        <label>正文</label>
        <textarea id="ne-content" class="edit-area" rows="14" placeholder="在这里写正文…">${esc(e.content)}</textarea>
        <div class="hint">段落之间空一行；单独一行的三个星号会渲染成分隔线。</div>
      </div>
      <button class="btn-primary" data-act="save-node">保存</button>
      <button class="btn-secondary" data-act="cancel-node">取消</button>
      <div class="hint" id="ne-msg"></div>
    `
  }

  const content = String(it.content || '')
  let html = `<div class="section-title">正文${content.trim() ? `（${content.length} 字）` : ''}</div>`
  html += content.trim()
    ? `<div class="detail-content">${renderText(content)}</div>`
    : '<div class="hint">这篇还没有正文。</div>'
  html += `<button class="btn-secondary" data-act="edit-node">${content.trim() ? '编辑正文' : '写正文'}</button>`
  return html
}

/** 进入节点编辑态（把当前值拷成草稿，取消时不影响原数据） */
function startNodeEdit() {
  const d = state.detail
  if (!d?.data) return
  d.editing = {
    title: d.data.title || d.data.name || '',
    content: d.data.content || '',
    status: d.data.status || 'Draft',
  }
  render()
}

/** 退出编辑态（不保存） */
function cancelNodeEdit() {
  if (!state.detail) return
  state.detail.editing = null
  render()
}

/** 保存节点编辑：标题 / 状态 / 正文 */
async function saveNodeEdit() {
  const d = state.detail
  if (!d?.editing) return
  const msg = $('ne-msg')
  const title = $('ne-title').value.trim()
  const content = $('ne-content').value
  const status = $('ne-status').value

  // 明确校验，不静默放过
  if (!title) {
    msg.textContent = '标题不能为空'
    return
  }

  msg.textContent = '正在保存…'
  try {
    const updated = await api.updateNode(d.id, { title, content, status })
    d.data = updated && typeof updated === 'object' ? updated : { ...d.data, title, content, status }
    d.title = d.data.title || d.data.name || title
    d.editing = null

    // 同步故事列表缓存，返回列表时不必重新拉
    const list = state.cache.nodes
    if (Array.isArray(list)) {
      const i = list.findIndex((x) => x.id === d.id)
      if (i >= 0) list[i] = { ...list[i], ...d.data }
    }
    toast('已保存')
    render()
  } catch (e) {
    console.error('[app] 保存节点失败', e)
    msg.textContent = `保存失败：${e.message}`
  }
}

/** 新建叙事节点（卷/弧线/章节/场景），正文随后在详情页里写 */
async function createNodePrompt() {
  const types = [
    ['Chapter', '章节'],
    ['Scene', '场景'],
    ['Volume', '卷'],
    ['Arc', '弧线'],
  ]
  const picked = prompt(`新建什么？\n${types.map(([v, label], i) => `${i + 1}. ${label}（${v}）`).join('\n')}\n\n输入序号，直接回车 = 章节`, '1')
  if (picked === null) return
  const idx = parseInt(picked, 10)
  if (!Number.isFinite(idx) || idx < 1 || idx > types.length) {
    toast('序号不在范围内')
    return
  }
  const nodeType = types[idx - 1][0]

  const title = prompt('标题（必填）')
  if (title === null) return
  if (!title.trim()) {
    toast('标题不能为空')
    return
  }

  try {
    const created = await api.createNode(state.project.id, {
      node_type: nodeType,
      title: title.trim(),
    })
    // 列表可能正开着，重新拉一次保证顺序/父子关系正确
    state.cache.nodes = await api.listNodes(state.project.id)
    state.cache.nodes = Array.isArray(state.cache.nodes)
      ? state.cache.nodes
      : (state.cache.nodes?.nodes ?? [])
    toast('已创建，点进去写正文')
    render()
    if (created?.id) {
      await openDetail('nodes', created.id)
    }
  } catch (e) {
    console.error('[app] 新建节点失败', e)
    toast('创建失败：' + e.message)
  }
}

/** 渲染一组「标签 → 值」字段，跳过空值 */
/**
 * 把一个字段值渲染成可读文本。
 *
 * 档案里不少字段是**结构化的**——`social_position` 是对象
 * （`{rank, authority_level, social_access}`）、`aliases` 是数组、
 * `drive` / `capabilities` / `arc_potential` 是嵌套对象。
 * 原先直接 `String(val)`，于是详情页上出现一行 `[object Object]`：
 * 既不报错也不为空，只是**悄悄显示成没用的东西**。
 *
 * 规则：
 *   - 数组 → 元素用「、」连接（对象元素优先取它说明内容的那个字段）
 *   - 对象 → 有值的键值对用「；」连接，空值跳过
 *   - 其余 → 转字符串
 */
function renderFieldValue(v) {
  if (v === null || v === undefined) return ''
  if (Array.isArray(v)) {
    return v.map((x) => renderFieldValue(x)).filter(Boolean).join('、')
  }
  if (typeof v === 'object') {
    for (const key of ['description', 'content', 'value', 'text', 'name', 'current', 'desire']) {
      const inner = v[key]
      if (typeof inner === 'string' && inner.trim()) return inner.trim()
    }
    const parts = []
    for (const [k, val] of Object.entries(v)) {
      const rendered = renderFieldValue(val)
      if (rendered) parts.push(`${k}：${rendered}`)
    }
    return parts.join('；')
  }
  return String(v)
}

/** 渲染一组「标签 → 值」字段，跳过空值 */
function renderFields(title, pairs) {
  const rows = pairs
    .map(([label, val]) => [label, renderFieldValue(val)])
    .filter(([, v]) => v !== null && v !== undefined && String(v).trim() !== '')
  if (rows.length === 0) return ''
  let html = `<div class="section-title">${esc(title)}</div><div class="list">`
  for (const [label, val] of rows) {
    html += `<div class="list-item"><div class="li-main">
      <div class="li-sub">${esc(label)}</div>
      <div class="li-title">${esc(String(val))}</div>
    </div></div>`
  }
  html += '</div>'
  return html
}

function renderNoProject() {
  return `
    <div class="empty">
      <div class="seal">笔</div>
      <h3>还没有项目</h3>
      <p>创建第一个项目，开始和向导一起构建你的世界。</p>
      <button class="btn-primary" data-act="create-project">新建项目</button>
    </div>`
}

function renderAgent() {
  const msgs = state.messages
  let html = ''

  if (msgs.length === 0) {
    html += `
      <div class="empty">
        <div class="seal">笔</div>
        <h3>开始你的创作之旅</h3>
        <p>用自然语言描述想法，向导会逐步帮你构建世界观、角色与剧情。</p>
        <div class="chips">
          <button class="chip" data-act="suggest" data-text="帮我构思一个都市异能题材的故事开端">帮我构思一个都市异能题材的故事开端</button>
          <button class="chip" data-act="suggest" data-text="我有个想法，先帮我确定故事的前提和核心冲突">我有个想法，先帮我确定故事的前提和核心冲突</button>
          <button class="chip" data-act="suggest" data-text="先创建 3 个主要角色，一男一女一反派">先创建 3 个主要角色，一男一女一反派</button>
        </div>
      </div>`
  } else {
    html += '<div class="msg-col">'
    for (const m of msgs) {
      if (m.role === 'user') {
        html += `<div class="msg user" data-msg-id="${esc(m.id)}">`
          + `<div class="bubble">${esc(m.content)}</div>`
          // 操作条：靠右排列，用图标而不是文字。
          // 原先这里是个裸 <button class="copy-btn">复制</button>，而 .copy-btn
          // 根本没有样式 —— 于是在 flex 布局里被拉满整行、还带着浏览器默认的白底。
          + `<div class="msg-tools">`
          + `<button class="icon-btn" data-act="copy" data-msg-id="${esc(m.id)}" aria-label="复制" title="复制">${COPY_ICON}</button>`
          + `</div>`
          + `</div>`
      } else {
        html += `<div class="msg ai" data-msg-id="${esc(m.id)}">`
        if (m.content) html += `<div class="bubble">${renderText(m.content)}</div>`
        for (const t of m.tools || []) html += renderToolCard(t)
        if (m.content) {
          html += `<div class="msg-tools">`
            + `<button class="icon-btn" data-act="copy" data-msg-id="${esc(m.id)}" aria-label="复制" title="复制">${COPY_ICON}</button>`
            + `</div>`
        }
        html += '</div>'
      }
    }
    if (state.thinking) {
      html += `<div class="thinking"><span class="pulse"></span><span>向导正在思考…</span></div>`
    }
    html += '</div>'
    if (state.usage) html += renderUsage()
  }

  return html
}

function renderToolCard(t) {
  const name = t.name || t.tool || '工具'
  const input = t.input ?? t.args ?? null
  const result = t.result ?? t.output ?? null
  const desc = input ? summarize(input) : ''
  const resultText = result != null
    ? (typeof result === 'string' ? result : JSON.stringify(result, null, 2))
    : ''
  return `
    <div class="tool-card" data-act="toggle-tool">
      <div class="tool-head">
        <span>⚙</span><span>${esc(name)}</span>
        ${resultText ? '<span class="tool-toggle">详情</span>' : ''}
      </div>
      ${desc ? `<div class="tool-desc">${esc(desc)}</div>` : ''}
      ${resultText ? `<div class="tool-result">${esc(resultText)}</div>` : ''}
    </div>`
}

function summarize(input) {
  if (typeof input === 'string') return input.slice(0, 120)
  if (input && typeof input === 'object') {
    const keys = ['name', 'title', 'question', 'summary', 'description', 'content']
    for (const k of keys) {
      if (input[k]) return String(input[k]).slice(0, 120)
    }
    return JSON.stringify(input).slice(0, 120)
  }
  return ''
}

function renderUsage() {
  const u = state.usage
  const total = u.total_tokens ?? u.prompt_tokens ?? 0
  const limit = u.context_limit ?? 0
  if (!total) return ''
  const pct = limit ? Math.min(100, Math.round((total / limit) * 100)) : 0
  const level = pct > 85 ? 'danger' : pct > 60 ? 'warn' : ''
  return `
    <div class="context-meter">
      <div class="meter-track"><div class="meter-fill ${level}" style="width:${pct}%"></div></div>
      <span>${total}${limit ? ' / ' + limit : ''} tokens</span>
    </div>`
}

/**
 * 搜索框。
 *
 * 输入时**不重渲染输入框本身**（只更新列表容器）——重建 input 会打断中文输入法
 * 的候选状态，在手机上表现为「打一个字就断」。所以这里给列表一个固定 id，
 * 由 `bindSearchInputs()` 负责局部更新。
 */
function renderSearchBar(scope, placeholder) {
  const value = state.search[scope] || ''
  // 「×」始终渲染、用 hidden 控制显示：局部重渲染只换列表容器，
  // 搜索栏不会跟着刷新，所以不能靠「有值才渲染」来决定它出不出现
  return `<div class="search-bar">
    <input class="search-input" id="search-${scope}" type="search"
           placeholder="${esc(placeholder)}" value="${esc(value)}" />
    <button class="search-clear" data-act="clear-search" data-scope="${scope}"${value ? '' : ' hidden'}>×</button>
  </div>`
}

/** 搜索框输入 → 只更新对应列表容器 */
function bindSearchInputs() {
  document.querySelectorAll('.search-input').forEach((input) => {
    const scope = input.id.replace('search-', '')
    input.addEventListener('input', () => {
      state.search[scope] = input.value
      // 同步「×」的显隐（搜索栏本身不重渲染）
      const clear = input.parentElement.querySelector('.search-clear')
      if (clear) clear.hidden = !input.value
      const box = document.getElementById(`${scope}-list`)
      if (!box) return
      box.innerHTML = scope === 'world' ? renderWorldList() : renderStoryList()
      bindActs(box)
    })
  })
}

/** 世界 Tab：各类数据各自的展示字段 */
const WORLD_SECTIONS = [
  { key: 'characters', label: '人物', kind: 'characters' },
  { key: 'locations', label: '地点', kind: 'locations' },
  { key: 'factions', label: '势力', kind: 'factions' },
  { key: 'items', label: '物品', kind: 'items' },
  { key: 'rules', label: '世界规则', kind: 'rules' },
  { key: 'relations', label: '关系', kind: 'relations' },
]

function renderWorld() {
  const w = state.world
  if (!w) return '<div class="empty"><p>该项目还没有主世界</p></div>'

  // 搜索框单独放在列表之外：输入时只更新 #world-list，
  // 不重建输入框本身（重建会打断中文输入法的候选状态）
  return `
    ${renderSearchBar('world', '搜索人物 / 地点 / 势力 / 物品 / 规则…')}
    <div id="world-list">${renderWorldList()}</div>
  `
}

/** 世界 Tab 的列表（可被搜索过滤后局部重渲染） */
function renderWorldList() {
  const c = state.cache
  const q = state.search.world.trim().toLowerCase()
  let html = ''
  let shown = 0

  for (const sec of WORLD_SECTIONS) {
    const data = c[sec.key]
    // 条目数显示在标题上，方便一眼看出这个项目有什么
    const count = Array.isArray(data) ? `（${data.length}）` : ''
    html += `<div class="section-title">${sec.label}${count}</div>`

    if (data === null || data === undefined) {
      html += `<div class="hint">加载中…</div>`
      continue
    }
    if (data.length === 0) {
      html += `<div class="hint">暂无${sec.label}</div>`
      continue
    }

    const hits = q ? data.filter((it) => worldItemMatches(it, q)) : data
    if (q && hits.length === 0) {
      continue   // 搜索时把没有命中的分类整个隐去，免得一屏全是「无匹配」
    }
    shown += hits.length

    html += '<div class="list">'
    for (const it of hits) {
      html += renderWorldItem(sec.kind, it)
    }
    html += '</div>'
  }

  if (q && shown === 0) {
    return `<div class="empty"><p>没有匹配「${esc(state.search.world.trim())}」的内容</p></div>`
  }
  return html
}

/**
 * 一条世界数据是否命中搜索词。
 *
 * 匹配范围覆盖这类数据实际有意义的文本字段：名称、摘要、描述、正文内容。
 * 规则/关系的正文藏在 rule_content / description 里，单独带上。
 */
function worldItemMatches(it, q) {
  const hay = [
    it.name,
    it.title,
    it.summary,
    it.description,
    it.content,
    it.rule_content,
    it.affected_scope,
    it.relation_type,
    it.entity_type,
    it.status,
  ]
    .filter(Boolean)
    .join(' ')
    .toLowerCase()
  return hay.includes(q)
}

/**
 * 从世界缓存里判断某个实体属于哪一类（角色 / 地点 / 势力 / 物品）。
 *
 * 关系图上的节点只有 id，点开详情需要知道 kind。
 * 找不到时返回 null，由调用方提示，而不是猜一个。
 */
function graphKindOf(id) {
  const c = state.cache
  for (const [kind, key] of [['characters', 'characters'], ['locations', 'locations'], ['factions', 'factions'], ['items', 'items']]) {
    if (Array.isArray(c[key]) && c[key].some((x) => x.id === id)) return kind
  }
  return null
}

/**
 * 详情页「类型」行的文案。
 *
 * 实体（人物 / 地点 / 势力 / 物品）后端给的是英文类型名（Character / Location …），
 * 这里翻成中文；叙事节点用的是另一套枚举；其余类型没有「类型」这个概念，返回空。
 * 认不出来的一律原样显示，不猜。
 */
function detailTypeLabel(kind, it) {
  if (!it) return ''
  if (kind === 'nodes') return it.node_type ? nodeTypeLabel(it.node_type) : ''
  if (kind === 'rules' || kind === 'relations' || kind === 'storylines' || kind === 'foreshadows') {
    return ''
  }
  const raw = it.entity_type || it.type
  return raw ? extractTypeLabel(raw) : ''
}

/**
 * 详情页标题。
 *
 * 不能一律用 `name || title`——规则和关系**这两个字段都没有**，
 * 于是标题会显示成毫无意义的「详情」。按类型取真正承载内容的字段。
 */
function detailTitle(kind, data) {
  if (!data) return '详情'
  if (kind === 'rules') {
    const content = String(data.rule_content || data.content || '').trim()
    if (content) return content.length > 24 ? content.slice(0, 24) + '…' : content
    return '世界规则'
  }
  if (kind === 'relations') {
    const src = data.source_name || shortId(data.source_entity_id)
    const tgt = data.target_name || shortId(data.target_entity_id)
    return `${src} → ${tgt}`
  }
  return data.name || data.title || '详情'
}

/** 只能拿到 uuid 时显示前 8 位，便于区分不同记录 */
function shortId(id) {
  const s = String(id || '')
  return s ? s.slice(0, 8) : '?'
}

/** 按类型渲染世界里的一条数据 */
function renderWorldItem(kind, it) {
  if (kind === 'rules') {
    // 规则：内容是主体，另外标出适用范围与强制程度
    const content = it.rule_content || it.content || ''
    const scope = it.affected_scope || ''
    const enforce = it.enforcement || ''
    return `<div class="list-item" data-act="detail" data-kind="rules" data-id="${esc(it.id)}">
      <div class="li-main">
        <div class="li-title rule-text">${esc(String(content).slice(0, 140))}</div>
        <div class="li-sub">${esc(scope.slice(0, 40))}${enforce ? ' · ' + esc(enforce) : ''}</div>
      </div>
      <span class="li-arrow">›</span>
    </div>`
  }

  if (kind === 'relations') {
    // 关系的核心信息是「谁和谁」——标题给两端名字，类型放到副标题。
    // （服务端现在会返回 source_name / target_name；旧数据缺失时退回 uuid 前 8 位，
    //   至少能看出这是两条不同的记录。）
    const type = it.relation_type || it.type || '关联'
    const desc = it.description || ''
    const src = it.source_name || shortId(it.source_entity_id)
    const tgt = it.target_name || shortId(it.target_entity_id)
    return `<div class="list-item" data-act="detail" data-kind="relations" data-id="${esc(it.id)}">
      <div class="li-main">
        <div class="li-title">${esc(src)} → ${esc(tgt)}</div>
        <div class="li-sub">${esc(type)}${desc ? ' · ' + esc(String(desc).slice(0, 50)) : ''}</div>
      </div>
    </div>`
  }

  const name = it.name || it.title || '未命名'
  const sub = it.summary || it.description || ''
  return `<div class="list-item" data-act="detail" data-kind="${esc(kind)}" data-id="${esc(it.id)}">
    <div class="li-main">
      <div class="li-title">${esc(name)}</div>
      ${sub ? `<div class="li-sub">${esc(String(sub).slice(0, 60))}</div>` : ''}
    </div>
    <span class="li-arrow">›</span>
  </div>`
}

function renderStory() {
  // 同世界 Tab：搜索框在列表之外，输入时只更新 #story-list
  return `
    ${renderSearchBar('story', '搜索章节 / 剧情线 / 伏笔…')}
    <div id="story-list">${renderStoryList()}</div>
  `
}

/** 故事 Tab 的列表（可被搜索过滤后局部重渲染） */
function renderStoryList() {
  const c = state.cache
  const q = state.search.story.trim().toLowerCase()
  const rows = [
    ['nodes', '故事结构', c.nodes],
    ['storylines', '剧情线', c.storylines],
    ['foreshadows', '伏笔', c.foreshadows],
  ]
  let html = ''
  let shown = 0
  for (const [key, label, data] of rows) {
    html += `<div class="section-title">${label}</div>`
    if (data === null) {
      html += `<div class="list"><div class="list-item" data-act="load-story" data-key="${key}"><div class="li-main"><div class="li-title">点击加载</div></div><span class="li-arrow">›</span></div></div>`
      continue
    }

    // 故事结构可以新建（剧情线 / 伏笔的创建放到各自的详情页里做）
    if (key === 'nodes') {
      html += '<button class="btn-secondary" data-act="new-node">新建卷 / 章节 / 场景</button>'
    }

    if (data.length === 0) {
      if (!q) {
        html += '<div class="list" style="margin-top:10px"><div class="list-item"><div class="li-main"><div class="li-sub">暂无数据</div></div></div></div>'
      }
      continue
    }

    const hits = q ? data.filter((it) => storyItemMatches(it, q)) : data
    if (q && hits.length === 0) continue
    shown += hits.length

    html += '<div class="list" style="margin-top:10px">'
    for (const it of hits) {
      const name = it.title || it.name || '未命名'
      const sub = storySubtitle(key, it)
      html += `<div class="list-item" data-act="detail" data-kind="${key}" data-id="${esc(it.id)}">
        <div class="li-main">
          <div class="li-title">${esc(String(name).slice(0, 60))}</div>
          ${sub ? `<div class="li-sub">${esc(sub)}</div>` : ''}
        </div>
        <span class="li-arrow">›</span>
      </div>`
    }
    html += `</div>`
  }

  if (q && shown === 0) {
    return `<div class="empty"><p>没有匹配「${esc(state.search.story.trim())}」的内容</p></div>`
  }
  return html
}

/** 一条故事数据是否命中搜索词（正文也要能搜到） */
function storyItemMatches(it, q) {
  const hay = [
    it.title,
    it.name,
    it.description,
    it.content,
    it.node_type,
    it.status,
  ]
    .filter(Boolean)
    .join(' ')
    .toLowerCase()
  return hay.includes(q)
}

/** 叙事节点类型的中文名（后端 `NarrativeNodeType`，认不出来就原样显示） */
function nodeTypeLabel(type) {
  const raw = String(type || '').trim()
  const map = {
    Volume: '卷',
    Arc: '弧线',
    Sequence: '序列',
    Chapter: '章节',
    Scene: '场景',
    Beat: '节拍',
    Storyline: '故事线',
    SubArc: '子弧线',
    Thread: '线索',
    Act: '幕',
  }
  return map[raw] || raw
}

/** 故事列表项的副标题：把该类型用得上的信息拼一行 */
function storySubtitle(key, it) {
  const parts = []
  if (key === 'nodes') {
    if (it.node_type) parts.push(nodeTypeLabel(it.node_type))
    if (it.status) parts.push(nodeStatusLabel(it.status))
    const len = String(it.content || '').length
    parts.push(len > 0 ? `${len} 字` : '未写正文')
  } else if (key === 'storylines') {
    if (it.importance) parts.push(it.importance)
    if (it.tone) parts.push(it.tone === 'dark' ? '暗线' : '明线')
    if (it.visibility) parts.push(it.visibility === 'hidden' ? '隐藏' : '可见')
  } else if (key === 'foreshadows') {
    if (it.importance) parts.push(it.importance)
    if (it.hint_level) parts.push(`提示强度 ${it.hint_level}`)
  }
  return parts.filter(Boolean).join(' · ')
}

function renderMore() {
  const c = state.cache
  const n = (v) => (Array.isArray(v) ? String(v.length) : '—')
  const proj = state.project

  return `
    <div class="section-title">当前项目</div>
    <div class="list">
      <div class="list-item">
        <div class="li-main"><div class="li-title">${esc(proj?.name || '')}</div>
          <div class="li-sub">${esc(proj?.status || '')}${proj?.language ? ' · ' + esc(proj.language) : ''}</div></div>
      </div>
      <div class="list-item" data-act="switch-project">
        <div class="li-main"><div class="li-title">切换项目</div>
          <div class="li-sub">共 ${state.projects.length} 个项目</div></div>
        <span class="li-arrow">›</span>
      </div>
      <div class="list-item" data-act="create-project">
        <div class="li-main"><div class="li-title">新建项目</div></div>
        <span class="li-arrow">›</span>
      </div>
    </div>

    <div class="section-title">内容概览</div>
    <div class="list">
      <div class="list-item"><div class="li-main"><div class="li-title">人物</div></div><span class="li-badge">${n(c.characters)}</span></div>
      <div class="list-item"><div class="li-main"><div class="li-title">地点</div></div><span class="li-badge">${n(c.locations)}</span></div>
      <div class="list-item"><div class="li-main"><div class="li-title">势力</div></div><span class="li-badge">${n(c.factions)}</span></div>
      <div class="list-item"><div class="li-main"><div class="li-title">故事节点</div></div><span class="li-badge">${n(c.nodes)}</span></div>
      <div class="list-item"><div class="li-main"><div class="li-title">剧情线</div></div><span class="li-badge">${n(c.storylines)}</span></div>
      <div class="list-item"><div class="li-main"><div class="li-title">伏笔</div></div><span class="li-badge">${n(c.foreshadows)}</span></div>
    </div>

    <div class="section-title">工具</div>
    <div class="list">
      <div class="list-item" data-act="load-misc" data-key="snapshots">
        <div class="li-main"><div class="li-title">快照</div>
          <div class="li-sub">${c.snapshots ? c.snapshots.length + ' 个' : '项目状态存档与回滚'}</div></div>
        <span class="li-arrow">›</span>
      </div>
      <div class="list-item" data-act="load-misc" data-key="proposals">
        <div class="li-main"><div class="li-title">AI 提案</div>
          <div class="li-sub">${c.proposals ? c.proposals.length + ' 条' : '待审批的变更提案'}</div></div>
        <span class="li-arrow">›</span>
      </div>
      <div class="list-item" data-act="load-misc" data-key="events">
        <div class="li-main"><div class="li-title">历史事件</div>
          <div class="li-sub">${c.events ? c.events.length + ' 条' : '已发生的事件记录'}</div></div>
        <span class="li-arrow">›</span>
      </div>
    </div>

    <div class="section-title">系统</div>
    <div class="list">
      <div class="list-item" data-act="open-extract">
        <div class="li-main"><div class="li-title">AI 文本抽取</div>
          <div class="li-sub">把一段正文抽成人物 / 地点 / 关系草稿</div></div>
        <span class="li-arrow">›</span>
      </div>
      <div class="list-item" data-act="settings">
        <div class="li-main"><div class="li-title">AI 设置</div>
          <div class="li-sub">API Key / 模型 / 网关地址</div></div>
        <span class="li-arrow">›</span>
      </div>
      <div class="list-item" data-act="recheck-engine">
        <div class="li-main"><div class="li-title">重新检测引擎</div>
          <div class="li-sub">${state.engine.ok ? '已就绪' : esc(state.engine.error || '未连接')}</div></div>
        <span class="li-arrow">›</span>
      </div>
      <div class="list-item" data-act="export-backup">
        <div class="li-main"><div class="li-title">备份手机数据</div>
          <div class="li-sub">把全部数据存成一个文件，防止丢失</div></div>
        <span class="li-arrow">›</span>
      </div>
      <div class="list-item" data-act="about">
        <div class="li-main"><div class="li-title">关于</div>
          <div class="li-sub">本地引擎 · 数据仅存本机</div></div>
        <span class="li-arrow">›</span>
      </div>
    </div>

    ${renderMiscLists()}
  `
}

/** 快照 / 提案 / 事件这类列表就地展开在「更多」页 */
function renderMiscLists() {
  const c = state.cache
  let html = ''

  // ---- 快照：可新建、可回滚、可删除 ----
  if (Array.isArray(c.snapshots)) {
    html += `<div class="section-title">快照（${c.snapshots.length}）</div>
      <button class="btn-secondary" data-act="new-snapshot">新建快照</button>`
    if (c.snapshots.length === 0) {
      html += '<div class="list" style="margin-top:10px"><div class="list-item"><div class="li-main"><div class="li-sub">还没有快照。写到一个阶段存一份，写坏了能退回来。</div></div></div></div>'
    } else {
      html += '<div class="list" style="margin-top:10px">'
      for (const it of c.snapshots.slice(0, 50)) {
        const label = esc(String(it.name || it.title || String(it.id).slice(0, 8)))
        const when = it.story_time ? esc(String(it.story_time)) : esc(fmtTime(it.created_at))
        html += `<div class="list-item">
          <div class="li-main">
            <div class="li-title">${label}</div>
            <div class="li-sub">${when}</div>
          </div>
          <button class="mini-btn" data-act="restore-snapshot" data-id="${esc(it.id)}">恢复</button>
          <button class="mini-btn danger" data-act="delete-snapshot" data-id="${esc(it.id)}">删除</button>
        </div>`
      }
      html += '</div>'
    }
  }

  // ---- AI 提案：可批准、可拒绝、可校验 ----
  if (Array.isArray(c.proposals)) {
    html += `<div class="section-title">AI 提案（${c.proposals.length}）</div>`
    if (c.proposals.length === 0) {
      html += '<div class="list"><div class="list-item"><div class="li-main"><div class="li-sub">暂无提案</div></div></div></div>'
    } else {
      html += '<div class="list">'
      for (const it of c.proposals.slice(0, 50)) {
        const status = String(it.status || '')
        const pending = isPendingProposal(status)
        const changes = Array.isArray(it.changes) ? it.changes : []
        const desc = it.summary || it.title || it.reason || changes[0]?.description || String(it.id).slice(0, 8)
        html += `<div class="list-item">
          <div class="li-main">
            <div class="li-title">${esc(String(desc))}</div>
            <div class="li-sub">${proposalStatusLabel(status)} · ${esc(fmtTime(it.created_at))}</div>
          </div>
          ${pending
            ? `<button class="mini-btn" data-act="accept-proposal" data-id="${esc(it.id)}">批准</button>
               <button class="mini-btn danger" data-act="reject-proposal" data-id="${esc(it.id)}">拒绝</button>`
            : '<span class="li-badge">已处理</span>'}
        </div>`
      }
      html += '</div>'
      html += `<div class="hint" style="margin-top:8px">批准后改动会立即写入正典（Canon），无法一键撤销；不确定时先「拒绝」。</div>`
    }
  }

  // ---- 历史事件：只读 ----
  if (Array.isArray(c.events)) {
    html += `<div class="section-title">历史事件（${c.events.length}）</div>`
    if (c.events.length === 0) {
      html += '<div class="list"><div class="list-item"><div class="li-main"><div class="li-sub">暂无数据</div></div></div></div>'
    } else {
      html += '<div class="list">'
      for (const it of c.events.slice(0, 50)) {
        html += `<div class="list-item"><div class="li-main">
          <div class="li-title">${esc(String(it.name || it.title || String(it.id).slice(0, 8)))}</div>
          <div class="li-sub">${esc(it.event_time ? String(it.event_time) : fmtTime(it.created_at))}</div>
        </div></div>`
      }
      html += '</div>'
    }
  }

  return html
}

/**
 * 提案状态是否仍可操作。
 *
 * 状态名来自后端 `ProposedChangeStatus::description()`（英文枚举名）：
 *   Draft / Validating / Valid / PendingApproval  → 还没定论，可批准或拒绝
 *   Approved / Committed / Applied / Rejected / Invalid / Conflicted / Expired / Failed
 *                                                 → 已有结论，不再给按钮
 * 这份名单在 `crates/sqlite-engine/tests/proposals_snapshots.rs` 里有断言守着。
 */
const ACTIONABLE_PROPOSAL_STATES = ['Draft', 'Validating', 'Valid', 'PendingApproval']

function isPendingProposal(status) {
  return ACTIONABLE_PROPOSAL_STATES.includes(String(status || '').trim())
}

/** 把后端的英文状态名翻成中文；认不出来就原样显示（不猜） */
function proposalStatusLabel(status) {
  const raw = String(status || '').trim()
  const map = {
    Draft: '草稿',
    Validating: '校验中',
    Valid: '待审批',
    PendingApproval: '待人工审批',
    Approved: '已批准',
    Committed: '已提交',
    Applied: '已生效',
    Invalid: '校验失败',
    Rejected: '已拒绝',
    Conflicted: '版本冲突',
    Expired: '已过期',
    Failed: '提交失败',
  }
  return map[raw] || raw || '未知状态'
}

// ---------------------------------------------------------------- 事件绑定

function bindViewEvents() {
  bindActs(els.view)
}

/**
 * 给某个容器里的 `[data-act]` 元素绑定点击处理。
 *
 * 单独抽出来是为了「局部重渲染」：搜索框输入时只更新列表容器，
 * 输入框本身不重建——否则中文输入法的候选状态会被打断。
 */
function bindActs(root) {
  root.querySelectorAll('[data-act]').forEach((el) => {
    el.addEventListener('click', async () => {
      const act = el.dataset.act
      try {
        if (act === 'suggest') {
          await send(el.dataset.text)
        } else if (act === 'copy') {
          await copyMessage(el.dataset.msgId)
        } else if (act === 'toggle-tool') {
          el.classList.toggle('open')
        } else if (act === 'create-project') {
          await askCreateProject()
        } else if (act === 'load-world') {
          await loadWorldData(el.dataset.key)
        } else if (act === 'load-story') {
          await loadStoryData(el.dataset.key)
        } else if (act === 'detail') {
          await openDetail(el.dataset.kind, el.dataset.id)
        } else if (act === 'close-detail') {
          state.detail = null
          render()
          syncBackGuard()
        } else if (act === 'open-entity') {
          await openDetail('characters', el.dataset.id)
        } else if (act === 'toggle-raw') {
          const n = document.getElementById('raw-json')
          if (n) n.style.display = n.style.display === 'none' ? 'block' : 'none'
        } else if (act === 'switch-project') {
          await askSwitchProject()
        } else if (act === 'recheck-engine') {
          const ok = await checkEngine()
          toast(ok ? '引擎已连接' : '引擎仍未连接')
        } else if (act === 'load-misc') {
          await loadMisc(el.dataset.key)
        } else if (act === 'accept-proposal') {
          await acceptProposalById(el.dataset.id)
        } else if (act === 'reject-proposal') {
          await rejectProposalById(el.dataset.id)
        } else if (act === 'new-snapshot') {
          await createSnapshotPrompt()
        } else if (act === 'open-extract') {
          renderExtractView()
        } else if (act === 'clear-search') {
          const scope = el.dataset.scope
          state.search[scope] = ''
          render()
          document.getElementById(`search-${scope}`)?.focus()
        } else if (act === 'toggle-graph') {
          state.detail.showGraph = !state.detail.showGraph
          render()
        } else if (act === 'graph-open') {
          // 点外圈节点：打开那个实体的详情（同一种详情页，继续探索）
          const kind = graphKindOf(el.dataset.id)
          if (!kind) {
            // 找不到归属就不硬跳——猜一个 kind 会让详情页读到空数据
            toast('这个条目不在已加载的列表里，先回列表刷新一下')
            return
          }
          await openDetail(kind, el.dataset.id)
        } else if (act === 'load-history') {
          await loadHistory()
        } else if (act === 'edit-detail') {
          startDetailEdit(el.dataset.what)
        } else if (act === 'cancel-detail-edit') {
          cancelDetailEdit()
        } else if (act === 'save-detail-edit') {
          await saveDetailEdit()
        } else if (act === 'edit-node') {
          startNodeEdit()
        } else if (act === 'cancel-node') {
          cancelNodeEdit()
        } else if (act === 'save-node') {
          await saveNodeEdit()
        } else if (act === 'new-node') {
          await createNodePrompt()
        } else if (act === 'restore-snapshot') {
          await restoreSnapshotById(el.dataset.id)
        } else if (act === 'delete-snapshot') {
          await deleteSnapshotById(el.dataset.id)
        } else if (act === 'export-backup') {
          await backupPhoneData()
        } else if (act === 'about') {
          toast('Novel Engine 手机版 · 引擎与数据都在本机')
        } else if (act === 'settings') {
          state.showingSettings = true
          renderSettings()
        }
      } catch (e) {
        state.error = e.message
        render()
      }
    })
  })
}

async function loadWorldData(key) {
  if (!state.world) { toast('没有主世界'); return }
  const wid = state.world.id
  if (key === 'characters') state.cache.characters = await api.listCharacters(wid)
  else if (key === 'locations') state.cache.locations = await api.listLocations(wid)
  else if (key === 'factions') state.cache.factions = await api.listFactions(wid)
  render()
}

/** 一次性加载世界 Tab 的全部数据 */
async function loadWorldAll() {
  if (!state.world) return
  const wid = state.world.id
  const c = state.cache
  const jobs = []
  if (c.characters === null) jobs.push(api.listCharacters(wid).then((d) => { c.characters = d }))
  if (c.locations === null) jobs.push(api.listLocations(wid).then((d) => { c.locations = d }))
  if (c.factions === null) jobs.push(api.listFactions(wid).then((d) => { c.factions = d }))
  // 物品按类型过滤（实体接口支持 type 参数）
  if (c.items === null) {
    jobs.push(api.get(`/worlds/${wid}/entities?type=Item`).then((d) => { c.items = d }))
  }
  if (c.rules === null) jobs.push(api.listRules(wid).then((d) => { c.rules = d }))
  if (c.relations === null) jobs.push(api.listRelations(wid).then((d) => { c.relations = d }))
  if (jobs.length) {
    await Promise.all(jobs)
    render()
  }
}

/** 一次性加载故事 Tab 的全部数据 */
async function loadStoryAll() {
  const pid = state.project.id
  const c = state.cache
  const jobs = []
  if (c.nodes === null) jobs.push(api.listNodes(pid).then((d) => { c.nodes = Array.isArray(d) ? d : (d?.nodes ?? d?.items ?? []) }))
  if (c.storylines === null) jobs.push(api.listStorylines(pid).then((d) => { c.storylines = Array.isArray(d) ? d : (d?.storylines ?? []) }))
  if (c.foreshadows === null) jobs.push(api.listForeshadows(pid).then((d) => { c.foreshadows = Array.isArray(d) ? d : (d?.foreshadows ?? []) }))
  if (jobs.length) {
    await Promise.all(jobs)
    render()
  }
}

async function loadStoryData(key) {
  const pid = state.project.id
  if (key === 'nodes') state.cache.nodes = await api.listNodes(pid)
  else if (key === 'storylines') state.cache.storylines = await api.listStorylines(pid)
  else if (key === 'foreshadows') state.cache.foreshadows = await api.listForeshadows(pid)
  render()
}

/** 把手机上的全部数据备份成一个文件（防丢失） */
async function backupPhoneData() {
  if (!isTauri) {
    toast('备份功能需要在手机 App 内使用')
    return
  }
  if (!confirm('将把手机上的全部数据备份成一个文件。\n\n备份不会修改或删除现有数据，可以放心执行。')) {
    return
  }

  toast('正在备份…')
  try {
    const path = await backupDatabase()
    // 路径较长，用确认框展示以便用户长按复制；同时给出取文件的办法
    alert(
      `备份完成 ✅\n\n文件位置：\n${path}\n\n` +
      `取走文件的办法：\n` +
      `1) 用手机自带的「文件管理」进入 Android/data/com.wangxingchao.novel/files/export\n` +
      `2) 或者用数据线连电脑，在同样的路径下拷出`,
    )
    toast('已备份')
  } catch (e) {
    console.error('[app] 备份失败', e)
    toast('备份失败：' + e.message, 4000)
  }
}

async function loadMisc(key) {
  const pid = state.project.id
  const c = state.cache
  if (key === 'snapshots') c.snapshots = await api.listSnapshots(pid)
  else if (key === 'proposals') c.proposals = await api.listProposals(pid)
  else if (key === 'events') c.events = await api.listEvents(pid)
  render()
}

// ------------------------------------------------- 提案审批 / 快照回滚

/** 批准一条 AI 提案：改动会真正写入正典，必须二次确认 */
async function acceptProposalById(id) {
  const p = (state.cache.proposals || []).find((x) => x.id === id)
  const desc = p?.summary || p?.reason || p?.changes?.[0]?.description || id.slice(0, 8)
  if (!confirm(`批准这条提案？\n\n${desc}\n\n改动会立即写入正典（Canon），落库后无法一键撤销。`)) return
  try {
    const r = await api.acceptProposal(id)
    toast(`已批准（${proposalStatusLabel(r?.status)}）`)
    await loadMisc('proposals')
  } catch (e) {
    console.error('[app] 批准提案失败', e)
    toast('批准失败：' + e.message)
  }
}

/** 拒绝一条 AI 提案 */
async function rejectProposalById(id) {
  const p = (state.cache.proposals || []).find((x) => x.id === id)
  const desc = p?.summary || p?.reason || p?.changes?.[0]?.description || id.slice(0, 8)
  if (!confirm(`拒绝这条提案？\n\n${desc}`)) return
  try {
    const r = await api.rejectProposal(id)
    toast(`已拒绝（${proposalStatusLabel(r?.status)}）`)
    await loadMisc('proposals')
  } catch (e) {
    console.error('[app] 拒绝提案失败', e)
    toast('拒绝失败：' + e.message)
  }
}

/** 新建快照：名字必填（便于以后认出来是哪一步） */
async function createSnapshotPrompt() {
  const name = prompt('给这个快照起个名字（例如「第二卷完稿」）')
  if (name === null) return
  if (!name.trim()) {
    toast('名字不能为空')
    return
  }
  try {
    await api.createSnapshot(state.project.id, { name: name.trim() })
    toast('快照已创建')
    await loadMisc('snapshots')
  } catch (e) {
    console.error('[app] 创建快照失败', e)
    toast('创建失败：' + e.message)
  }
}

/** 把项目状态回滚到某个快照 */
async function restoreSnapshotById(id) {
  const s = (state.cache.snapshots || []).find((x) => x.id === id)
  const label = s?.name || s?.title || id.slice(0, 8)
  if (!confirm(`恢复到快照「${label}」？\n\n当前的宏观状态会被这个快照覆盖。`)) return
  try {
    await api.restoreSnapshot(id)
    toast('已恢复到该快照')
    // 状态变了，缓存里的世界/故事数据都可能过期，清掉重取
    clearCache()
    await loadMisc('snapshots')
  } catch (e) {
    console.error('[app] 恢复快照失败', e)
    toast('恢复失败：' + e.message)
  }
}

/** 删除一个快照 */
async function deleteSnapshotById(id) {
  const s = (state.cache.snapshots || []).find((x) => x.id === id)
  const label = s?.name || s?.title || id.slice(0, 8)
  if (!confirm(`删除快照「${label}」？删除后无法恢复。`)) return
  try {
    await api.deleteSnapshot(id)
    toast('快照已删除')
    await loadMisc('snapshots')
  } catch (e) {
    console.error('[app] 删除快照失败', e)
    toast('删除失败：' + e.message)
  }
}

// ---------------------------------------------------------------- 详情页

/** 从各种形状的返回值里取出一个可显示的名字 */
function extractName(v) {
  if (v == null) return ''
  if (typeof v === 'string') return v
  if (typeof v === 'object') return v.name || v.title || v.entity_name || ''
  return String(v)
}

/** 打开某条数据的全屏详情页（手机窄屏下比右侧抽屉更好用） */
async function openDetail(kind, id) {
  if (!id) {
    toast('缺少内容标识')
    return
  }
  state.detail = { loading: true, kind, id, title: '加载中' }
  render()
  pushDetailHistory()

  try {
    let data = null
    let profile = null

    if (kind === 'characters') {
      data = await api.get(`/characters/${id}`)
      try { profile = await api.get(`/characters/${id}/profile`) } catch (e) {
        console.warn('[app] 角色档案读取失败（可能尚未建立）', e)
      }
    } else if (kind === 'locations') {
      data = await api.get(`/locations/${id}`)
      try { profile = await api.get(`/locations/${id}/profile`) } catch (e) {
        console.warn('[app] 地点档案读取失败', e)
      }
    } else if (kind === 'factions') {
      data = await api.get(`/factions/${id}`)
      try { profile = await api.get(`/factions/${id}/profile`) } catch (e) {
        console.warn('[app] 势力档案读取失败', e)
      }
    } else if (kind === 'items' || kind === 'rules' || kind === 'relations') {
      // 这几类没有单独的详情接口，直接用列表缓存里的数据
      data = (state.cache[kind] || []).find((x) => x.id === id) || null
      if (!data) throw new Error('本地缓存里找不到这条数据，请返回列表重新加载')
    } else if (kind === 'nodes') {
      data = (state.cache.nodes || []).find((x) => x.id === id) || await api.get(`/narrative/${id}`)
    } else if (kind === 'storylines') {
      data = (state.cache.storylines || []).find((x) => x.id === id)
    } else if (kind === 'foreshadows') {
      data = (state.cache.foreshadows || []).find((x) => x.id === id)
    }

    state.detail = {
      loading: false,
      kind,
      id,
      data,
      profile,
      title: detailTitle(kind, data),
    }
    render()

    // 附加数据（关系 / 相关人物 / 事件）单独异步加载，不挡住主内容先显示
    await loadDetailExtras(kind, id)
    render()
  } catch (e) {
    console.error('[app] 详情读取失败', e)
    state.detail = { loading: false, kind, id, error: e.message }
    render()
  }
}

/** 按类型加载详情页的附加数据 */
async function loadDetailExtras(kind, id) {
  const d = state.detail
  if (!d || d.error) return

  if (kind === 'characters') {
    // 当前状态与已知信息：拿不到就当作没有，不影响主内容展示
    try {
      const st = await api.characterState(id)
      d.status = st && typeof st === 'object' && !Array.isArray(st) ? st : null
    } catch (e) {
      console.warn('[app] 角色状态读取失败', e)
      d.status = null
    }
    try {
      const kn = await api.characterKnowledge(id)
      d.knowledge = Array.isArray(kn) ? kn : []
    } catch (e) {
      console.warn('[app] 角色已知信息读取失败', e)
      d.knowledge = []
    }

    let rels = []
    try {
      rels = await api.characterRelationships(id)
    } catch (e) {
      console.warn('[app] 人物关系读取失败', e)
      rels = []
    }
    const list = Array.isArray(rels) ? rels : []
    // 服务端已经双向查过，并直接给出「对方」的 id / 名字 / 方向；
    // 旧字段（target / source_name 之类）保留兼容，避免版本混用时显示成空。
    d.relations = list.map((r) => {
      const otherName =
        r.other_name ||
        (r.source_entity_id === id ? r.target_name : r.source_name) ||
        r.target_name || r.source_name || r.target || r.name || ''
      const otherId =
        r.other_id || r.target_id ||
        (r.source_entity_id === id ? r.target_entity_id : r.source_entity_id) || ''
      return {
        otherId,
        otherName: extractName(otherName) || '未知对象',
        // 关系描述里通常已经说明「谁对谁是什么关系」，正文一并展示
        description: r.description || '',
        type: r.relation_type || r.type || r.relation || '关联',
        // true = 当前条目指向对方；图上用实线/虚线区分
        outgoing: r.outgoing !== false,
      }
    })
    d.relationsLoaded = true
    return
  }

  if (kind === 'locations') {
    try {
      const ents = await api.locationEntities(id)
      d.entities = Array.isArray(ents) ? ents : []
    } catch (e) {
      console.warn('[app] 地点相关人物读取失败', e)
      d.entities = []
    }
    try {
      const evs = await api.locationEvents(id)
      d.events = Array.isArray(evs) ? evs : []
    } catch (e) {
      console.warn('[app] 地点事件读取失败', e)
      d.events = []
    }
  }
}

/** 详情页入栈一条历史，使 Android 返回键/手势先关详情而不是退出 App */
function pushDetailHistory() {
  const h = '#/detail'
  if (location.hash !== h) {
    history.pushState({ detail: true }, '', h)
  }
}

/** 同步返回键行为：详情打开时监听 popstate */
function syncBackGuard() {
  window.onpopstate = () => {
    if (state.onboarding) {
    els.composer.hidden = true
    els.view.innerHTML = renderOnboarding()
    bindOnboarding()
    return
  }

  // 设置页不是底部 Tab 之一，用独立标志判定；
  // 若把它当作 tab 值塞进 renderers 映射，点底部 Tab 时会取到 undefined 而崩。
  if (state.showingSettings) {
    renderSettings()
    return
  }

  if (state.detail) {
      state.detail = null
      render()
      return
    }
    if (state.showingSettings) {
      state.showingSettings = false
      state.tab = 'agent'
      render()
    }
  }
}

async function askCreateProject() {
  const name = prompt('新项目名称')
  if (!name || !name.trim()) return
  await createProject(name.trim())
}

async function askSwitchProject() {
  if (state.projects.length === 0) { toast('还没有项目'); return }
  const lines = state.projects.map((p, i) => `${i + 1}. ${p.name}`).join('\n')
  const ans = prompt(`选择项目（输入序号）:\n${lines}`)
  const idx = Number(ans) - 1
  if (idx >= 0 && idx < state.projects.length) {
    await selectProject(state.projects[idx].id)
  }
}

// ---------------------------------------------------------------- AI 文本抽取

/**
 * 把一个实体类型的英文名翻成中文（抽取结果展示用）
 */
function extractTypeLabel(t) {
  const raw = String(t || '').trim()
  const map = {
    Character: '人物',
    Location: '地点',
    Organization: '势力',
    Faction: '势力',
    Event: '事件',
    Item: '物品',
  }
  return map[raw] || raw
}

/**
 * AI 文本抽取页。
 *
 * 后端接口 `POST /projects/{id}/extract` 是**同步等 LLM** 的（与电脑端 Extract 页一致），
 * 一次可能等十几到几十秒。所以这里：
 *   - 抽取期间拿屏幕常亮 + 开前台服务（两者都是防「切后台被系统冻结」）
 *   - 因此可以明确告诉用户：切走也没关系，后台会跑完
 *   - 完成后提示抽取产出的提案在哪里审批——抽取只生成**草稿**，绝不直接改正典
 */
function renderExtract() {
  const s = state.extract
  const busy = s.running

  let html = `
    <div class="list"><div class="list-item" data-act="close-extract">
      <div class="li-main"><div class="li-title">‹ 返回</div></div></div></div>
    <div class="detail-title">AI 文本抽取</div>
    <div class="hint" style="margin-bottom:12px">
      粘贴一段正文，AI 会从中抽出现人物 / 地点 / 势力 / 事件以及它们之间的关系。
      抽取结果只是**待审批的草稿**，不会直接改动你的设定——批准在「更多 → AI 提案」里做。
    </div>
    <div class="field">
      <label>原文</label>
      <textarea id="ex-text" class="edit-area" rows="12"
        placeholder="把正文粘到这里…（一段到一章都可以，太长会超出模型输出上限）"
        ${busy ? 'disabled' : ''}>${esc(s.text)}</textarea>
      <div class="hint">建议一次 3000 字以内；抽取只读这段文字，不会动已有设定。</div>
    </div>
  `

  if (busy) {
    html += `<div class="extract-running">
      <span class="spinner"></span>
      <div>正在抽取，可能要十几到几十秒。<br>切到别的应用也没关系——通知栏会显示「正在生成」，抽取会在后台跑完。</div>
    </div>`
  } else {
    html += `<button class="btn-primary" data-act="run-extract">开始抽取</button>
      <button class="btn-secondary" data-act="clear-extract">清空原文</button>`
  }

  if (s.error) {
    html += `<div class="hint status-fail" style="margin-top:12px">抽取失败：${esc(s.error)}</div>`
  }

  if (s.result) {
    const r = s.result
    const ents = Array.isArray(r.entities) ? r.entities : []
    const rels = Array.isArray(r.relations) ? r.relations : []

    html += `<div class="section-title">抽出的人物 / 地点 / 势力 / 事件（${ents.length}）</div>`
    if (ents.length === 0) {
      html += '<div class="hint">这段文字里没有抽出实体。</div>'
    } else {
      html += '<div class="list">'
      for (const e of ents) {
        html += `<div class="list-item"><div class="li-main">
          <div class="li-title">${esc(e.name || '未命名')}</div>
          <div class="li-sub">${esc(extractTypeLabel(e.entity_type))}${e.summary ? ' · ' + esc(String(e.summary).slice(0, 60)) : ''}</div>
        </div></div>`
      }
      html += '</div>'
    }

    html += `<div class="section-title">抽出的关系（${rels.length}）</div>`
    if (rels.length === 0) {
      html += '<div class="hint">没有抽出关系。</div>'
    } else {
      html += '<div class="list">'
      for (const r2 of rels) {
        html += `<div class="list-item"><div class="li-main">
          <div class="li-title">${esc(r2.from || '?')} → ${esc(r2.to || '?')}</div>
          <div class="li-sub">${esc(String(r2.relation_type || ''))}${r2.description ? ' · ' + esc(String(r2.description).slice(0, 50)) : ''}</div>
        </div></div>`
      }
      html += '</div>'
    }

    // 抽出的实体与关系都已经落成草案，去提案页批准才生效
    html += `<div class="hint" style="margin-top:12px">
      已生成 ${ents.length + rels.length} 条待审批草稿。到「更多 → AI 提案」逐条批准（或拒绝）后才会写入正典；
      <strong>先批准实体、再批准关系</strong>，否则关系会找不到端点。
    </div>
    <button class="btn-secondary" data-act="goto-proposals">去审批这些提案</button>`
  }

  html += '<button class="btn-secondary" data-act="close-extract">返回「更多」</button>'
  return html
}

/** 抽取页的渲染与绑定（与设置页同模式：自己管自己的 DOM） */
function renderExtractView() {
  state.showingExtract = true
  if (location.hash !== '#/extract') history.pushState({ extract: true }, '', '#/extract')
  document.querySelectorAll('.tab').forEach((t) => t.classList.remove('active'))
  els.composer.hidden = true
  els.view.innerHTML = renderExtract()
  els.view.querySelectorAll('[data-act]').forEach((el) => {
    el.addEventListener('click', async () => {
      const act = el.dataset.act
      if (act === 'close-extract') {
        state.showingExtract = false
        state.tab = 'more'
        render()
      } else if (act === 'clear-extract') {
        state.extract.text = ''
        renderExtractView()
      } else if (act === 'run-extract') {
        await runExtract()
      } else if (act === 'goto-proposals') {
        state.showingExtract = false
        state.tab = 'more'
        render()
        // 列表默认收起，这里直接把提案拉出来
        await loadMisc('proposals')
      }
    })
  })
  // 文本框输入要留在 state 里：抽取中/重渲染时不能丢用户粘的内容
  const ta = $('ex-text')
  if (ta) ta.addEventListener('input', () => { state.extract.text = ta.value })
}

/**
 * 真正执行抽取。
 *
 * 这是本功能唯一的长耗时操作：接口同步等 LLM，中途切后台会被系统挂起导致请求中断，
 * 因此这里显式申请屏幕常亮，并在界面上写明。失败原因（模型没返回可解析 JSON、
 * 文本太长撞输出上限等）由后端给出，这里原样展示，不做兜底。
 */
async function runExtract() {
  const text = (state.extract.text || '').trim()
  if (!text) {
    toast('请先粘贴要抽取的正文')
    return
  }

  state.extract.running = true
  state.extract.error = null
  state.extract.result = null
  renderExtractView()
  // 同上：不 await，避免常亮申请挂住抽取主流程
  acquireWakeLock()
  // 抽取最长实测 67 秒，同样会被系统冻结（切后台就断）。
  // 开前台服务让进程保持存活；同样不 await——失败不该挡住抽取。
  setKeepAlive(true).catch((e) => console.warn('[app] 前台服务启动失败', e))

  try {
    const result = await api.extractText(state.project.id, text)
    state.extract.result = result || { entities: [], relations: [] }
    toast('抽取完成，去「AI 提案」审批草稿')
  } catch (e) {
    console.error('[app] 文本抽取失败', e)
    // 服务端的抽取跑在独立任务里：网络断掉（切后台被挂起）只影响这次请求，
    // 任务本身会继续跑完并把草稿写进库。所以失败时要说清去哪找结果，
    // 否则用户会以为白等一场。
    state.extract.error = `${e.message}（若是网络中断导致的失败，抽取任务可能仍在后台继续——稍后到「更多 → AI 提案」看看有没有新草稿）`
  } finally {
    state.extract.running = false
    releaseWakeLock()
    setKeepAlive(false).catch((e) => console.warn('[app] 前台服务停止失败', e))
    // 抽完可能已经切到别的页，这里只在还停在抽取页时重绘
    if (state.showingExtract) renderExtractView()
  }
}

// ---------------------------------------------------------------- 设置页

async function renderSettings() {
  state.showingSettings = true
  if (location.hash !== '#/settings') history.pushState({ settings: true }, '', '#/settings')
  document.querySelectorAll('.tab').forEach((t) => t.classList.remove('active'))
  els.composer.hidden = true
  els.view.innerHTML = '<div style="text-align:center;padding:30px"><span class="spinner"></span></div>'

  let s = {}
  try {
    s = await api.getSettings()
  } catch (e) {
    toast('读取设置失败: ' + e.message)
  }

  // 设置页自己维护草稿：它不参与全局 render()，交互时不重绘，输入不会丢。
  // 字段名与电脑端设置页一致（camelCase），否则引擎读不到（见 db/ai_settings.rs）。
  let all = s && typeof s === 'object' ? s : {}

  // 库里没有的值用「当前生效值」补齐：密钥通常来自构建注入而不写库，
  // 不补的话设置页会显示成空，用户会以为密钥没带过来。
  let effective = {}
  try {
    const ready = await api.engineReady()
    effective = ready?.config || {}
  } catch (e) {
    toast('读取引擎当前配置失败: ' + e.message)
  }
  const keyFromBoot = !all.aiApiKey && !!effective.aiApiKey

  const form = {
    aiBaseUrl: all.aiBaseUrl || effective.aiBaseUrl || '',
    aiApiKey: all.aiApiKey || effective.aiApiKey || '',
    defaultModel: all.defaultModel || effective.defaultModel || '',
    contextLimits: { ...(all.contextLimits || {}) },
    maxOutputTokens: all.maxOutputTokens || effective.maxOutputTokens || 22000,
  }

  // 内置模型目录（模型 → 真实上下文上限）；网关不返回该数据
  let catalog = { default_limit: 128000, context_limits: {} }
  try {
    catalog = await api.getModelCatalog()
  } catch (e) {
    toast('读取模型目录失败: ' + e.message)
  }

  /** 网关返回的可用模型（模型名下拉的候选） */
  let modelOptions = []
  let showKey = false

  const limitForModel = (m) => catalog.context_limits?.[m] ?? catalog.default_limit
  const limitOverridden = (m) => Object.prototype.hasOwnProperty.call(form.contextLimits, m)

  els.view.innerHTML = `
    <div class="section-title">AI 网关</div>
    <div class="field">
      <label>接口地址</label>
      <input id="s-base" type="url" placeholder="https://opencode.ai/zen/go/v1"
             value="${esc(form.aiBaseUrl)}" />
      <div class="hint">OpenAI 兼容端点前缀，不含 /chat/completions</div>
    </div>
    <div class="field">
      <label>API Key</label>
      <div class="input-row">
        <input id="s-key" type="password" placeholder="sk-…" value="${esc(form.aiApiKey)}" />
        <button class="inline-btn" id="s-key-toggle" type="button">显示</button>
      </div>
      <div class="hint">${keyFromBoot
        ? '当前显示的是 App 内置密钥（构建时从电脑端的 .env 注入），点「保存」即写入本机数据库。'
        : '仅保存在本机数据库，直连 AI 服务商。'}</div>
    </div>
    <div class="field">
      <label>模型名</label>
      <div class="input-row">
        <select id="s-model"></select>
        <button class="inline-btn" id="s-refresh-models" type="button">刷新列表</button>
      </div>
      <div class="hint" id="s-models-hint">正在获取可用模型…</div>
    </div>
    <div class="field">
      <label>上下文上限</label>
      <div class="input-row">
        <input id="s-ctx" type="number" min="1" step="1000" inputmode="numeric" />
        <span class="unit">tokens</span>
      </div>
      <div class="hint" id="s-ctx-hint"></div>
    </div>
    <div class="field">
      <label>单次输出上限</label>
      <div class="input-row">
        <input id="s-maxout" type="number" min="256" step="512" inputmode="numeric"
               value="${form.maxOutputTokens}" />
        <span class="unit">tokens</span>
      </div>
      <div class="hint">默认 22000。中文长文本 + 工具调用 JSON 容易撞到输出上限。</div>
    </div>
    <button class="btn-secondary" id="s-test">测试连接</button>
    <div class="hint" id="s-test-result"></div>
    <button class="btn-primary" id="s-save">保存</button>
    <div class="hint" id="s-msg"></div>

    <div class="section-title">从电脑导入（同一 WiFi）</div>
    <div class="field">
      <label>电脑地址</label>
      <input id="s-host" type="text" placeholder="192.168.1.11:8080"
             value="${esc(prefs.lastHost || '192.168.1.11:8080')}" />
      <div class="hint">电脑端需正在运行（同一个 WiFi 下）。下面两个按钮都走这个地址。</div>
    </div>
    <button class="btn-secondary" id="s-sync-ai">同步电脑的 AI 配置</button>
    <div id="s-sync-result" class="hint" style="margin-top:10px"></div>
    <button class="btn-secondary" id="s-fetch-projects">读取电脑上的项目</button>
    <div id="s-remote-list" class="list" style="margin-top:10px"></div>
    <div id="s-import-result" class="hint" style="margin-top:10px"></div>

    <div class="section-title">备份到电脑</div>
    <div class="field">
      <div class="hint">把当前项目的数据传到电脑上保存一份，防止手机数据丢失。</div>
    </div>
    <button class="btn-secondary" id="s-upload">把当前项目传到电脑</button>
    <div id="s-upload-result" class="hint" style="margin-top:10px"></div>

    <button class="btn-secondary" id="s-back">返回对话</button>
  `

  // ---------- 模型下拉 ----------

  /**
   * 用「拉到的模型列表 + 当前已保存的模型」重填下拉。
   * 把当前值并入是为了让已保存的模型在刷新前也能正确显示与选中。
   */
  function fillModels() {
    const sel = $('s-model')
    const list = [...modelOptions]
    if (form.defaultModel && !list.includes(form.defaultModel)) list.unshift(form.defaultModel)
    if (list.length === 0) {
      sel.innerHTML = '<option value="">请先点「刷新列表」获取模型</option>'
      sel.disabled = true
      return
    }
    sel.disabled = false
    sel.innerHTML =
      '<option value="">请选择模型</option>' +
      list.map((m) => `<option value="${esc(m)}"${m === form.defaultModel ? ' selected' : ''}>${esc(m)}</option>`).join('')
  }

  /** 把「当前模型」的上限带进输入框：优先用户覆盖，其次内置目录。 */
  function syncCtx() {
    const m = form.defaultModel
    const value = m ? (limitOverridden(m) ? form.contextLimits[m] : limitForModel(m)) : ''
    $('s-ctx').value = value === '' ? '' : value
    $('s-ctx-hint').textContent = m
      ? (limitOverridden(m)
          ? '当前为你的自定义覆盖值。'
          : `已按「${m}」自动带入内置目录的真实上限；如需调整可直接修改，保存后会记为该模型的覆盖值。`)
      : '请先选择模型。'
  }

  fillModels()
  syncCtx()

  /** 拉取网关可用模型列表（设置页模型下拉的数据源） */
  async function loadModels() {
    const hint = $('s-models-hint')
    hint.textContent = '正在获取模型列表…'
    try {
      const r = await api.listModels({
        base_url: $('s-base').value.trim(),
        api_key: $('s-key').value.trim(),
      })
      if (r.ok) {
        modelOptions = r.models || []
        fillModels()
        hint.textContent = modelOptions.length
          ? `已获取 ${modelOptions.length} 个可用模型，只可从下拉中选择。`
          : '网关没有返回任何模型。'
      } else {
        modelOptions = []
        fillModels()
        hint.textContent = `模型列表获取失败：${r.error || '未知错误'}`
      }
    } catch (e) {
      modelOptions = []
      fillModels()
      hint.textContent = `模型列表获取失败：${e.message}`
    }
  }

  $('s-refresh-models').addEventListener('click', () => loadModels())

  // 进页面即拉一次可用模型（与电脑端设置页一致）
  loadModels()

  $('s-key-toggle').addEventListener('click', () => {
    showKey = !showKey
    $('s-key').type = showKey ? 'text' : 'password'
    $('s-key-toggle').textContent = showKey ? '隐藏' : '显示'
  })

  $('s-model').addEventListener('change', () => {
    form.defaultModel = $('s-model').value
    syncCtx()
  })

  $('s-ctx').addEventListener('input', () => {
    const m = form.defaultModel
    if (!m) return
    const v = parseInt($('s-ctx').value, 10)
    if (!Number.isFinite(v) || v <= 0) return
    // 只更新草稿与提示：这里不回调 syncCtx()，否则会重设输入框的值打断输入
    form.contextLimits[m] = v
    $('s-ctx-hint').textContent =
      v === limitForModel(m)
        ? `与内置目录一致（${v} tokens），保存后不会额外记录覆盖值。`
        : `已记为「${m}」的自定义覆盖值（内置目录为 ${limitForModel(m)} tokens）。`
  })

  $('s-save').addEventListener('click', async () => {
    const msg = $('s-msg')
    const baseUrl = $('s-base').value.trim()
    const apiKey = $('s-key').value.trim()
    const model = $('s-model').value
    const maxOut = parseInt($('s-maxout').value, 10)

    // 明确校验，不静默放过
    if (!baseUrl) { msg.textContent = '请填写接口地址'; return }
    if (!apiKey) { msg.textContent = '请填写 API Key'; return }
    if (!model) { msg.textContent = '请先从列表中选择模型'; return }
    if (!Number.isFinite(maxOut) || maxOut <= 0) { msg.textContent = '单次输出上限需为正整数'; return }

    // 只有与内置目录不一致时才写入按模型的覆盖，避免堆一批冗余条目；
    // 改回目录值时顺手清掉旧覆盖。
    const limits = { ...form.contextLimits }
    const ctx = parseInt($('s-ctx').value, 10)
    if (Number.isFinite(ctx) && ctx > 0 && ctx !== limitForModel(model)) {
      limits[model] = ctx
    } else {
      delete limits[model]
    }

    msg.textContent = '正在保存…'
    try {
      // 整体回写：/settings 是全量覆盖，必须保留项目名称、语言等原有字段
      const payload = {
        ...all,
        aiBaseUrl: baseUrl,
        aiApiKey: apiKey,
        defaultModel: model,
        contextLimits: limits,
        maxOutputTokens: maxOut,
      }
      const saved = await api.saveSettings(payload)
      all = saved && typeof saved === 'object' ? saved : payload
      form.aiBaseUrl = all.aiBaseUrl
      form.aiApiKey = all.aiApiKey
      form.defaultModel = all.defaultModel
      form.contextLimits = { ...(all.contextLimits || {}) }
      form.maxOutputTokens = all.maxOutputTokens
      msg.textContent = `已保存 · ${new Date().toLocaleTimeString()}`
      toast('设置已保存，立即生效')
    } catch (e) {
      msg.textContent = `保存失败：${e.message}`
    }
  })

  $('s-test').addEventListener('click', async () => {
    const out = $('s-test-result')
    out.className = 'hint'
    out.textContent = '正在发起一次真实请求…'
    try {
      const r = await api.testConnection({
        base_url: $('s-base').value.trim(),
        api_key: $('s-key').value.trim(),
        model: $('s-model').value,
      })
      if (r.ok) {
        out.className = 'hint status-ok'
        out.textContent = `连接成功 · ${r.model} · ${r.latency_ms}ms${r.reply ? ` · 回复「${r.reply.slice(0, 20)}」` : ''}`
        // 连通即拉取可用模型列表，省掉用户再去点一次「刷新列表」
        await loadModels()
      } else {
        out.className = 'hint status-fail'
        out.textContent = `连接失败：${r.error || '未知错误'}`
      }
    } catch (e) {
      out.className = 'hint status-fail'
      out.textContent = `连接失败：${e.message}`
    }
  })

  $('s-upload').addEventListener('click', () => uploadCurrentProject())

  $('s-sync-ai').addEventListener('click', async () => {
    const host = $('s-host').value.trim()
    setPref('lastHost', host)
    const out = $('s-sync-result')
    out.className = 'hint'
    out.textContent = '正在读取电脑端设置…'
    try {
      const remote = await fetchRemoteSettings(host)

      const applied = []
      if (remote.aiBaseUrl) {
        form.aiBaseUrl = remote.aiBaseUrl
        $('s-base').value = remote.aiBaseUrl
        applied.push('接口地址')
      }
      if (remote.aiApiKey) {
        form.aiApiKey = remote.aiApiKey
        $('s-key').value = remote.aiApiKey
        applied.push('API Key')
      }
      if (remote.defaultModel) {
        form.defaultModel = remote.defaultModel
        fillModels()
        syncCtx()
        applied.push(`模型（${remote.defaultModel}）`)
      }
      if (remote.maxOutputTokens) {
        form.maxOutputTokens = remote.maxOutputTokens
        $('s-maxout').value = remote.maxOutputTokens
        applied.push('单次输出上限')
      }
      if (remote.contextLimits && typeof remote.contextLimits === 'object') {
        form.contextLimits = { ...form.contextLimits, ...remote.contextLimits }
        syncCtx()
        applied.push('上下文覆盖')
      }

      if (applied.length === 0) {
        out.className = 'hint status-fail'
        out.textContent = '电脑端没有可同步的 AI 配置'
        return
      }
      // 电脑端的 Key 常来自环境变量、不写库；拿不到时提示别的原因，不静默
      const missingKey = !remote.aiApiKey ? '；API Key 未存在电脑端数据库（走环境变量），此处保持原值' : ''
      out.className = 'hint status-ok'
      out.textContent = `已带过来：${applied.join('、')}${missingKey}。确认后点上面的「保存」才会生效。`
    } catch (e) {
      out.className = 'hint status-fail'
      out.textContent = `同步失败：${e.message}`
    }
  })

  $('s-fetch-projects').addEventListener('click', async () => {
    const host = $('s-host').value.trim()
    setPref('lastHost', host)
    const box = $('s-remote-list')
    const out = $('s-import-result')
    out.textContent = '正在连接电脑…'
    box.innerHTML = ''
    try {
      const list = await fetchRemoteProjects(host)
      if (list.length === 0) {
        out.textContent = '电脑上还没有项目'
        return
      }
      // 只列最近更新的 30 个，避免一次拉太多
      const recent = list.slice(0, 30)
      box.innerHTML = recent.map((p) => `
        <div class="list-item" data-remote="${esc(p.id)}">
          <div class="li-main">
            <div class="li-title">${esc(p.name || '未命名')}</div>
            <div class="li-sub">${esc(p.status || '')} · ${esc(fmtTime(p.updated_at))}</div>
          </div>
          <span class="li-arrow">导入</span>
        </div>`).join('')
      out.textContent = list.length > recent.length
        ? `共 ${list.length} 个项目，显示最近 ${recent.length} 个`
        : `共 ${list.length} 个项目`
      box.querySelectorAll('[data-remote]').forEach((el) => {
        el.addEventListener('click', () => doRemoteImport(host, el.dataset.remote, el))
      })
    } catch (e) {
      out.textContent = `连接电脑失败：${e.message}`
    }
  })

  $('s-back').addEventListener('click', () => {
    state.showingSettings = false
    state.detail = null
    state.tab = 'agent'
    render()
  })
}

/** 删除一个历史会话（需二次确认，避免误删） */
async function deleteSessionById(sessionId) {
  const s = state.sessions.find((x) => x.id === sessionId)
  const label = s?.title || '未命名会话'
  if (!confirm(`删除会话「${label}」？该会话的对话记录会一并删除，无法恢复。`)) return

  try {
    await api.deleteSession(sessionId)
    toast('会话已删除')
    state.sessions = await api.listSessions(state.project.id)
    if (state.session?.id === sessionId) {
      // 删的是当前会话：切到别的会话，没有就新建一个
      if (state.sessions.length > 0) {
        state.session = state.sessions[0]
        setPref('lastSessionId', state.session.id)
        await loadMessages(state.session.id)
      } else {
        await newSession()
        return
      }
    }
    closeDrawer()
    render()
  } catch (e) {
    console.error('[app] 删除会话失败', e)
    toast('删除失败：' + e.message)
  }
}

/** 复制一条消息的原文到剪贴板 */
async function copyMessage(msgId) {
  const m = state.messages.find((x) => x.id === msgId)
  if (!m) return
  const text = m.content || ''
  if (!text) return
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text)
    } else {
      // 旧 WebView 兜底：用隐藏 textarea + execCommand
      const ta = document.createElement('textarea')
      ta.value = text
      ta.style.position = 'fixed'
      ta.style.opacity = '0'
      document.body.appendChild(ta)
      ta.select()
      const ok = document.execCommand('copy')
      ta.remove()
      if (!ok) throw new Error('execCommand 复制失败')
    }
    toast('已复制')
  } catch (e) {
    console.error('[app] 复制失败', e)
    toast('复制失败：' + e.message)
  }
}

/** 把当前项目导出并上传到电脑保存（防手机数据丢失） */
async function uploadCurrentProject() {
  const out = $('s-upload-result')
  if (!state.project) {
    out.textContent = '请先回到对话页选择一个项目'
    return
  }
  const host = $('s-host').value.trim()
  if (!host) {
    out.textContent = '请先填写上面的电脑地址'
    return
  }
  setPref('lastHost', host)

  out.textContent = '正在导出并上传…'
  try {
    const base = normalizeHost(host)

    // 1) 从本地引擎取该项目的完整数据（与电脑端导出同一格式）
    const local = await fetch(
      `http://127.0.0.1:8080/api/v1/projects/${state.project.id}/export`,
    )
    if (!local.ok) throw new Error(`本地导出失败 HTTP ${local.status}`)
    const text = await local.text()

    // 2) 上传到电脑
    const resp = await fetch(`${base}/api/v1/backup/receive`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'x-novel-device': 'novel-mobile',
      },
      body: text,
    })
    const bodyText = await resp.text()
    if (!resp.ok) {
      let detail = bodyText
      try { detail = JSON.parse(bodyText).error || bodyText } catch {}
      throw new Error(`电脑端返回 HTTP ${resp.status}：${detail}`)
    }
    const r = JSON.parse(bodyText)
    out.innerHTML = `✅ 已备份到电脑：${esc(r.project_name)}<br>`
      + `${r.tables} 张表 / ${(r.bytes / 1024 / 1024).toFixed(2)} MB<br>`
      + `电脑上的位置：<br>${esc(r.saved_to)}`
    toast('已备份到电脑')
  } catch (e) {
    console.error('[app] 备份到电脑失败', e)
    out.textContent = `备份失败：${e.message}`
  }
}

/** 从电脑端导入一个项目（局域网） */
async function doRemoteImport(host, projectId, el) {
  const out = $('s-import-result')
  const title = el?.querySelector('.li-title')?.textContent || projectId
  out.textContent = `正在导入「${title}」，大项目需要一会儿…`
  try {
    const r = await importFromComputer(host, projectId)
    out.innerHTML = `✅ 导入完成：${esc(r.project_name)}<br>`
      + `${r.tables_written} 张表 / ${r.rows_written} 行`
      + (r.skipped_rows ? `<br>跳过 ${r.skipped_rows} 行（源数据引用无效）` : '')
    toast('导入完成')

    state.projects = await api.listProjects()
    await selectProject(r.project_id, { silent: true })
    state.tab = 'agent'
    render()
  } catch (e) {
    console.error('[app] 局域网导入失败', e)
    out.textContent = `导入失败：${e.message}`
  }
}

// ---------------------------------------------------------------- 抽屉

function openDrawer() {
  els.drawer.hidden = false
  els.drawerMask.hidden = false
  els.drawerBody.innerHTML = state.sessions.length
    ? state.sessions.map((s) => {
        const t = s.title || '未命名会话'
        const step = s.current_step || ''
        const n = (s.messages || []).length
        return `<div class="drawer-item ${s.id === state.session?.id ? 'active' : ''}" data-sid="${esc(s.id)}">
          <div class="di-main">
            <div class="di-title">${esc(t)}</div>
            <div class="di-sub">${esc(step)} · ${n} 条消息</div>
          </div>
          <button class="di-del" data-del="${esc(s.id)}" aria-label="删除会话">删除</button>
        </div>`
      }).join('')
    : '<div class="drawer-item">暂无会话</div>'

  els.drawerBody.querySelectorAll('[data-sid]').forEach((el) => {
    el.addEventListener('click', (ev) => {
      // 删除按钮在会话项内部，先判断是不是点到了删除
      const delBtn = ev.target.closest('[data-del]')
      if (delBtn) {
        ev.stopPropagation()
        deleteSessionById(delBtn.dataset.del)
        return
      }
      switchSession(el.dataset.sid)
    })
  })
}

function closeDrawer() {
  els.drawer.hidden = true
  els.drawerMask.hidden = true
}

// ---------------------------------------------------------------- 输入框

function autoGrow() {
  els.input.style.height = 'auto'
  els.input.style.height = Math.min(132, els.input.scrollHeight) + 'px'
}

function updateSendBtn() {
  if (state.streaming) {
    els.send.textContent = '停止'
    els.send.classList.add('stop')
    els.send.disabled = false
  } else {
    els.send.textContent = '发送'
    els.send.classList.remove('stop')
    els.send.disabled = els.input.value.trim().length === 0
  }
}

// ---------------------------------------------------------------- 启动

function bindGlobal() {
  // 极简 hash 路由：#/agent、#/world、#/story、#/more、#/settings
  // 作用有二：底部 Tab 切换能被浏览器历史记录；调试时可直接跳到某个页面。
  const applyHash = () => {
    const h = (location.hash || '').replace(/^#\//, '')
    if (!h) return
    if (h === 'settings') {
      state.showingSettings = true
      state.tab = 'agent'
    } else if (['agent', 'world', 'story', 'more'].includes(h)) {
      state.showingSettings = false
      state.tab = h
    } else {
      return
    }
    render()
  }

  window.addEventListener('hashchange', applyHash)

  // 页面被隐藏/卸载时释放，避免无谓地占着屏幕
  window.addEventListener('pagehide', () => { releaseWakeLock() })

  // 从后台回来时补齐可能被掐断的回复
  document.addEventListener('visibilitychange', async () => {
    if (document.visibilityState !== 'visible') return
    if (state.streaming) {
      // 切后台时系统会强制释放 wake lock；回到前台若还在生成就重新申请
      acquireWakeLock()
      return
    }
    if (!state.session || state.onboarding) return

    const changed = await refreshSessionFromDb()
    if (changed) {
      render()
      toast('已同步完整回复')
      return
    }

    // 另一种情况：切后台时回复「还没生成完」，引擎随即被系统冻结，
    // 回到前台时数据库里仍然停在用户那条消息 —— 这时补齐拿不到东西，
    // 需要明确告诉用户可以重试（否则会让人以为消息发丢了）。
    if (state.messages.length === 0) return
    const last = state.messages[state.messages.length - 1]
    if (last.role !== 'user') return

    const snippet = (last.content || '').slice(0, 20)
    if (confirm(`上一条消息「${snippet}${(last.content || '').length > 20 ? '…' : ''}」还没有收到回复。\n\n可能是切到后台导致生成中断。要重新发送一次吗？`)) {
      await send(last.content)
    }
  })

  document.querySelectorAll('.tab').forEach((t) => {
    t.addEventListener('click', async () => {
      state.tab = t.dataset.tab
      state.detail = null
      state.showingSettings = false
      state.error = null   // 换页时清掉上一页的报错
      if (location.hash !== '#/' + t.dataset.tab) {
        history.pushState({ tab: t.dataset.tab }, '', '#/' + t.dataset.tab)
      }
      if (t.dataset.tab === 'agent') els.composer.hidden = false
      render()
      // 这两个 Tab 需要真实数据，切进来就自动拉，避免用户面对"点击加载"
      if (state.project) {
        try {
          if (t.dataset.tab === 'world') await loadWorldAll()
          else if (t.dataset.tab === 'story') await loadStoryAll()
        } catch (e) {
          state.error = e.message
          render()
        }
      }
    })
  })

  $('btn-menu').addEventListener('click', openDrawer)
  $('btn-drawer-close').addEventListener('click', closeDrawer)
  els.drawerMask.addEventListener('click', closeDrawer)
  $('btn-new').addEventListener('click', () => newSession().catch((e) => toast(e.message)))

  els.input.addEventListener('input', () => { autoGrow(); updateSendBtn() })
  els.input.addEventListener('keydown', (e) => {
    // 手机软键盘的回车用于换行，不直接发送
    if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault()
      send()
    }
  })

  els.send.addEventListener('click', () => {
    if (state.streaming) stopStreaming()
    else send()
  })
}

async function boot() {
  bindGlobal()
  syncBackGuard()
  // 启动时若 URL 指定了页面（调试或系统恢复），先切过去
  const initial = (location.hash || '').replace(/^#\//, '')
  if (initial === 'settings') {
    state.showingSettings = true
  } else if (['agent', 'world', 'story', 'more'].includes(initial)) {
    state.tab = initial
  }
  updateSendBtn()

  const ok = await checkEngine()
  if (!ok) {
    render()
    toast('引擎未连接，请稍后重试', 3000)
    return
  }

  // 未配置 AI 网关时先引导配置（否则一说话就报错，体验很差）
  try {
    const ready = await api.engineReady()
    // 上报「引擎当前生效的 AI 配置」。手机端没有控制台，而「设置页保存了但引擎
    // 仍在用旧配置」这类问题只能从这里看出来（曾真实发生过）。
    // 只报来源与长度，不把密钥本身写进日志。
    const cfg = ready.config || {}
    reportDiagnostic('ai-config', JSON.stringify({
      provider_configured: ready.provider_configured,
      model: ready.model,
      base_url: ready.base_url,
      context_limit: cfg.contextLimit,
      max_output_tokens: cfg.maxOutputTokens,
      key_length: (cfg.aiApiKey || '').length,
    }))
    if (!ready.provider_configured) {
      state.onboardingData = {
        base_url: ready.base_url || '',
        model: ready.model || '',
      }
      state.onboarding = true
      render()
      return
    }
  } catch (e) {
    console.warn('[app] 读取引擎状态失败，跳过首启引导', e)
  }

  try {
    await loadProjects()
  } catch (e) {
    state.error = `加载项目失败: ${e.message}`
  }
  render()
  updateSendBtn()

  // 上报一次运行环境能力，便于在没法开控制台的手机端排查问题
  reportDiagnostic('capabilities', JSON.stringify({
    wakeLock: 'wakeLock' in navigator,
    clipboard: !!(navigator.clipboard && navigator.clipboard.writeText),
    online: navigator.onLine,
    dpr: window.devicePixelRatio,
    viewport: `${window.innerWidth}x${window.innerHeight}`,
  }))

  // 若数据库因结构变更被重建过，明确告诉用户（不静默）
  try {
    const notice = await takeSchemaRebuildNotice()
    if (notice) toast(notice, 6000)
  } catch (e) {
    console.warn('[app] 重建通知读取失败', e)
  }
}

boot()
