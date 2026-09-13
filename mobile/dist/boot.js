// P0 骨架验证脚本
const statusEl = document.getElementById('status')
const detailEl = document.getElementById('detail')

const info = {
  ua: navigator.userAgent,
  tauri: typeof window.__TAURI_INTERNALS__ !== 'undefined',
  origin: location.origin,
  protocol: location.protocol,
}

if (statusEl) {
  statusEl.textContent = '页面已加载'
  statusEl.className = 'ok'
}
if (detailEl) {
  detailEl.textContent = JSON.stringify(info, null, 2)
}

// 探测本地 axum 引擎（P1 阶段会由 Tauri Rust 侧启动）
// 探测内嵌引擎，并把结果映射为可视状态（便于自动化验证）
fetch('http://127.0.0.1:8080/api/v1/health', { cache: 'no-store' })
  .then((r) => r.text().then((t) => ({ ok: r.ok, status: r.status, body: t })))
  .then((res) => {
    if (detailEl) detailEl.textContent += `\n\n本地引擎: 已连接 (HTTP ${res.status}) ${res.body}`
    const seal = document.querySelector('.seal')
    if (seal) { seal.style.background = '#6FB26F'; seal.title = 'engine-ok' }
    document.body.dataset.engine = 'ok'
  })
  .catch((e) => {
    if (detailEl) detailEl.textContent += '\n\n本地引擎: 连接失败\n' + e.message
    const seal = document.querySelector('.seal')
    if (seal) { seal.style.background = '#D9534F'; seal.title = 'engine-fail' }
    document.body.dataset.engine = 'fail'
  })
