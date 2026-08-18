<template>
  <div class="writing-layout">
    <!-- Left: Story Tree -->
    <aside class="writing-sidebar" :style="{ width: leftWidth + 'px' }">
      <div class="sidebar-header">
        <span class="sidebar-title">故事结构</span>
        <button class="icon-btn" @click="$router.back()" title="返回项目">
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <path d="M10 7H4M4 7L7 4M4 7L7 10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>
      </div>
      <div class="sidebar-content">
        <template v-for="node in storyStore.tree" :key="node.id">
          <div class="tree-volume">
            <div class="tree-label volume-label">{{ node.title }}</div>
            <template v-for="arc in node.children" :key="arc.id">
              <div class="tree-arc">
                <div class="tree-label arc-label">{{ arc.title }}</div>
                <template v-for="chapter in arc.children" :key="chapter.id">
                  <div class="tree-chapter">
                    <div class="tree-label chapter-label">{{ chapter.title }}</div>
                    <template v-for="scene in chapter.children" :key="scene.id">
                      <div
                        class="tree-scene"
                        :class="{ active: scene.id === editorStore.currentSceneId }"
                        @click="loadScene(scene.id)"
                      >
                        <span class="status-dot" :class="scene.status.toLowerCase()"></span>
                        <span class="scene-title">{{ scene.title }}</span>
                      </div>
                    </template>
                  </div>
                </template>
              </div>
            </template>
          </div>
        </template>
      </div>
    </aside>

    <!-- Resize handle -->
    <div class="resize-handle" @mousedown="startResizeLeft"></div>

    <!-- Center: Editor -->
    <main class="writing-editor">
      <div class="editor-header" v-if="editorStore.currentSceneId">
        <div class="editor-title">
          <span class="scene-name">{{ currentSceneTitle }}</span>
          <span class="save-status" :class="{ dirty: editorStore.isDirty }">
            {{ editorStore.isDirty ? '未保存' : '已保存' }}
          </span>
        </div>
        <div class="editor-actions">
          <button class="action-btn" @click="editorStore.saveContent()">
            保存
          </button>
          <button class="action-btn primary" @click="startGeneration">
            AI 生成
          </button>
        </div>
      </div>
      <div class="editor-body" v-if="editorStore.currentSceneId">
        <StructuredEditor
          :model-value="editorStore.content"
          @update:model-value="editorStore.updateContent"
        />
      </div>
      <div class="editor-empty" v-else>
        <div class="empty-icon">✍️</div>
        <div class="empty-title">选择一个场景开始写作</div>
        <div class="empty-desc">从左侧故事结构中选择一个场景</div>
      </div>
      <div class="editor-footer" v-if="editorStore.currentSceneId">
        <span class="footer-item">字数：{{ editorStore.wordCount }}</span>
        <span class="footer-item">字符：{{ editorStore.charCount }}</span>
      </div>
    </main>

    <!-- Resize handle -->
    <div class="resize-handle" @mousedown="startResizeRight"></div>

    <!-- Selection Actions -->
    <SelectionActions @action="handleAiAction" />

    <!-- Right: Context Panel -->
    <aside class="writing-context" :style="{ width: rightWidth + 'px' }">
      <div class="context-tabs">
        <button
          v-for="tab in contextTabs"
          :key="tab.id"
          class="context-tab"
          :class="{ active: activeContextTab === tab.id }"
          @click="activeContextTab = tab.id"
        >
          {{ tab.label }}
        </button>
      </div>

      <!-- Context Entities Tab -->
      <div class="context-content" v-if="activeContextTab === 'context'">
        <div class="context-header">
          <span class="context-title">上下文</span>
          <span class="context-tokens">{{ contextStore.totalTokens.toLocaleString() }} tokens</span>
        </div>

        <!-- Context Entities -->
        <div class="context-subtitle">实体 (Entities)</div>
        <div class="context-list">
          <div
            v-for="entity in contextStore.entities"
            :key="entity.entity_id"
            class="context-entity"
            :class="{ pinned: entity.policy === 'Pinned', excluded: entity.policy === 'Excluded' }"
          >
            <div class="entity-header">
              <span class="entity-type-badge">{{ entity.entity_type }}</span>
              <span class="entity-name">{{ entity.entity_name }}</span>
              <span class="entity-relevance">{{ Math.round(entity.relevance * 100) }}%</span>
            </div>
            <div class="entity-reasons">
              <div v-for="reason in entity.reasons" :key="reason" class="reason-item">
                ✓ {{ reason }}
              </div>
            </div>
            <div class="entity-actions">
              <button
                class="ctx-btn"
                :class="{ active: entity.policy === 'Pinned' }"
                @click="contextStore.togglePin(entity.entity_id)"
                title="钉住"
              >
                📌
              </button>
              <button
                class="ctx-btn"
                :class="{ active: entity.policy === 'Excluded' }"
                @click="contextStore.toggleExclude(entity.entity_id)"
                title="排除"
              >
                🚫
              </button>
            </div>
          </div>
        </div>

        <div class="context-items">
          <div class="context-subtitle">上下文项目</div>
          <div v-for="item in contextStore.items" :key="item.id" class="context-item">
            <span class="item-type">{{ item.type }}</span>
            <span class="item-content">{{ item.content }}</span>
          </div>
        </div>

        <!-- Story State -->
        <div class="context-subtitle">故事状态 (Story State)</div>
        <div class="context-meta-list">
          <div class="meta-item"><span class="meta-label">当前卷</span><span class="meta-value">第一卷：黑石城</span></div>
          <div class="meta-item"><span class="meta-label">当前弧线</span><span class="meta-value">王家追杀</span></div>
          <div class="meta-item"><span class="meta-label">当前章节</span><span class="meta-value">第四章：地下遗迹</span></div>
          <div class="meta-item"><span class="meta-label">进度</span><span class="meta-value">场景2/5</span></div>
        </div>

        <!-- Selection Reasons -->
        <div class="context-subtitle">选择原因 (Selection Reasons)</div>
        <div class="reason-list">
          <div class="reason-item">林凡：当前Scene主角，最近3个Scene出现</div>
          <div class="reason-item">苏晚晴：当前Scene参与者，与主角关系密切</div>
          <div class="reason-item">地下遗迹：当前Scene地点，核心剧情地点</div>
          <div class="reason-item">王家：当前剧情线相关势力，追杀主角</div>
        </div>
      </div>

      <!-- Knowledge Tab -->
      <div class="context-content" v-if="activeContextTab === 'knowledge'">
        <KnowledgePanel character-name="林凡" />
      </div>

      <!-- Constraint Tab -->
      <div class="context-content" v-if="activeContextTab === 'constraint'">
        <ConstraintPanel />
      </div>

      <!-- Events Tab -->
      <div class="context-content" v-if="activeContextTab === 'events'">
        <EventLog />
      </div>

      <!-- Generation Tab -->
      <div class="context-content" v-if="activeContextTab === 'generation'">
        <div class="gen-header">
          <span class="context-title">AI 生成</span>
        </div>
        <div class="gen-task" v-if="generationStore.currentTask">
          <div class="gen-task-header">
            <span class="task-type">{{ generationStore.currentTask.type }}</span>
            <span class="task-status" :class="generationStore.currentTask.status.toLowerCase()">
              {{ statusLabels[generationStore.currentTask.status] || generationStore.currentTask.status }}
            </span>
          </div>
          <div class="gen-progress">
            <div class="progress-step" :class="{ done: isStageDone('BuildingContext'), active: generationStore.currentTask.status === 'BuildingContext' }">
              <span class="step-dot"></span>
              <span>构建 Context</span>
            </div>
            <div class="progress-step" :class="{ done: isStageDone('Generating'), active: generationStore.currentTask.status === 'Generating' }">
              <span class="step-dot"></span>
              <span>生成内容</span>
            </div>
            <div class="progress-step" :class="{ done: isStageDone('Validating'), active: generationStore.currentTask.status === 'Validating' }">
              <span class="step-dot"></span>
              <span>验证</span>
            </div>
            <div class="progress-step" :class="{ done: generationStore.currentTask.status === 'Completed', active: generationStore.currentTask.status === 'Completed' }">
              <span class="step-dot"></span>
              <span>完成</span>
            </div>
          </div>
          <div class="gen-result" v-if="generationStore.currentTask.result">
            <div class="result-label">结果</div>
            <div class="result-text">{{ generationStore.currentTask.result }}</div>
          </div>
        </div>
        <div class="gen-history">
          <div class="context-subtitle">历史任务</div>
          <div v-for="task in generationStore.tasks" :key="task.id" class="gen-history-item">
            <span class="history-type">{{ task.type }}</span>
            <span class="history-status" :class="task.status.toLowerCase()">{{ statusLabels[task.status] || task.status }}</span>
          </div>
        </div>
      </div>

      <!-- Story Info Tab -->
      <div class="context-content" v-if="activeContextTab === 'info'">
        <div class="info-section">
          <div class="info-title">当前场景</div>
          <div class="info-grid" v-if="currentScene">
            <div class="info-row">
              <span class="info-label">状态</span>
              <span class="info-value">{{ currentScene.status }}</span>
            </div>
            <div class="info-row" v-if="currentScene.attributes?.time">
              <span class="info-label">时间</span>
              <span class="info-value">{{ currentScene.attributes.time }}</span>
            </div>
            <div class="info-row" v-if="currentScene.attributes?.objective">
              <span class="info-label">目标</span>
              <span class="info-value">{{ currentScene.attributes.objective }}</span>
            </div>
            <div class="info-row" v-if="currentScene.attributes?.conflict">
              <span class="info-label">冲突</span>
              <span class="info-value">{{ currentScene.attributes.conflict }}</span>
            </div>
          </div>
        </div>
        <div class="info-section">
          <div class="info-title">剧情线</div>
          <div v-for="sl in storyStore.storylines" :key="sl.id" class="storyline-item">
            <span class="sl-status" :class="sl.status.toLowerCase()"></span>
            <span class="sl-name">{{ sl.name }}</span>
            <span class="sl-importance">{{ sl.importance }}</span>
          </div>
        </div>
        <div class="info-section">
          <div class="info-title">伏笔</div>
          <div v-for="fs in storyStore.foreshadows" :key="fs.id" class="foreshadow-item">
            <span class="fs-status" :class="fs.status.toLowerCase()">{{ fs.status }}</span>
            <span class="fs-name">{{ fs.name }}</span>
          </div>
        </div>
      </div>
    </aside>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useStoryStore } from '@/stores/story'
