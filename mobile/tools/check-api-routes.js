#!/usr/bin/env node
/**
 * 校验前端调用的 API 路径是否都能在后端找到。
 *
 * 背景：前端调错路径、或后端改了路由而前端没跟上，在 App 里只表现为
 * 「点了没反应」或「读取失败」，很难定位。改完前后端后跑一遍：
 *   node tools/check-api-routes.js
 */
const fs = require('fs')
const path = require('path')

const root = path.join(__dirname, '..', '..')
const engineApi = path.join(root, 'crates', 'sqlite-engine', 'src', 'api', 'mod.rs')
const distJs = path.join(__dirname, '..', 'dist', 'js')

// 后端路由 → 正则（{xxx} 视为「单段通配」）
const apiSrc = fs.readFileSync(engineApi, 'utf8')
const routes = [...apiSrc.matchAll(/\.route\("([^"]+)"/g)].map((m) => m[1])
const toRegex = (route) =>
  new RegExp('^' + route.split(/\{[^}]+\}/).map(escapeRe).join('[^/]+') + '$')
function escapeRe(s) {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}
const patterns = routes.map(toRegex)

// 前端用到的路径
const collect = (file) => {
  const s = fs.readFileSync(file, 'utf8')
  const out = new Set()
  for (const m of s.matchAll(/(?:get|post|put|del)\(\s*(['"`])(\/[^'"`]+)\1/g)) out.add(m[2])
  return out
}
const used = new Set([...collect(path.join(distJs, 'api.js')), ...collect(path.join(distJs, 'app.js'))])

const BASE = '/api/v1'
const bad = []
let ok = 0

for (const raw of used) {
  // 去 query；把 ${...} 模板变量换成 X（占位一段）
  const p = raw.replace(/\?.*$/, '').replace(/\$\{[^}]*\}/g, 'X')
  const full = p.startsWith(BASE) ? p : BASE + p
  // 把 X 当通配段参与匹配
  const probe = full.split('/').map((seg) => (seg === 'X' ? 'x' : seg)).join('/')
  const matched = patterns.some((rx) => rx.test(full) || rx.test(probe))
  if (matched) ok++
  else bad.push(raw)
}

console.log(`后端路由: ${routes.length}`)
console.log(`前端调用: ${used.size}`)
console.log(`匹配成功: ${ok}`)
if (bad.length) {
  console.error('\n❌ 后端找不到这些路径:')
  for (const b of bad) console.error('   ' + b)
  process.exit(1)
}
console.log('\n✅ 前端所有调用都能对上后端路由')
