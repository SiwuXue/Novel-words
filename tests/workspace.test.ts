import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import ReadingPanels from '@/components/novel/ReadingPanels.vue'
import { navigationSection, novelReadingLocation, readingProgressForChapters } from '@/utils/workspace'

describe('workspace navigation', () => {
  it('keeps review separate from vocabulary and identifies nested pages', () => {
    expect(navigationSection('/vocabulary/3/review')).toBe('/review')
    expect(navigationSection('/vocabulary/3')).toBe('/vocabulary')
    expect(navigationSection('/novels/2')).toBe('/novels')
  })
  it('opens existing novels read-only without changing editor deep links', () => {
    expect(novelReadingLocation(42)).toBe('/novels/42?mode=read')
  })
})
describe('reading tools', () => {
  it('opens one panel at a time, preserves the reading content and closes with Escape', async () => {
    const wrapper = mount(ReadingPanels, { slots: { directory: '<p>Chapter one</p>', tools: '<p>Vocabulary</p>' } })
    await wrapper.get('[data-panel="directory"]').trigger('click')
    expect(wrapper.find('[role="dialog"]').text()).toContain('Chapter one')
    await wrapper.get('[data-panel="tools"]').trigger('click')
    expect(wrapper.findAll('[role="dialog"]')).toHaveLength(1)
    expect(wrapper.find('[role="dialog"]').text()).toContain('Vocabulary')
    expect(wrapper.find('[role="dialog"]').text()).not.toContain('Chapter one')
    await wrapper.get('[role="dialog"]').trigger('keydown', { key: 'Escape' })
    expect(wrapper.find('[role="dialog"]').exists()).toBe(false)
  })
  it('toggles the active panel closed', async () => {
    const wrapper = mount(ReadingPanels)
    await wrapper.get('[data-panel="tools"]').trigger('click')
    await wrapper.get('[data-panel="tools"]').trigger('click')
    expect(wrapper.find('[role="dialog"]').exists()).toBe(false)
  })
})

it('computes whole-book progress without changing the current chapter', () => {
  const chapters = [{contentLength:100},{contentLength:300}]
  expect(readingProgressForChapters(chapters,1,0.5)).toBe(0.625)
  expect(readingProgressForChapters([],0,0)).toBe(0)
  expect(readingProgressForChapters(chapters,99,2)).toBe(1)
})
