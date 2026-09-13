import type { ProfileField } from '@/components/ui/ProfilePanel.vue'

/**
 * 「阶段弧线」（arc_stages）在档案面板里的展示配置 —— 人物 / 势力 / 地点共用。
 *
 * 后端把三个阶段字段（arc_stages / arc_stages_mode / remove_arc_stages）暴露给 AI，
 * 面板这里只读展示 arc_stages；戏份用 Light/Medium/Heavy 角标映射成「轻/中/重」。
 */
export const ARC_STAGES_FIELD: ProfileField = {
  key: 'arc_stages',
  label: '阶段弧线',
  shape: 'object-list',
  readOnly: true,
  primaryKey: 'stage',
  badgeKey: 'screen_weight',
  badgeLabels: { Light: '轻', Medium: '中', Heavy: '重' },
  subFields: [
    { key: 'role', label: '身份' },
    { key: 'goal', label: '目标' },
    { key: 'function', label: '功能' },
    { key: 'entry_trigger', label: '进入' },
    { key: 'status', label: '现状' },
  ],
}
