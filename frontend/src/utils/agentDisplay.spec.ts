import { describe, it, expect } from 'vitest'
import { sanitizeAssistantText, toolAction, toolSubject } from './agentDisplay'

describe('sanitizeAssistantText', () => {
  it('剥离 XML 风格的工具调用并说明原因', () => {
    const raw = [
      '好的，我来修改。',
      '<tool_calls>',
      '<invoke name="get_entity">',
      '<parameter name="id">133b6607-1cc8-4161-92b4-40c54857e9c0</parameter>',
      '</invoke>',
      '</tool_calls>',
    ].join('\n')

    const out = sanitizeAssistantText(raw)
    expect(out).toContain('好的，我来修改。')
    expect(out).not.toContain('invoke')
    expect(out).not.toContain('parameter')
    expect(out).toContain('不受支持')
  })

  it('未闭合的 XML 调用（流式中途）也要剥离', () => {
    const out = sanitizeAssistantText('正文\n<invoke name="get_entity">\n<parameter name="id">abc')
    expect(out).toContain('正文')
    expect(out).not.toContain('<invoke')
  })

  it('本项目格式的工具调用替换为一句说明', () => {
    const out = sanitizeAssistantText('我来改<<CALL_TOOL>>{"name":"x","input":{}}<<END>>')
    expect(out).toContain('我来改')
    expect(out).not.toContain('CALL_TOOL')
    expect(out).not.toContain('"name"')
  })

  it('剥离「全角竖线」形态的标记（模型真实输出过的形态）', () => {
    // 数据库里的真实样本：本体标记被写成 ｜｜CALL_TOOL｜｜（U+FF5C 全角竖线）
    const RAW =
      '<\uFF5C\uFF5CCALL_TOOL\uFF5C\uFF5C>{"name":"create_rule","input":{"rule_level":"核心"}}<<END>>'
    const out = sanitizeAssistantText(RAW)
    expect(out).not.toContain('CALL_TOOL')
    expect(out).not.toContain('create_rule')
    expect(out).not.toContain('\uFF5C')
  })

  it('普通中文里的全角竖线不受影响', () => {
    const s = '表格｜列名｜说明'
    expect(sanitizeAssistantText(s)).toBe(s)
  })

  it('剥离流式边界残留的标记前缀', () => {
    expect(sanitizeAssistantText('正文<<CAL')).toBe('正文')
    expect(sanitizeAssistantText('正文<<')).toBe('正文')
  })

  it('普通正文原样保留', () => {
    const s = '你好，我们先聊聊这个世界的基本规则。'
    expect(sanitizeAssistantText(s)).toBe(s)
  })

  it('含尖括号的普通正文不受影响', () => {
    const s = '这段用 < 和 > 表示比较关系。'
    expect(sanitizeAssistantText(s)).toBe(s)
  })
})

describe('toolAction', () => {
  it('把内部工具名翻译成中文动作', () => {
    expect(toolAction('create_character')).toBe('创建角色')
    expect(toolAction('revise_entity')).toBe('修改实体')
    expect(toolAction('create_foreshadow')).toBe('埋下伏笔')
  })

  it('未知工具回退到原名而不是显示"未知"', () => {
    expect(toolAction('some_future_tool')).toBe('some_future_tool')
    expect(toolAction('')).toBe('调用工具')
  })
})

describe('toolSubject', () => {
  it('优先取 name 作为操作对象', () => {
    expect(toolSubject({ name: '王久财', id: 'abc' })).toBe('王久财')
  })

  it('没有可读对象时返回空串（不把 UUID 当名字）', () => {
    expect(toolSubject({ id: 'abc', world_id: 'def' })).toBe('')
    expect(toolSubject(null)).toBe('')
  })
})

describe('sanitizeAssistantText - 畸形 XML 标签', () => {
  // 模型在工具调用失败重试时，会把 `<>` 当成标签分隔符写坏，
  // 吐出 `<<> calls>` / `</<> invoke>` 这类畸形标签（实测出现过）。
  it('剥离畸形标签的整块调用（含其中的 JSON）', () => {
    const raw =
      '重试\n<<> calls>\n<<> invoke name="CALL_TOOL">{"name":"get_character_profile","input":{"id":"x"}}</<> parameter>\n</<> invoke>\n</<> calls>'
    const out = sanitizeAssistantText(raw)
    expect(out).not.toContain('<<>')
    expect(out).not.toContain('</<>')
    expect(out).not.toContain('get_character_profile')
    expect(out).toContain('重试')
    expect(out).toContain('不受支持的格式')
  })

  it('单个畸形标签也被清掉', () => {
    const out = sanitizeAssistantText('正文 <<> parameter> 结尾')
    expect(out).not.toContain('parameter')
    expect(out).toContain('正文')
    expect(out).toContain('结尾')
  })

  it('正常尖括号内容不受影响', () => {
    const out = sanitizeAssistantText('比较 a < b 且 c > d 的情况')
    expect(out).toContain('a < b')
    expect(out).toContain('c > d')
  })

  it('剥离 DSML 包裹的畸形工具调用（数据库真实样本）', () => {
    const bar = '\uFF5C'
    const raw = [
      `<${bar}${bar}DSML${bar}${bar} calls>`,
      `<${bar}${bar}DSML${bar}${bar} invoke name="get_character_profile">`,
      `<${bar}${bar}DSML${bar}${bar} parameter name="id" string="true">133b6607-1cc8-4161-92b4-40c54857e9c0</${bar}${bar}DSML${bar}${bar} parameter>`,
      `</${bar}${bar}DSML${bar}${bar} invoke>`,
      `</${bar}${bar}DSML${bar}${bar} calls>`,
    ].join('\n')
    const out = sanitizeAssistantText(raw)
    expect(out).not.toContain('DSML')
    expect(out).not.toContain('calls')
    expect(out).not.toContain('invoke')
    expect(out).not.toContain('parameter')
    expect(out).not.toContain('133b6607')
    expect(out).not.toContain(bar)
    expect(out).toContain('不受支持')
  })

  it('剥离 ASCII ||DSML|| 包裹的畸形标签', () => {
    const raw = '<||DSML|| calls>\\n<||DSML|| invoke name="get_entity">\\n</||DSML|| invoke>\\n</||DSML|| calls>'
    const out = sanitizeAssistantText(raw)
    expect(out).not.toContain('DSML')
    expect(out).not.toContain('invoke')
    expect(out).toContain('不受支持')
  })
})