import { useEditorStore } from '@/stores/editor'
import { useContextStore } from '@/stores/context'
import { useGenerationStore } from '@/stores/generation'

const route = useRoute()
const storyStore = useStoryStore()
const editorStore = useEditorStore()
const contextStore = useContextStore()
const generationStore = useGenerationStore()

const leftWidth = ref(220)
const rightWidth = ref(320)
const activeContextTab = ref('context')

const contextTabs = [
  { id: 'context', label: '上下文' },
  { id: 'knowledge', label: '知识' },
  { id: 'constraint', label: '约束' },
  { id: 'generation', label: 'AI' },
  { id: 'events', label: '事件' },
  { id: 'info', label: '信息' },
]

const statusLabels: Record<string, string> = {
  Pending: '等待中',
  BuildingContext: '构建上下文',
  Generating: '生成中',
  Validating: '验证中',
  Completed: '已完成',
  Failed: '失败',
}

const currentScene = computed(() =>
  storyStore.nodes.find(n => n.id === editorStore.currentSceneId)
)

const currentSceneTitle = computed(() =>
  currentScene.value?.title || '未选择场景'
)

// Load data and scene
storyStore.loadMockData()
contextStore.loadMockContext()
generationStore.loadMockData()

const sceneId = route.params.sceneId as string
if (sceneId) {
  editorStore.loadScene(sceneId)
}

