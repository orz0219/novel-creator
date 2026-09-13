#!/usr/bin/env node
/**
 * 校验模板里的 data-act 是否都有处理分支。
 *
 * 背景：模板写了 data-act="xxx" 但事件分支漏了，表现就是「点了没反应」。
 *   node tools/check-ui-actions.js
 */
const fs = require('fs')
const path = require('path')

const app = fs.readFileSync(path.join(__dirname, '..', 'dist', 'js', 'app.js'), 'utf8')
const used = new Set([...app.matchAll(/data-act="([^"]+)"/g)].map((m) => m[1]))
const handled = new Set([...app.matchAll(/act === '([^']+)'/g)].map((m) => m[1]))

const missing = [...used].filter((a) => !handled.has(a))
const unused = [...handled].filter((a) => !used.has(a))

console.log(`data-act 使用: ${used.size}  处理分支: ${handled.size}`)
if (missing.length) {
  console.error('\n❌ 这些 data-act 没有处理分支（点了会没反应）:')
  for (const m of missing) console.error('   ' + m)
  process.exit(1)
}
console.log('✅ 所有 data-act 都有处理分支')
if (unused.length) console.log('（提示）有分支但模板未使用: ' + unused.join(', '))
