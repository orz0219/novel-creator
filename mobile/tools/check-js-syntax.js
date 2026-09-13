#!/usr/bin/env node
/**
 * 语法自检：把 dist/js 下的每个 ES module 交给 Node 按模块解析一遍。
 *
 * 为什么需要它：`node tools/check-ui-ids.js` 这类脚本用的是正则匹配，
 * 语法错误（例如在模板字符串里又写了一对反引号，把字符串提前结束）
 * 它们照样能过，但页面在手机上会整个卡住——实测踩过一次：
 * 首页一直停在「正在连接引擎…」，`#view` 是空的，看不出任何报错。
 *
 * 用法：node tools/check-js-syntax.js
 */
const fs = require('fs')
const os = require('os')
const path = require('path')
const { execFileSync } = require('child_process')

const jsDir = path.join(__dirname, '..', 'dist', 'js')
const files = fs.readdirSync(jsDir).filter((f) => f.endsWith('.js'))
if (files.length === 0) {
  console.error('❌ dist/js 下没有 .js 文件，路径不对？')
  process.exit(1)
}

// Node 只对 .mjs 按 ES module 解析，所以复制一份带后缀的临时文件
const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'novel-syntax-'))
let failed = 0

for (const f of files) {
  const src = path.join(jsDir, f)
  const dest = path.join(tmp, f.replace(/\.js$/, '.mjs'))
  fs.copyFileSync(src, dest)
  try {
    execFileSync(process.execPath, ['--check', dest], { stdio: 'pipe' })
    console.log(`✅ ${f}`)
  } catch (e) {
    failed++
    console.error(`❌ ${f} 语法错误：`)
    console.error(String(e.stderr || e.message).split('\n').slice(0, 12).join('\n'))
  }
}

fs.rmSync(tmp, { recursive: true, force: true })

if (failed > 0) {
  console.error(`\n❌ ${failed} 个文件有语法错误`)
  process.exit(1)
}
console.log(`\n✅ ${files.length} 个文件语法正常`)
