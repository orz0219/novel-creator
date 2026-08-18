<template>
  <div class="home-page">
    <!-- Hero Section -->
    <div class="home-hero">
      <div class="hero-content">
        <div class="hero-badge">Novel Engine</div>
        <h1 class="hero-title">小说创作工坊</h1>
        <p class="hero-desc">
          一个结构化、可验证、可追踪的小说世界运行引擎。<br/>
          AI 在你构建的世界中帮助你创作。
        </p>
        <div class="hero-actions">
          <button class="btn-primary" @click="createNewProject">创建新项目</button>
          <button class="btn-secondary">了解更多</button>
        </div>
      </div>
    </div>

    <!-- Recent Projects -->
    <div class="home-section">
      <h2 class="section-title">最近项目</h2>
      <div class="project-grid">
        <div
          v-for="project in mockProjects"
          :key="project.id"
          class="project-card"
          @click="$router.push('/project/' + project.id)"
        >
          <div class="project-card-header">
            <span class="project-status" :class="project.status.toLowerCase()">{{ statusLabels[project.status] }}</span>
          </div>
          <h3 class="project-name">{{ project.name }}</h3>
          <p class="project-desc">{{ project.description }}</p>
          <div class="project-stats">
            <span class="stat">👤 {{ project.characters }}</span>
            <span class="stat">📍 {{ project.locations }}</span>
            <span class="stat">📖 {{ project.chapters }} 章</span>
          </div>
          <div class="project-meta">
            <span>更新于 {{ project.updated }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Quick Actions -->
    <div class="home-section">
      <h2 class="section-title">快速开始</h2>
      <div class="action-grid">
        <div class="action-card" @click="$router.push('/project/p1/world/characters')">
          <span class="action-icon">👤</span>
          <span class="action-label">管理人物</span>
          <span class="action-desc">创建和编辑角色设定</span>
        </div>
        <div class="action-card" @click="$router.push('/project/p1/world/locations')">
          <span class="action-icon">📍</span>
          <span class="action-label">管理地点</span>
          <span class="action-desc">构建你的世界地图</span>
        </div>
        <div class="action-card" @click="$router.push('/project/p1/story')">
          <span class="action-icon">📖</span>
          <span class="action-label">故事规划</span>
          <span class="action-desc">规划卷、弧线和章节</span>
        </div>
        <div class="action-card" @click="$router.push('/project/p1/write/scene-1')">
          <span class="action-icon">✍️</span>
          <span class="action-label">开始写作</span>
          <span class="action-desc">进入创作工作台</span>
        </div>
        <div class="action-card" @click="$router.push('/project/p1/graph')">
          <span class="action-icon">🗺️</span>
          <span class="action-label">关系图谱</span>
          <span class="action-desc">可视化世界关系</span>
        </div>
        <div class="action-card" @click="$router.push('/project/p1/proposals')">
          <span class="action-icon">📋</span>
          <span class="action-label">AI 提案</span>
          <span class="action-desc">审查 AI 生成的内容</span>
        </div>
      </div>
    </div>

    <!-- Architecture Overview -->
    <div class="home-section">
      <h2 class="section-title">系统架构</h2>
      <div class="arch-flow">
        <div class="arch-step">
          <span class="arch-icon">🌍</span>
          <span class="arch-label">World</span>
          <span class="arch-desc">世界事实</span>
        </div>
        <span class="arch-arrow">→</span>
        <div class="arch-step">
          <span class="arch-icon">📖</span>
          <span class="arch-label">Story</span>
          <span class="arch-desc">叙事结构</span>
        </div>
        <span class="arch-arrow">→</span>
        <div class="arch-step">
          <span class="arch-icon">🧠</span>
          <span class="arch-label">Context</span>
          <span class="arch-desc">AI 上下文</span>
        </div>
        <span class="arch-arrow">→</span>
        <div class="arch-step">
          <span class="arch-icon">🤖</span>
          <span class="arch-label">Generation</span>
          <span class="arch-desc">AI 生成</span>
        </div>
        <span class="arch-arrow">→</span>
        <div class="arch-step">
          <span class="arch-icon">📋</span>
          <span class="arch-label">Proposal</span>
          <span class="arch-desc">AI 提案</span>
        </div>
        <span class="arch-arrow">→</span>
        <div class="arch-step">
          <span class="arch-icon">✅</span>
          <span class="arch-label">Validation</span>
          <span class="arch-desc">系统审查</span>
        </div>
        <span class="arch-arrow">→</span>
        <div class="arch-step">
          <span class="arch-icon">📝</span>
          <span class="arch-label">Commit</span>
          <span class="arch-desc">提交变更</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
const statusLabels: Record<string, string> = {
  Writing: '创作中',
  Planning: '规划中',
  Concept: '概念',
  Paused: '暂停',
  Completed: '已完成',
  Archived: '已归档',
}

const mockProjects = [
  {
    id: 'p1',
    name: '天玄大陆',
    description: '一部修仙题材长篇小说，讲述边境散修林凡在黑石城的冒险故事。',
    status: 'Writing',
    characters: 3,
    locations: 3,
    chapters: 5,
    updated: '3月12日',
  },
]

function createNewProject() {
  // TODO: open create project dialog
}
</script>

<style scoped>
.home-page {
  height: 100%;
  overflow-y: auto;
  padding: var(--space-8) var(--space-16);
}

.home-hero {
  text-align: center;
  padding: var(--space-16) 0;
  margin-bottom: var(--space-8);
}

.hero-badge {
  display: inline-block;
  font-size: var(--text-xs);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.1em;
  color: var(--color-primary-text);
  background: var(--color-primary-subtle);
  padding: var(--space-1) var(--space-4);
  border-radius: 20px;
  margin-bottom: var(--space-4);
}

.hero-title {
  font-size: 48px;
  font-weight: 700;
  font-family: var(--font-serif);
  margin-bottom: var(--space-4);
  background: linear-gradient(135deg, var(--text-primary) 0%, var(--color-primary-text) 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.hero-desc {
  font-size: var(--text-lg);
  color: var(--text-secondary);
  line-height: var(--leading-relaxed);
  max-width: 480px;
  margin: 0 auto var(--space-8);
}

.hero-actions {
  display: flex;
  gap: var(--space-3);
  justify-content: center;
}

.btn-primary {
  padding: var(--space-3) var(--space-6);
  background: var(--color-primary);
  border: none;
  color: white;
  border-radius: var(--radius-md);
  font-size: var(--text-md);
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition-fast);
}
.btn-primary:hover { background: var(--color-primary-hover); }

.btn-secondary {
  padding: var(--space-3) var(--space-6);
  background: transparent;
  border: 1px solid var(--border-default);
  color: var(--text-secondary);
  border-radius: var(--radius-md);
  font-size: var(--text-md);
  cursor: pointer;
  transition: all var(--transition-fast);
}
.btn-secondary:hover { border-color: var(--border-emphasis); color: var(--text-primary); }

.home-section {
  margin-bottom: var(--space-12);
}

.section-title {
  font-size: var(--text-xl);
  font-weight: 600;
  margin-bottom: var(--space-6);
}

.project-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: var(--space-4);
}

.project-card {
  padding: var(--space-5);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-lg);
  background: var(--bg-panel);
  cursor: pointer;
  transition: all var(--transition-fast);
}
.project-card:hover { border-color: var(--border-emphasis); transform: translateY(-2px); }

