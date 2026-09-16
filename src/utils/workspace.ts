export function navigationSection(path: string): string {
  if (path === '/review' || /^\/vocabulary\/[^/]+\/review$/.test(path)) return '/review'
  for (const section of ['/novels', '/vocabulary', '/presets', '/stats', '/settings']) {
    if (path === section || path.startsWith(section + '/')) return section
  }
  return '/'
}
export function novelReadingLocation(id: number): string {
  return `/novels/${id}?mode=read`
}

/** Aggregate progress; persisted percentages remain local to each chapter. */
export function readingProgressForChapters(chapters: readonly { contentLength?: number; content?: string }[], index: number, percent: number): number {
  if (!chapters.length) return 0
  const current = Math.min(chapters.length - 1, Math.max(0, Math.floor(index) || 0))
  const lengths = chapters.map(ch => Math.max(1, ch.contentLength || ch.content?.length || 0))
  const total = lengths.reduce((sum, length) => sum + length, 0)
  const before = lengths.slice(0, current).reduce((sum, length) => sum + length, 0)
  return (before + lengths[current] * Math.min(1, Math.max(0, percent || 0))) / total
}
