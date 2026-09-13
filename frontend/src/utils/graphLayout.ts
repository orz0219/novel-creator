// 关系图谱的轻量力导向布局。
//
// 不引入 d3，使用确定性的初始分组 + 弹簧/斥力迭代：
// - 同类型节点先放在各自聚类中心附近；
// - 有关系的节点互相拉近；
// - 所有节点互相排斥，避免重叠；
// - 最后钳制在画布内。
//
// 布局结果只依赖 nodes / edges / 画布尺寸，因此同一份数据每次得到相同的图，
// 不会出现刷新一次位置全变的情况。

export interface LayoutNode {
  id: string
  type: string
}

export interface LayoutEdge {
  from: string
  to: string
}

export interface ForceLayoutOptions {
  width: number
  height: number
  /** 迭代次数；节点多时可适当降低。 */
  iterations?: number
  /** 画布内边距。 */
  padding?: number
}

export interface Point {
  x: number
  y: number
}

const GOLDEN_ANGLE = 2.399963229728653

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value))
}

/**
 * 计算力导向布局，返回 `id -> { x, y }`。
 */
export function computeForceLayout(
  nodes: LayoutNode[],
  edges: LayoutEdge[],
  options: ForceLayoutOptions,
): Record<string, Point> {
  const { width, height, iterations = 260, padding = 70 } = options
  if (nodes.length === 0) return {}
  if (nodes.length === 1) {
    return { [nodes[0].id]: { x: width / 2, y: height / 2 } }
  }

  const cx = width / 2
  const cy = height / 2

  // 按类型分组，给每组一个聚类中心，避免所有节点从一个点开始。
  const groups = new Map<string, LayoutNode[]>()
  for (const node of nodes) {
    const list = groups.get(node.type) ?? []
    list.push(node)
    groups.set(node.type, list)
  }
  const typeNames = [...groups.keys()].sort()
  const clusterCount = typeNames.length
  const clusterRadius = clusterCount > 1 ? Math.min(width, height) * 0.27 : 0

  const positions: Point[] = new Array(nodes.length)
  const indexOf = new Map<string, number>()

  typeNames.forEach((type, typeIndex) => {
    const members = groups.get(type) ?? []
    const angle = (Math.PI * 2 * typeIndex) / Math.max(clusterCount, 1)
    const groupCx = clusterCount > 1 ? cx + clusterRadius * Math.cos(angle) : cx
    const groupCy = clusterCount > 1 ? cy + clusterRadius * Math.sin(angle) : cy

    members.forEach((node, i) => {
      const localRadius = i === 0 ? 0 : 38 * Math.sqrt(i)
      const localAngle = i * GOLDEN_ANGLE
      const index = nodes.findIndex((n) => n.id === node.id)
      indexOf.set(node.id, index)
      positions[index] = {
        x: groupCx + localRadius * Math.cos(localAngle),
        y: groupCy + localRadius * Math.sin(localAngle),
      }
    })
  })

  const repulsion = (width * height) / Math.max(nodes.length, 1) * 0.9
  const idealDistance = Math.min(width, height) * 0.18
  const spring = 0.018
  const gravity = 0.008
  const maxStep = 26

  for (let iter = 0; iter < iterations; iter++) {
    const dispX = new Array(nodes.length).fill(0)
    const dispY = new Array(nodes.length).fill(0)

    // 1) 节点间斥力
    for (let i = 0; i < nodes.length; i++) {
      for (let j = i + 1; j < nodes.length; j++) {
        let dx = positions[i].x - positions[j].x
        let dy = positions[i].y - positions[j].y
        let dist = Math.sqrt(dx * dx + dy * dy)
        if (dist < 0.01) {
          dx = (i - j) * 0.5 || 0.5
          dy = (j - i) * 0.5 || 0.5
          dist = Math.sqrt(dx * dx + dy * dy)
        }
        const force = repulsion / (dist * dist)
        const fx = (dx / dist) * force
        const fy = (dy / dist) * force
        dispX[i] += fx
        dispY[i] += fy
        dispX[j] -= fx
        dispY[j] -= fy
      }
    }

    // 2) 有边关系的节点互相吸引
    for (const edge of edges) {
      const a = indexOf.get(edge.from)
      const b = indexOf.get(edge.to)
      if (a == null || b == null) continue
      const dx = positions[a].x - positions[b].x
      const dy = positions[a].y - positions[b].y
      const dist = Math.sqrt(dx * dx + dy * dy) || 0.01
      const force = (dist - idealDistance) * spring
      const fx = (dx / dist) * force
      const fy = (dy / dist) * force
      dispX[a] -= fx
      dispY[a] -= fy
      dispX[b] += fx
      dispY[b] += fy
    }

    // 3) 轻微向画布中心聚拢，防止孤立节点飞出去
    for (let i = 0; i < nodes.length; i++) {
      dispX[i] += (cx - positions[i].x) * gravity
      dispY[i] += (cy - positions[i].y) * gravity
    }

    // 4) 应用位移，后期逐渐降温
    const alpha = Math.max(0.08, 1 - iter / iterations)
    for (let i = 0; i < nodes.length; i++) {
      const dx = clamp(dispX[i] * alpha, -maxStep, maxStep)
      const dy = clamp(dispY[i] * alpha, -maxStep, maxStep)
      positions[i].x = clamp(positions[i].x + dx, padding, width - padding)
      positions[i].y = clamp(positions[i].y + dy, padding, height - padding)
    }
  }

  const result: Record<string, Point> = {}
  nodes.forEach((node, i) => {
    result[node.id] = {
      x: Number.isFinite(positions[i].x) ? positions[i].x : cx,
      y: Number.isFinite(positions[i].y) ? positions[i].y : cy,
    }
  })
  return result
}
