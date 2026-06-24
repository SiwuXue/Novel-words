import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      name: 'Home',
      component: () => import('@/views/HomePage.vue'),
      meta: { title: '词阅 PDF 工坊' },
    },
    {
      path: '/novels',
      name: 'NovelList',
      component: () => import('@/views/NovelListPage.vue'),
      meta: { title: '小说库' },
    },
    {
      path: '/novels/new',
      name: 'NovelCreate',
      component: () => import('@/views/NovelEditorPage.vue'),
      meta: { title: '导入小说' },
    },
    {
      path: '/novels/:id',
      name: 'NovelEdit',
      component: () => import('@/views/NovelEditorPage.vue'),
      meta: { title: '编辑小说' },
    },
    {
      path: '/vocabulary',
      name: 'VocabBookList',
      component: () => import('@/views/VocabBookListPage.vue'),
      meta: { title: '词汇本' },
    },
    {
      path: '/vocabulary/:id',
      name: 'VocabBookDetail',
      component: () => import('@/views/VocabBookDetailPage.vue'),
      meta: { title: '词汇详情' },
    },
    {
      path: '/settings',
      name: 'Settings',
      component: () => import('@/views/SettingsPage.vue'),
      meta: { title: '设置' },
    },
    {
      path: '/:pathMatch(.*)*',
      name: 'NotFound',
      component: () => import('@/views/NotFoundPage.vue'),
      meta: { title: '404 - 页面不存在' },
    },
  ],
})

router.afterEach((to) => {
  document.title = (to.meta?.title as string) || '词阅 PDF 工坊'
})

export default router
