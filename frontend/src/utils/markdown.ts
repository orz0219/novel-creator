// Markdown 渲染工具：marked 解析 + DOMPurify 消毒（防 XSS）。
//
// 聊天内容来自 LLM，必须消毒后再 v-html。对聊天长度的增量流，每次 token
// 追加都整体重渲染一次（参考 deepseek-harness 的流式思路，但聊天体量下
// 全量重解析开销可忽略，故不引入其 mdast 增量冻结方案）。
import { marked } from 'marked'
import DOMPurify from 'dompurify'

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
