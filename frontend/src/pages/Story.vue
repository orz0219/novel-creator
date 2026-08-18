<template>
  <div class="story-page">
    <div class="page-header">
      <h1 class="page-title">故事结构</h1>
      <button class="btn-primary">+ 新建卷/章节</button>
    </div>

    <div class="story-tree">
      <template v-for="vol in storyStore.tree" :key="vol.id">
        <div class="tree-volume">
          <div class="volume-header" @click="toggleExpand(vol.id)">
            <span class="expand-icon">{{ expanded[vol.id] ? '▼' : '▶' }}</span>
            <span class="volume-title">{{ vol.title }}</span>
            <StatusBadge :status="vol.status.toLowerCase()" :label="vol.status" />
            <span class="volume-meta">{{ vol.description }}</span>
          </div>
          <div class="volume-body" v-if="expanded[vol.id]">
            <template v-for="arc in vol.children" :key="arc.id">
              <div class="tree-arc">
                <div class="arc-header" @click="toggleExpand(arc.id)">
                  <span class="expand-icon">{{ expanded[arc.id] ? '▼' : '▶' }}</span>
                  <span class="arc-title">{{ arc.title }}</span>
                  <StatusBadge :status="arc.status.toLowerCase()" :label="arc.status" />
                  <span class="arc-meta">{{ arc.description }}</span>
                </div>
                <div class="arc-body" v-if="expanded[arc.id]">
                  <template v-for="ch in arc.children" :key="ch.id">
                    <div class="tree-chapter">
                      <div class="chapter-header" @click="toggleExpand(ch.id)">
                        <span class="expand-icon" v-if="ch.children?.length">{{ expanded[ch.id] ? '▼' : '▶' }}</span>
                        <span class="expand-icon" v-else>　</span>
                        <span class="chapter-title">{{ ch.title }}</span>
                        <StatusBadge :status="ch.status.toLowerCase()" :label="ch.status" />
                        <span class="chapter-meta">{{ ch.description }}</span>
                        <button class="write-btn" @click.stop="goToWrite(ch)">写作</button>
                      </div>
                      <div class="chapter-body" v-if="expanded[ch.id] && ch.children?.length">
                        <div v-for="scene in ch.children" :key="scene.id" class="tree-scene" @click="goToWrite(scene)">
                          <span class="scene-dot" :class="scene.status.toLowerCase()"></span>
                          <span class="scene-title">{{ scene.title }}</span>
                          <span class="scene-time">{{ scene.attributes?.time || '' }}</span>
                        </div>
                      </div>
                    </div>
                  </template>
                </div>
              </div>
            </template>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useStoryStore } from '@/stores/story'
import StatusBadge from '@/components/ui/StatusBadge.vue'

const router = useRouter()
const storyStore = useStoryStore()
storyStore.loadMockData()

const expanded = ref<Record<string, boolean>>({ 'vol-1': true, 'arc-2': true, 'ch-4': true })

function toggleExpand(id: string) {
  expanded.value[id] = !expanded.value[id]
}

function goToWrite(node: any) {
  if (node.node_type === 'Scene') {
    router.push('/project/p1/write/' + node.id)
  } else if (node.children?.length) {
    const firstScene = findFirstScene(node)
    if (firstScene) router.push('/project/p1/write/' + firstScene.id)
  }
}

function findFirstScene(node: any): any {
  if (node.node_type === 'Scene') return node
  for (const child of node.children || []) {
    const found = findFirstScene(child)
    if (found) return found
  }
  return null
}
</script>

<style scoped>
.story-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-6); }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }
.btn-primary { padding: var(--space-2) var(--space-4); background: var(--color-primary); border: none; color: white; border-radius: var(--radius-sm); font-size: var(--text-sm); cursor: pointer; }
.btn-primary:hover { background: var(--color-primary-hover); }

.story-tree { display: flex; flex-direction: column; gap: var(--space-4); }

.tree-volume { border: 1px solid var(--border-default); border-radius: var(--radius-md); background: var(--bg-panel); overflow: hidden; }
.volume-header { display: flex; align-items: center; gap: var(--space-3); padding: var(--space-4) var(--space-5); cursor: pointer; transition: background var(--transition-fast); }
.volume-header:hover { background: var(--bg-hover); }
.volume-title { font-size: var(--text-lg); font-weight: 600; }
.volume-meta { font-size: var(--text-sm); color: var(--text-secondary); margin-left: auto; }

.expand-icon { font-size: var(--text-xs); color: var(--text-tertiary); width: 16px; }

.volume-body { border-top: 1px solid var(--border-muted); }

.tree-arc { border-bottom: 1px solid var(--border-muted); }
.tree-arc:last-child { border-bottom: none; }
.arc-header { display: flex; align-items: center; gap: var(--space-3); padding: var(--space-3) var(--space-5) var(--space-3) var(--space-8); cursor: pointer; transition: background var(--transition-fast); }
.arc-header:hover { background: var(--bg-hover); }
.arc-title { font-size: var(--text-md); font-weight: 500; }
.arc-meta { font-size: var(--text-sm); color: var(--text-secondary); margin-left: auto; }

.arc-body { }

.tree-chapter { border-bottom: 1px solid var(--border-muted); }
.tree-chapter:last-child { border-bottom: none; }
.chapter-header { display: flex; align-items: center; gap: var(--space-3); padding: var(--space-3) var(--space-5) var(--space-3) calc(var(--space-8) + var(--space-4)); cursor: pointer; transition: background var(--transition-fast); }
.chapter-header:hover { background: var(--bg-hover); }
.chapter-title { font-size: var(--text-sm); font-weight: 500; }
.chapter-meta { font-size: var(--text-xs); color: var(--text-tertiary); margin-left: auto; }

.write-btn {
  padding: var(--space-1) var(--space-3);
  border: 1px solid var(--color-primary);
  background: transparent;
  color: var(--color-primary-text);
  border-radius: var(--radius-sm);
  font-size: var(--text-xs);
  cursor: pointer;
  transition: all var(--transition-fast);
}
.write-btn:hover { background: var(--color-primary-subtle); }

.chapter-body { }

.tree-scene {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-5) var(--space-2) calc(var(--space-8) + var(--space-8));
  cursor: pointer;
  transition: background var(--transition-fast);
  font-size: var(--text-sm);
}
.tree-scene:hover { background: var(--bg-hover); }
.scene-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
.scene-dot.completed { background: var(--color-success); }
.scene-dot.inprogress { background: var(--color-accent); }
.scene-dot.planned { background: var(--text-tertiary); }
.scene-dot.draft { background: var(--color-warning); }
.scene-title { color: var(--text-secondary); }
.scene-time { margin-left: auto; font-size: var(--text-xs); color: var(--text-tertiary); }
</style>
