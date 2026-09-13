// 工具调用的面向用户展示辅助。
//
// 设计目标：用户看到的是「AI 做了什么」，而不是 `create_character` 这类内部标识
// 或一大段 JSON。折叠态只占一行，需要细节时再展开。

/** 工具名 → 面向用户的中文动作名。 */
const TOOL_ACTIONS: Record<string, string> = {
  // 实体
  create_character: '创建角色',
  create_location: '创建地点',
  create_faction: '创建势力',
  create_item: '创建物品',
  revise_entity: '修改实体',
  retire_entity: '删除实体',
  create_relation: '建立关系',
  end_relation: '结束关系',
  get_entity: '查询实体',
  list_entities: '罗列实体',
  get_character_profile: '读取角色档案',
  update_character_profile: '更新角色档案',
  // 叙事 / 故事线
  create_node: '创建叙事节点',
  revise_node: '修改叙事节点',
  remove_node: '删除叙事节点',
  get_node: '查询叙事节点',
  list_nodes: '罗列叙事节点',
  create_storyline: '创建故事线',
  revise_storyline: '修改故事线',
  retire_storyline: '删除故事线',
  list_storylines: '罗列故事线',
  // 伏笔 / 规则
  create_foreshadow: '埋下伏笔',
  revise_foreshadow: '修改伏笔',
  retire_foreshadow: '删除伏笔',
  list_foreshadows: '罗列伏笔',
  create_rule: '创建世界规则',
  revise_rule: '修改世界规则',
  retire_rule: '删除世界规则',
  get_rule: '查询世界规则',
  list_rules: '罗列世界规则',
  // 世界 / 项目 / 历史
  get_main_world: '读取世界设定',
  update_main_world: '更新世界设定',
  get_project: '读取项目信息',
  list_projects: '罗列项目',
  update_project: '更新项目',
  create_event: '记录历史事件',
  create_fact: '记录事实',
  list_events: '罗列历史事件',
  list_facts: '罗列事实',
  // 快照 / 引导
  create_snapshot: '创建快照',
  list_snapshots: '罗列快照',
  delete_snapshot: '删除快照',
  confirm_step: '推进引导阶段',
  tool_format_error: '工具格式错误',
  tool_call_json_error: '工具参数错误',
  // 基础
  echo: '连通性检查',
  ask_question: '向你提问',
}

/** 取工具的中文动作名；未知工具回退到原名（好过显示"未知"）。 */
export function toolAction(name: string): string {
  if (!name) return '调用工具'
  return TOOL_ACTIONS[name] ?? name
}

/**
 * 从工具入参里挑一句最能说明「操作对象」的摘要，用于折叠态那一行。
 *
 * 只认语义明确的字段（`name` / `title` / `entity_name`）——
 * 刻意**不**回退到"任意字符串字段"，否则会把 `id`、`world_id` 这类 UUID
 * 当成对象名显示（如「修改实体「133b6607-…」」），反而更难看懂。
 */
export function toolSubject(input: unknown): string {
  if (input == null || typeof input !== 'object') return ''
  const obj = input as Record<string, unknown>
  for (const key of ['name', 'title', 'entity_name']) {
    const v = obj[key]
    if (typeof v === 'string' && v.trim()) return v.trim()
  }
  return ''
}

/** 内部协议标记：这些是后端与前端之间的解析约定，绝不该出现在用户视野里。 */
const CALL_TOOL = '<<CALL_TOOL>>'
const ASK_QUESTION = '<<ASK_QUESTION>>'
const TOOL_RESULT = '<<TOOL_RESULT>>'
const END = '<<END>>'

/**
 * XML 风格的工具调用（部分模型的「母语」格式，本项目不识别、不会执行）。
 *
 * 例：`<invoke name="get_entity"><parameter name="id">…</parameter></invoke>`
 * 未闭合到文末的也算（流式进行中就会出现这种半截形态）。
 *
 * `calls` 也算标签名：模型写坏时会把 `<tool_calls>` 写成 `<calls>`（见 MALFORMED_TAG）。
 */
const XML_CALL_BLOCK =
  /<(?:tool_calls|calls)>[\s\S]*?(?:<\/(?:tool_calls|calls)>|$)|<invoke\b[\s\S]*?(?:<\/invoke>|$)|<function_call>[\s\S]*?(?:<\/function_call>|$)/gi

/** 上一条剥离后可能剩下的孤立标签（如单独出现的 `<parameter …>`）。 */
const XML_CALL_LEFTOVER =
  /<\/?(?:tool_calls|calls|invoke|parameter|function_call|function|arguments)\b[^>]*>/gi

