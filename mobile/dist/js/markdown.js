// Markdown 渲染（手机端）。
//
// 与电脑端 `frontend/src/utils/markdown.ts` 用**同一套规则与同一对库**，
// 这样两端看到的排版一致。
//
// 为什么不用原先手写的轻量渲染器：那个只认标题 / 列表 / 引用 / 分隔线 /
// 行内粗体，**不认代码块与表格**。而 AI 的回复里这两样很常见——
// 实测一条回复里有 6 个代码围栏、105 个表格竖线，于是它们原样显示成一堆
// ``` 和 |，看起来就"不是 Markdown 格式"。
//
// 依赖是本地单文件（`vendor/` 下），不走网络；手机端没有打包工具，
// 所以直接把电脑端 node_modules 里的 ESM 单文件复制过来。
// 两个文件都用 `.js` 后缀：`.mjs` 在静态服务器上常被当成 application/octet-stream，
// 浏览器会拒绝把它当作模块加载（实测就是这样加载失败、页面一片空白）。
import { marked } from './vendor/marked.esm.js'
import DOMPurify from './vendor/purify.es.js'

/*
 * 自定义 strong 规则：强制**单行配对**（照搬电脑端的修正）。
 *
 * marked 默认的 `**` 规则允许跨行（`**a\nb**` 也算 strong）。当模型在
 * `> 引用` 块外写一个 `**`、块内再写一个 `**` 时，marked 会贪婪地把块外的开标记
 * 与块内的第一对配对、再跨块找闭合，结果把整段引用包成一个大加粗——用户看到
 * 整段变色、重点全无。模型的强调标记都在同一行，单行规则足够根治这个问题。
 */
const inlineStrongExtension = {
  name: 'strong',
  level: 'inline',
  start(src) {
    const idx = src.indexOf('**')
    if (idx === -1) return undefined
    // `** 文字**` 这种（紧跟空格）其实是普通文本，不当强调
    if (src[idx + 2] === ' ') return undefined
    return idx
  },
  tokenizer(src) {
    const match = /^\*\*([^*\n][^*\n]*?)\*\*/.exec(src)
    if (!match) return undefined
    return {
      type: 'strong',
      raw: match[0],
      text: match[1],
      tokens: [{ type: 'text', raw: match[1], text: match[1] }],
    }
  },
}

marked.use({ extensions: [inlineStrongExtension] })
// gfm：表格、任务列表、删除线；breaks：单个换行也断行（对话里模型常这么写）
marked.setOptions({ gfm: true, breaks: true })

/**
 * 把 Markdown 源文本渲染成**已消毒**的 HTML。
 *
 * 消毒是必须的，不能只靠 marked 的 `html: false`——实测它会放行原始
 * `<script>` 与 `javascript:` 链接（两者都原样出现在输出里）。
 * 内容虽然来自本机 AI，但没有理由让页面上出现可执行脚本。
 */
export function renderMarkdown(src) {
  if (!src) return ''
  const html = marked.parse(src, { async: false })
  return DOMPurify.sanitize(html)
}