.project-card-header { margin-bottom: var(--space-3); }

.project-status {
  font-size: var(--text-xs);
  padding: 2px 8px;
  border-radius: 10px;
}
.project-status.writing { background: var(--color-success-subtle); color: var(--color-success); }
.project-status.planning { background: var(--color-info-subtle); color: var(--color-info); }

.project-name {
  font-size: var(--text-lg);
  font-weight: 600;
  margin-bottom: var(--space-2);
}

.project-desc {
  font-size: var(--text-sm);
  color: var(--text-secondary);
  margin-bottom: var(--space-4);
  line-height: var(--leading-relaxed);
}

.project-stats {
  display: flex;
  gap: var(--space-4);
  margin-bottom: var(--space-3);
}
.stat {
  font-size: var(--text-sm);
  color: var(--text-secondary);
}

.project-meta {
  font-size: var(--text-xs);
  color: var(--text-tertiary);
}

.action-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: var(--space-3);
}

.action-card {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  padding: var(--space-5);
  border: 1px solid var(--border-muted);
  border-radius: var(--radius-md);
  background: var(--bg-panel);
  cursor: pointer;
  transition: all var(--transition-fast);
}
.action-card:hover { border-color: var(--color-primary); background: var(--color-primary-subtle); }

.action-icon { font-size: 24px; }
.action-label { font-size: var(--text-md); font-weight: 500; }
.action-desc { font-size: var(--text-xs); color: var(--text-secondary); }

.arch-flow {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  flex-wrap: wrap;
  padding: var(--space-6);
  background: var(--bg-panel);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-lg);
}

.arch-step {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-1);
  padding: var(--space-3);
}
.arch-icon { font-size: 24px; }
.arch-label { font-size: var(--text-sm); font-weight: 600; }
.arch-desc { font-size: var(--text-xs); color: var(--text-tertiary); }
.arch-arrow { color: var(--text-tertiary); font-size: var(--text-lg); }
</style>