function loadScene(id: string) {
  editorStore.loadScene(id)
}

function handleAiAction(payload: any) {
  generationStore.startGeneration(payload.action === "rewrite" ? "RewriteSelection" : "GenerateScene")
  activeContextTab.value = "generation"
}

function startGeneration() {
  generationStore.startGeneration('GenerateScene', editorStore.currentSceneId || undefined)
  activeContextTab.value = 'generation'
}

function isStageDone(stage: string): boolean {
  const stages = ['BuildingContext', 'Generating', 'Validating', 'Completed']
  const currentIndex = stages.indexOf(generationStore.currentTask?.status || '')
  const stageIndex = stages.indexOf(stage)
  return currentIndex > stageIndex
}

// Resize handlers
function startResizeLeft(e: MouseEvent) {
  const startX = e.clientX
  const startWidth = leftWidth.value
  const onMove = (ev: MouseEvent) => {
    leftWidth.value = Math.max(160, Math.min(400, startWidth + ev.clientX - startX))
  }
  const onUp = () => {
    document.removeEventListener('mousemove', onMove)
    document.removeEventListener('mouseup', onUp)
  }
  document.addEventListener('mousemove', onMove)
  document.addEventListener('mouseup', onUp)
}

function startResizeRight(e: MouseEvent) {
  const startX = e.clientX
  const startWidth = rightWidth.value
  const onMove = (ev: MouseEvent) => {
    rightWidth.value = Math.max(240, Math.min(500, startWidth - (ev.clientX - startX)))
  }
  const onUp = () => {
    document.removeEventListener('mousemove', onMove)
    document.removeEventListener('mouseup', onUp)
  }
  document.addEventListener('mousemove', onMove)
  document.addEventListener('mouseup', onUp)
}
</script>

