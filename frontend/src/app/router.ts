import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      component: () => import('@/layouts/AppLayout.vue'),
      children: [
        { path: '', name: 'Home', component: () => import('@/pages/Home.vue') },
        { path: 'search', name: 'Search', component: () => import('@/pages/Search.vue') },
        { path: 'settings', name: 'Settings', component: () => import('@/pages/Settings.vue') },
        {
          path: 'project/:id',
          component: () => import('@/layouts/ProjectLayout.vue'),
          children: [
            { path: '', name: 'ProjectDashboard', component: () => import('@/pages/ProjectDashboard.vue') },
            { path: 'world', name: 'World', component: () => import('@/pages/World.vue') },
            { path: 'world/characters', name: 'Characters', component: () => import('@/pages/Characters.vue') },
            { path: 'world/locations', name: 'Locations', component: () => import('@/pages/Locations.vue') },
            { path: 'world/factions', name: 'Factions', component: () => import('@/pages/Factions.vue') },
            { path: 'world/items', name: 'Items', component: () => import('@/pages/Items.vue') },
            { path: 'world/rules', name: 'Rules', component: () => import('@/pages/Rules.vue') },
            { path: 'world/relationships', name: 'Relationships', component: () => import('@/pages/Relationships.vue') },
            { path: 'world/timeline', name: 'Timeline', component: () => import('@/pages/Timeline.vue') },
            { path: 'story', name: 'Story', component: () => import('@/pages/Story.vue') },
            { path: 'story/board', name: 'StoryBoard', component: () => import('@/pages/StoryBoard.vue') },
            { path: 'story/storylines', name: 'Storylines', component: () => import('@/pages/Storylines.vue') },
            { path: 'story/foreshadows', name: 'Foreshadows', component: () => import('@/pages/Foreshadows.vue') },
            { path: 'graph', name: 'Graph', component: () => import('@/pages/Graph.vue') },
            { path: 'proposals', name: 'Proposals', component: () => import('@/pages/Proposals.vue') },
            { path: 'extract', name: 'Extract', component: () => import('@/pages/Extract.vue') },
            { path: 'history', name: 'History', component: () => import('@/pages/History.vue') },
            { path: 'snapshots', name: 'Snapshots', component: () => import('@/pages/Snapshots.vue') },
          ],
        },
        { path: 'project/:id/write/:sceneId?', name: 'Writing', component: () => import('@/layouts/WritingLayout.vue') },
      ],
    },
  ],
})

export default router
