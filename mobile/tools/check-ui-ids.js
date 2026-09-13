#!/usr/bin/env node
/**
 * 校验前端引用的 DOM id 是否都有定义。
 *
 * 背景：曾经出现过「事件绑定引用了不存在的元素」导致整个设置页失效，
 * 而这类问题在浏览器里只表现为「点了没反应」，极难排查。
 * 每次改 UI 后跑一遍：
 *   node tools/check-ui-ids.js
 */
const fs = require('fs')
const path = require('path')

const root = path.join(__dirname, '..', 'dist')
const js = fs.readFileSync(path.join(root, 'js', 'app.js'), 'utf8')
const html = fs.readFileSync(path.join(root, 'index.html'), 'utf8')

const used = new Set([...js.matchAll(/\$\('([^']+)'\)/g)].map((m) => m[1]))
const defined = new Set([
  ...[...html.matchAll(/id="([^"]+)"/g)].map((m) => m[1]),
  ...[...js.matchAll(/id="([a-zA-Z0-9_-]+)"/g)].map((m) => m[1]),
])

const missing = [...used].filter((id) => !defined.has(id)).sort()

console.log(`引用的 DOM id: ${used.size}`)
console.log(`定义的元素 id: ${defined.size}`)
if (missing.length) {
  console.error('\n❌ 以下 id 被引用但没有定义（会导致事件绑定抛异常）:')
  for (const id of missing) console.error('   - ' + id)
  process.exit(1)
}
console.log('\n✅ 所有引用的 id 都有定义')