<style scoped>
.writing-layout {
  display: flex;
  height: 100%;
  overflow: hidden;
}

.writing-sidebar {
  background: var(--bg-panel);
  border-right: 1px solid var(--border-default);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  overflow: hidden;
}

.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-3);
  border-bottom: 1px solid var(--border-muted);
}

.sidebar-title {
  font-size: var(--text-xs);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--text-tertiary);
}

.icon-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border: none;
  background: transparent;
  color: var(--text-tertiary);
  border-radius: var(--radius-sm);
  cursor: pointer;
}

.icon-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.sidebar-content {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-2);
}

.tree-volume { margin-bottom: var(--space-3); }
.volume-label { font-size: var(--text-sm); font-weight: 600; color: var(--text-primary); padding: var(--space-1) 0; }
.arc-label { font-size: var(--text-sm); color: var(--text-secondary); padding-left: var(--space-3); }
.chapter-label { font-size: var(--text-sm); color: var(--text-secondary); padding-left: var(--space-6); font-weight: 500; }

.tree-scene {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-1) var(--space-1) var(--space-1) calc(var(--space-6) + var(--space-4));
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: var(--text-xs);
  color: var(--text-secondary);
}

.tree-scene:hover { background: var(--bg-hover); color: var(--text-primary); }
.tree-scene.active { background: var(--color-primary-subtle); color: var(--color-primary-text); }

.status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
}
.status-dot.completed { background: var(--color-success); }
.status-dot.inprogress { background: var(--color-accent); }
.status-dot.planned { background: var(--text-tertiary); }
.status-dot.draft { background: var(--color-warning); }

.resize-handle {
  width: 3px;
  background: transparent;
  cursor: col-resize;
  flex-shrink: 0;
  transition: background var(--transition-fast);
}
.resize-handle:hover { background: var(--color-primary); }

.writing-editor {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  background: var(--bg-base);
}

.editor-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border-default);
  flex-shrink: 0;
}