/**
 * DSML 包裹的畸形标签：模型会把标签写成 `<\uFF5C\uFF5CDSML\uFF5C\uFF5C calls>`、
 * `</\uFF5C\uFF5CDSML\uFF5C\uFF5C invoke>` 这种形式。
 *
 * `\uFF5C` 是全角竖线，在界面上与 `|` 几乎一样；模型通过它把 XML 标签名包在
 * DSML 分隔符里。若先交给 `FULLWIDTH_MARKER` 处理，会被误改写成 `<<DSML>>`，
 * 导致整段调用无法被后续 XML 规则识别、原样泄漏给用户。
 */
const DSML_TAG = /(<\/?)(?:\uFF5C{2}|\|{2})DSML(?:\uFF5C{2}|\|{2})\s*/gi

/**
 * 畸形标签：模型把 `<>` 当成了标签分隔符，写出 `<<> calls>`、`</<> invoke name="…">`
 * 这种不合法但看得出来意图的东西。
 *
 * 先把 `<<>` / `</<>` 归一化成正常标签起始，上面两条规则才能认出并按 XML 调用处理；
 * 否则这些片段会原样留在气泡里，用户看到的是一串看不懂的符号。
 */
const MALFORMED_TAG = /<(\/?)<>\s*/g

/**
 * 全角竖线形态的标记：`｜｜CALL_TOOL｜｜`。
 *
 * `｜` 是 U+FF5C，在界面上与 `<<` `>>` 几乎无法分辨，但字节完全不同 ——
 * 模型偶尔会这样写，导致后端识别失败、整段调用被当作正文显示出来。
 * 这里先归一化成 ASCII 形态，交给下面的清理逻辑统一处理。
 */
const FULLWIDTH_MARKER = /\uFF5C\uFF5C([A-Z_]+)\uFF5C\uFF5C/g

/**
 * 清理助手正文里的内部协议内容，避免把「实现细节」暴露给用户。
 *
 * 处理三类：
 * - `<<CALL_TOOL>>{...}<<END>>`（本项目格式，但未被执行，例如模型格式写错）→ 换成人话说明；
 * - XML 风格工具调用（模型原生格式，从未被执行）→ 剥离并换成人话说明；
 * - 孤立的标记 / 未写完的标记前缀（流式边界情况）→ 直接剥离。
 */
export function sanitizeAssistantText(text: string): string {
  if (!text) return ''

  // 0) 先还原 DSML 包裹的标签（`<\uFF5C\uFF5CDSML\uFF5C\uFF5C calls>` → `<calls>`），
  //    必须在 FULLWIDTH_MARKER 之前，否则 `DSML` 会被误写成 `<<DSML>>`。
  let out = text.replace(DSML_TAG, '$1')

  // 0.5) 归一化全角竖线形态的标记，让后续逻辑能统一识别
  out = out.replace(FULLWIDTH_MARKER, '<<$1>>')

  // 0.6) 归一化畸形标签（`<<> calls>` → `<calls>`），交给下面的 XML 逻辑统一剥离
  out = out.replace(MALFORMED_TAG, '<$1')

  // 1) 本项目格式的完整工具调用：连同其中的 JSON 一起替换掉
  const callStart = out.indexOf(CALL_TOOL)
  if (callStart !== -1) {
    const endIdx = out.indexOf(END, callStart + CALL_TOOL.length)
    const before = out.slice(0, callStart)
    const after = endIdx === -1 ? '' : out.slice(endIdx + END.length)
    out = `${before}\n\n（此处 AI 尝试调用工具，但未形成可识别格式）\n\n${after}`
  }

  // 2) XML 风格工具调用：整体剥离（这类调用不会被执行，展示出来只会让人困惑）
  const hadXmlCall = XML_CALL_BLOCK.test(out)
  XML_CALL_BLOCK.lastIndex = 0
  if (hadXmlCall) {
    out = out.replace(XML_CALL_BLOCK, '')
  }
  // 孤立标签单独处理：即使没有配成完整的块（流式半截、或只剩一个 `<parameter>`），
  // 也不该显示给用户。这一步不能只在检测到块时才做，否则残余标签会留在气泡里。
  XML_CALL_LEFTOVER.lastIndex = 0
  out = out.replace(XML_CALL_LEFTOVER, '')
  if (hadXmlCall) {
    out = `${out}\n\n（AI 曾尝试调用工具，但使用了不受支持的格式，该次调用未执行）`
  }

  // 3) 其它孤立标记
  out = out
    .split(ASK_QUESTION)
    .join('')
    .split(TOOL_RESULT)
    .join('')
    .split(CALL_TOOL)
    .join('')
    .split(END)
    .join('')

  // 4) 末尾可能残留的未完成标记前缀（如流式中途的 `<<CAL`）
  out = out.replace(/<<[A-Z_]*$/, '')

  return out.trim()
}
