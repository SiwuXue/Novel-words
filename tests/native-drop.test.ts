import { expect, it, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import NovelListPage from '@/views/NovelListPage.vue'
const state = vi.hoisted(() => ({ handler:null as any, unlisten:vi.fn() }))
vi.mock('@tauri-apps/api/core', () => ({ invoke:vi.fn().mockResolvedValue([]) }))
vi.mock('@tauri-apps/api/webview', () => ({ getCurrentWebview:() => ({ onDragDropEvent:vi.fn(async handler => { state.handler = handler; return state.unlisten }) }) }))
it('imports a native Tauri dropped path and disposes the listener', async () => {
  const router = createRouter({ history:createMemoryHistory(), routes:[{path:'/novels',component:NovelListPage}] })
  await router.push('/novels'); await router.isReady()
  const ImportDialog = { props:['initialPath'], template:'<div class="test-import">{{ initialPath }}</div>' }
  const wrapper = mount(NovelListPage, { global:{ plugins:[createPinia(),router], directives:{loading:() => {}}, stubs:{ NovelFormDialog:true, ImportDialog, 'el-icon':true, 'el-button':true, 'el-input':true, 'el-tag':true, 'el-dropdown':true, 'el-dropdown-menu':true, 'el-dropdown-item':true, 'el-table':true, 'el-table-column':true } } })
  await flushPromises()
  expect(state.handler).toBeTypeOf('function')
  state.handler({payload:{type:'drop',paths:['C:/books/garden.txt']}})
  await flushPromises()
  expect(wrapper.findComponent(ImportDialog).props('initialPath')).toBe('C:/books/garden.txt')
  wrapper.unmount(); expect(state.unlisten).toHaveBeenCalled()
})