.editor-title {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}

.scene-name {
  font-size: var(--text-md);
  font-weight: 500;
}

.save-status {
  font-size: var(--text-xs);
  color: var(--color-success);
}
.save-status.dirty { color: var(--color-warning); }

.editor-actions {
  display: flex;
  gap: var(--space-2);
}

.action-btn {
  padding: var(--space-1) var(--space-3);
  border: 1px solid var(--border-default);
  background: var(--bg-panel);
  color: var(--text-secondary);
  border-radius: var(--radius-sm);
  font-size: var(--text-sm);
  cursor: pointer;
  transition: all var(--transition-fast);
}
.action-btn:hover { background: var(--bg-hover); color: var(--text-primary); }
.action-btn.primary {
  background: var(--color-primary);
  border-color: var(--color-primary);
  color: white;
}
.action-btn.primary:hover { background: var(--color-primary-hover); }

.editor-body {
  flex: 1;
  overflow: hidden;
  min-height: 0;
}

.editor-textarea {
  width: 100%;
  height: 100%;
  padding: var(--space-8) var(--space-16);
  background: transparent;
  border: none;
  outline: none;
  color: var(--text-primary);
  font-family: var(--font-serif);
  font-size: var(--text-md);
  line-height: var(--leading-relaxed);
  resize: none;
}

.editor-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--space-3);
  color: var(--text-tertiary);
}
.empty-icon { font-size: 48px; }
.empty-title { font-size: var(--text-lg); }
.empty-desc { font-size: var(--text-sm); }

.editor-footer {
  display: flex;
  gap: var(--space-4);
  padding: var(--space-2) var(--space-4);
  border-top: 1px solid var(--border-muted);
  font-size: var(--text-xs);
  color: var(--text-tertiary);
}

.writing-context {
  background: var(--bg-panel);
  border-left: 1px solid var(--border-default);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  overflow: hidden;
}

.context-tabs {
  display: flex;
  border-bottom: 1px solid var(--border-muted);
  flex-shrink: 0;
}

.context-tab {
  flex: 1;
  padding: var(--space-2) var(--space-3);
  border: none;
  background: transparent;
  color: var(--text-tertiary);
  font-size: var(--text-xs);
  cursor: pointer;
  transition: all var(--transition-fast);
  border-bottom: 2px solid transparent;
}
.context-tab:hover { color: var(--text-secondary); }
.context-tab.active { color: var(--text-primary); border-bottom-color: var(--color-primary); }

.context-content {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-3);
}

.context-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--space-3);
}
.context-title { font-size: var(--text-sm); font-weight: 600; }
.context-tokens { font-size: var(--text-xs); color: var(--text-tertiary); background: var(--bg-panel-secondary); padding: 2px 8px; border-radius: 10px; }

.context-entity {
  padding: var(--space-3);
  border: 1px solid var(--border-muted);
  border-radius: var(--radius-md);
  margin-bottom: var(--space-2);
  transition: all var(--transition-fast);
}
.context-entity.pinned { border-color: var(--color-accent); background: var(--color-accent-subtle); }
.context-entity.excluded { border-color: var(--color-error); opacity: 0.5; }

.entity-header {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin-bottom: var(--space-2);
}
.entity-type-badge {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 3px;
  background: var(--bg-panel-secondary);
  color: var(--text-tertiary);
}
.entity-name { font-size: var(--text-sm); font-weight: 500; }
.entity-relevance { margin-left: auto; font-size: var(--text-xs); color: var(--text-tertiary); }

.entity-reasons {
  margin-bottom: var(--space-2);
}
.reason-item {
  font-size: var(--text-xs);
  color: var(--text-secondary);
  padding: 1px 0;
}

.entity-actions {
  display: flex;
  gap: var(--space-1);
}
.ctx-btn {
  padding: 2px 6px;
  border: 1px solid var(--border-muted);
  background: transparent;
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: var(--text-xs);
  transition: all var(--transition-fast);
}
.ctx-btn:hover { background: var(--bg-hover); }
.ctx-btn.active { background: var(--bg-active); border-color: var(--border-emphasis); }

