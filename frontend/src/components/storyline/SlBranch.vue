<!--
  SlBranch.vue — 副线节点 + 递归子节点

  用 SFC 模板递归：自引用 <SlBranch :node="child" :depth="depth+1" />
  因为 Vue 3 SFC 支持 name 自引用。

  Props:
    - node: Storyline（当前副线）
    - depth: 深度（用于缩进）
    - isLast: 是否是父节点的最后一个子（用于连接线样式）
    - childrenMap: { parentId → child Storyline[] }
    - expandedIds: { id → bool }
  Emits: 同 SlNode
-->
<template>
  <div class="sl-branch" :class="[`depth-${depth}`, isLast ? 'is-last' : 'has-next']">
    <SlNode
      :node="node"
      :depth="depth"
      :has-children="hasChildren"
      :expanded="expanded"
      @toggle="emit('toggle', node.id)"
      @edit="(n) => emit('edit', n)"
      @delete="(n) => emit('delete', n)"
      @add-child="(id) => emit('add-child', id)"
    />

    <div v-if="expanded && childList.length > 0" class="sl-children">
      <SlBranch
        v-for="(child, i) in childList"
        :key="child.id"
        :node="child"
        :depth="depth + 1"
        :is-last="i === childList.length - 1"
        :children-map="childrenMap"
        :expanded-ids="expandedIds"
        @toggle="(id) => emit('toggle', id)"
        @edit="(n) => emit('edit', n)"
        @delete="(n) => emit('delete', n)"
        @add-child="(id) => emit('add-child', id)"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { Storyline } from '@/types/narrative'
import SlNode from './SlNode.vue'

const props = defineProps<{
  node: Storyline
  depth: number
  isLast: boolean
  childrenMap: Record<string, Storyline[]>
  expandedIds: Record<string, boolean>
}>()

const emit = defineEmits<{
  toggle: [id: string]
  edit: [n: Storyline]
  delete: [n: Storyline]
  'add-child': [parentId: string]
}>()

const childList = computed<Storyline[]>(() => props.childrenMap[props.node.id] || [])
const hasChildren = computed<boolean>(() => childList.value.length > 0)
const expanded = computed<boolean>(() => props.expandedIds[props.node.id] !== false)
</script>

<style scoped>
.sl-branch {
  position: relative;
}

/* 子节点的左侧缩进 + 连接线（虚线） */
.sl-children {
  margin-left: 28px;
  border-left: 1px dashed var(--border-default);
  padding-left: 0;
}

/* 最后一个子：连接线不延伸到根；其他：连接线继续延伸 */
.sl-branch.has-next > .sl-children::before {
  content: '';
  display: block;
  position: absolute;
  left: 14px;
  top: 0;
  bottom: 0;
  width: 1px;
  border-left: 1px dashed var(--border-default);
}
</style>
