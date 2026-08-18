<template>
  <div class="story-tree">
    <template v-for="node in nodes" :key="node.id">
      <StoryNode
        :node="node"
        :expanded="expanded[node.id]"
        :has-children="(node.children?.length || 0) > 0"
        @toggle="toggleExpand"
        @select="$emit('select', $event)"
        @write="$emit('write', $event)"
      />
      <div v-if="expanded[node.id] && node.children?.length" class="tree-children">
        <StoryTree
          :nodes="node.children"
          :expanded="expanded"
          :depth="depth + 1"
          @select="$emit('select', $event)"
          @write="$emit('write', $event)"
          @toggle="toggleExpand"
        />
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import StoryNode from './StoryNode.vue'
import type { NarrativeNode } from '@/types'

defineProps<{
  nodes: (NarrativeNode & { children?: NarrativeNode[] })[]
  expanded: Record<string, boolean>
  depth: number
}>()
const emit = defineEmits(['select', 'write', 'toggle'])

function toggleExpand(id: string) {
  emit('toggle', id)
}
</script>

<style scoped>
.story-tree { display: flex; flex-direction: column; }
.tree-children { padding-left: var(--space-4); }
</style>