.context-subtitle {
  font-size: var(--text-xs);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-tertiary);
  margin: var(--space-4) 0 var(--space-2);
}

.context-item {
  display: flex;
  gap: var(--space-2);
  padding: var(--space-1) 0;
  font-size: var(--text-xs);
}
.item-type {
  color: var(--text-tertiary);
  text-transform: uppercase;
  min-width: 60px;
}
.item-content { color: var(--text-secondary); }

/* Generation tab */
.gen-header { margin-bottom: var(--space-3); }
.gen-task {
  border: 1px solid var(--border-muted);
  border-radius: var(--radius-md);
  padding: var(--space-3);
  margin-bottom: var(--space-3);
}
.gen-task-header {
  display: flex;
  justify-content: space-between;
  margin-bottom: var(--space-3);
}
.task-type { font-size: var(--text-sm); font-weight: 500; }
.task-status { font-size: var(--text-xs); padding: 2px 8px; border-radius: 10px; }
.task-status.buildingcontext { background: var(--color-info-subtle); color: var(--color-info); }
.task-status.generating { background: var(--color-warning-subtle); color: var(--color-warning); }
.task-status.validating { background: var(--color-accent-subtle); color: var(--color-accent); }
.task-status.completed { background: var(--color-success-subtle); color: var(--color-success); }

.gen-progress {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.progress-step {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--text-xs);
  color: var(--text-tertiary);
}
.progress-step.done { color: var(--color-success); }
.progress-step.active { color: var(--text-primary); }
.step-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  border: 1.5px solid var(--text-tertiary);
  flex-shrink: 0;
}
.progress-step.done .step-dot { background: var(--color-success); border-color: var(--color-success); }
.progress-step.active .step-dot { border-color: var(--color-accent); animation: pulse 1.5s infinite; }

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

.gen-result {
  margin-top: var(--space-3);
  padding-top: var(--space-3);
  border-top: 1px solid var(--border-muted);
}
.result-label { font-size: var(--text-xs); color: var(--text-tertiary); margin-bottom: var(--space-1); }
.result-text { font-size: var(--text-sm); color: var(--text-secondary); }

.gen-history-item {
  display: flex;
  justify-content: space-between;
  padding: var(--space-2) 0;
  font-size: var(--text-xs);
  border-bottom: 1px solid var(--border-muted);
}
.history-type { color: var(--text-secondary); }
.history-status { padding: 1px 6px; border-radius: 3px; }
.history-status.completed { background: var(--color-success-subtle); color: var(--color-success); }

/* Info tab */
.info-section { margin-bottom: var(--space-4); }
.info-title {
  font-size: var(--text-xs);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-tertiary);
  margin-bottom: var(--space-2);
}
.info-row {
  display: flex;
  justify-content: space-between;
  padding: var(--space-1) 0;
  font-size: var(--text-xs);
}
.info-label { color: var(--text-tertiary); }
.info-value { color: var(--text-secondary); }

.storyline-item {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-1) 0;
  font-size: var(--text-xs);
}
.sl-status {
  width: 6px;
  height: 6px;
  border-radius: 50%;
}
.sl-status.active { background: var(--color-success); }
.sl-status.planned { background: var(--text-tertiary); }
.sl-name { color: var(--text-secondary); }
.sl-importance { margin-left: auto; color: var(--text-tertiary); }

.foreshadow-item {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-1) 0;
  font-size: var(--text-xs);
}
.fs-status {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 3px;
}
.fs-status.introduced { background: var(--color-info-subtle); color: var(--color-info); }
.fs-status.active { background: var(--color-warning-subtle); color: var(--color-warning); }
.fs-status.planned { background: var(--bg-panel-secondary); color: var(--text-tertiary); }
.fs-name { color: var(--text-secondary); }
</style>