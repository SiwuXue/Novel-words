import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      name: 'Home',
      component: () => import('@/views/HomePage.vue'),
    },
    {
      path: '/novels',
      name: 'NovelList',
      component: () => import('@/views/NovelListPage.vue'),
    },
    {
      path: '/novels/new',
      name: 'NovelCreate',
      component: () => import('@/views/NovelEditorPage.vue'),
    },
    {
      path: '/novels/:id',
      name: 'NovelEdit',
      component: () => import('@/views/NovelEditorPage.vue'),
    },
    {
      path: '/vocabulary',
      name: 'VocabBookList',
      component: () => import('@/views/VocabBookListPage.vue'),
    },
    {
      path: '/vocabulary/:id',
      name: 'VocabBookDetail',
      component: () => import('@/views/VocabBookDetailPage.vue'),
    },
    {
      path: '/settings',
      name: 'Settings',
      component: () => import('@/views/SettingsPage.vue'),
    },
  ],
})

export default router
