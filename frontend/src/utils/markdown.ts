// Markdown 渲染工具：marked 解析 + DOMPurify 消毒（防 XSS）。
//
// 已知问题：marked@18 默认的 `**` strong 规则允许跨行（`**a\nb**` 也算 strong）。
// 当 LLM 在 `> blockquote` 块**外**写 `**`、块**内**写另一对 `**` 时，marked 贪婪地把
// 块外的开 strong 跟块内的第一对 `**` 配对、再跨块找第二对 `**` 闭合，结果就是 strong 嵌套
// 错乱、把整段 quote 包成一个大 strong（用户看到整段红色、无重点）。
//
// 修法：用 marked extension 替换默认 strong 规则，强制**单行**配对（开 `**` 必须在同行
// 找到闭 `**`，中间不能有换行）。LLM 的强标记全在同一行——单行规则足够，根治跨块错位。
import { marked } from 'marked'
import DOMPurify from 'dompurify'

// 自定义 strong：单行配对（不允许跨行）
const inlineStrongExtension = {
  name: 'strong',
  level: 'inline' as const,
  start(src: string): number | undefined {
    const idx = src.indexOf('**')
    if (idx === -1) return undefined
    // 跳过 `**` 后面紧跟空格的（如 `** bold**` 实际是普通文本）
    if (src[idx + 2] === ' ') return undefined
    return idx
  },
  tokenizer(src: string) {
    // 单行匹配：开 `**` 之后到下一个 `**`，中间不能有换行
    const match = /^\*\*([^*\n][^*\n]*?)\*\*/.exec(src)
    if (!match) return undefined
    return {
      type: 'strong',
      raw: match[0],
      text: match[1],
      tokens: [{ type: 'text', raw: match[1], text: match[1] }],
    }
  },
  // 不写 renderer：让 marked 用默认 strong 渲染器（输出 <strong>text</strong>）
}

marked.use({ extensions: [inlineStrongExtension] })
marked.setOptions({ gfm: true, breaks: true })

// 外链统一新标签页打开，避免跳出当前会话
DOMPurify.addHook('afterSanitizeAttributes', (node) => {
  if (node.tagName === 'A') {
    node.setAttribute('target', '_blank')
    node.setAttribute('rel', 'noopener noreferrer')
  }
})

/** 将 Markdown 源渲染为已消毒的 HTML 字符串。 */
export function renderMarkdown(src: string): string {
  if (!src) return ''
  const html = marked.parse(src, { async: false }) as string
  return DOMPurify.sanitize(html)
}

/** 把工具入参/结果（可能是 JSON 字符串或对象）美化为可读文本。 */
export function prettyJson(v: unknown): string {
  if (typeof v === 'string') {
    const t = v.trim()
    if (t.startsWith('{') || t.startsWith('[')) {
      try {
        return JSON.stringify(JSON.parse(t), null, 2)
      } catch {
        return t
      }
    }
    return t
  }
  try {
    return JSON.stringify(v, null, 2)
  } catch {
    return String(v)
  }
}
