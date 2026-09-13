/**
 * 档案字段值的展示文本。
 *
 * 关键约束：**枚举字段的存储值是英文，界面上必须显示中文标签。**
 *
 * 后端契约要求 `age_range` / `gender` / `role_in_story` 存 `Adult` / `Male` /
 * `Protagonist` 这类规范值（传中文会被拒收），但把原始值直接铺在界面上，
 * 等于把实现细节暴露给使用者——档案里明明白白写着 `YoungAdult` 而不是「青年」。
 * 因此展示时按字段的 options 做一次值 → 标签映射。
 */

export interface DisplayOption {
  value: string
  label: string
}

export interface DisplayField {
  key: string
  options?: DisplayOption[]
}

/** 值 → 展示文本。空值返回空串，由调用方决定显示「未填写」。 */
export function displayProfileValue(
  key: string,
  value: unknown,
  fields: readonly DisplayField[],
): string {
  if (value === null || value === undefined) return ''

  if (Array.isArray(value)) {
    return value.filter(Boolean).map(String).join('、')
  }
  if (typeof value === 'object') {
    return JSON.stringify(value)
  }

  const text = String(value).trim()
  if (!text) return ''

  const field = fields.find((f) => f.key === key)
  const hit = field?.options?.find((o) => o.value === text)
  return hit ? hit.label : text
}
