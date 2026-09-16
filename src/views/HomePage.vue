<template>
  <div class="home-page">
    <PageHeader :title="t('ui.today')" :description="t('ui.homeHint')">
      <el-button type="primary" @click="importNovel"><el-icon><FolderOpened /></el-icon>{{ t('home.importNovel') }}</el-button>
    </PageHeader>
    <PageState v-if="loading" :title="t('ui.loading')" loading />
    <template v-else>
      <PageState v-if="error" :title="t('ui.loadFailed')" :description="t('ui.homeErrorHint')" error @retry="load" />
      <div class="home-workspace">
        <section class="recent-section">
          <h3>{{ t('home.continueReading') }}</h3>
          <div v-if="recentNovels.length" class="recent-list">
            <RouterLink v-for="n in recentNovels" :key="n.id" :to="novelReadingLocation(n.id)" class="recent-item">
              <div class="book-mark"><el-icon><Reading /></el-icon></div>
              <div class="recent-copy"><strong>{{ n.title || t('home.unnamed') }}</strong><span>{{ n.author || t('home.unknownAuthor') }}</span><el-progress :percentage="readingProgress[n.id] || 0" :stroke-width="4" /></div>
              <el-icon><ArrowRight /></el-icon>
            </RouterLink>
          </div>
          <PageState v-else-if="!error" :title="t('ui.noNovels')" :description="t('ui.noNovelsHint')">
            <el-button type="primary" @click="importNovel">{{ t('home.importNovel') }}</el-button>
          </PageState>
        </section>
        <section class="daily-review">
          <span class="section-eyebrow">{{ t('home.statDue') }}</span><strong class="due-number">{{ dueCount ?? '—' }}</strong>
          <p>{{ dueCount == null ? t('ui.loadFailed') : dueCount ? t('ui.dueHint', { n:dueCount }) : t('ui.noDue') }}</p>
          <el-button :disabled="dueCount == null" :type="dueCount ? 'primary' : 'default'" @click="router.push('/review')">{{ t('ui.startReview') }}<el-icon><ArrowRight /></el-icon></el-button>
        </section>
      </div>
      <div class="home-summary">
        <RouterLink to="/novels"><strong>{{ novelCount ?? '—' }}</strong><span>{{ t('home.statNovels') }}</span></RouterLink>
        <RouterLink to="/vocabulary"><strong>{{ bookCount ?? '—' }}</strong><span>{{ t('home.statBooks') }}</span></RouterLink>
        <RouterLink to="/vocabulary"><strong>{{ wordCount ?? '—' }}</strong><span>{{ t('home.statWords') }}</span></RouterLink>
      </div>
    </template>
  </div>
</template>
<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { FolderOpened, Reading, ArrowRight } from '@element-plus/icons-vue'
import { invoke } from '@tauri-apps/api/core'
import PageHeader from '@/components/common/PageHeader.vue'
import PageState from '@/components/common/PageState.vue'
import { novelReadingLocation, readingProgressForChapters } from '@/utils/workspace'
import { t } from '@/i18n'
import type { VocabBook } from '@/types/vocabBook'
import type { VocabWord } from '@/types/vocabWord'
import type { Novel, ChapterSummary } from '@/types/novel'
const router = useRouter()
const novelCount = ref<number | null>(null), bookCount = ref<number | null>(null), wordCount = ref<number | null>(null), dueCount = ref<number | null>(null)
const recentNovels = ref<Novel[]>([]), readingProgress = ref<Record<number, number>>({})
const loading = ref(true), error = ref(false)
function importNovel() { void router.push({ path:'/novels', query:{ import:'1' } }) }
async function load() {
  loading.value = true; error.value = false
  novelCount.value = bookCount.value = wordCount.value = dueCount.value = null
  const results = await Promise.allSettled([
    invoke<Novel[]>('get_all_novels').then(async novels => {
      novelCount.value = novels.length; recentNovels.value = novels.slice(0,3)
      const entries = await Promise.all(recentNovels.value.map(async novel => {
        try {
          const raw = await invoke<string>('get_setting', { key:`reading_pos_${novel.id}` })
          if (!raw) return [novel.id, 0]
          const parsed = JSON.parse(raw) as { percent?:number; chapterIndex?:number }
          const chapters = await invoke<ChapterSummary[]>('get_chapter_list', { novelId:novel.id })
          const progress = chapters.length ? readingProgressForChapters(chapters, parsed.chapterIndex || 0, Number(parsed.percent) || 0) : Math.min(1, Math.max(0, Number(parsed.percent) || 0))
          return [novel.id, Math.round(progress * 100)]
        }
        catch { return [novel.id, 0] }
      }))
      readingProgress.value = Object.fromEntries(entries)
    }),
    invoke<number>('get_due_words_count').then(count => { dueCount.value = count }),
    invoke<VocabBook[]>('get_all_vocab_books').then(async all => {
      const books = all.filter(b => !b.isPreset); bookCount.value = books.length
      const words = await Promise.all(books.map(b => invoke<VocabWord[]>('get_vocab_words', { vocabBookId:b.id })))
      wordCount.value = words.reduce((sum, list) => sum + list.length,0)
    }),
  ])
  error.value = results.some(result => result.status === 'rejected'); loading.value = false
}
onMounted(load)
</script>
<style scoped>
.home-page { width:100%; max-width:1120px; margin:0 auto; }
.home-workspace { display:grid; grid-template-columns:minmax(0,1fr) 240px; gap:20px; margin-top:24px; }
.recent-section { min-width:0; }h3 { font-size:16px; font-weight:600; margin-bottom:14px; }
.recent-list { border:1px solid var(--border-color); border-radius:8px; background:var(--bg-primary); overflow:hidden; }
.recent-item { display:flex; align-items:center; gap:16px; padding:20px; border-bottom:1px solid var(--border-color); }
.recent-item:last-child { border-bottom:0; }.recent-item:hover { background:var(--accent-light); }
.book-mark { display:grid; place-items:center; flex-shrink:0; width:44px; height:56px; border-radius:6px; background:var(--accent-light); color:var(--accent-color); font-size:22px; }
.recent-copy { flex:1; min-width:0; display:flex; flex-direction:column; gap:8px; }
.recent-copy strong { font-size:16px; overflow-wrap:anywhere; }.recent-copy span { font-size:13px; color:var(--text-secondary); }
.daily-review { display:flex; flex-direction:column; align-items:flex-start; gap:14px; padding:24px; border:1px solid var(--border-color); border-radius:8px; background:var(--bg-primary); }
.section-eyebrow { font-size:14px; color:var(--text-secondary); }.due-number { font-size:48px; line-height:1.1; font-weight:600; color:var(--accent-color); }
.daily-review p { font-size:14px; color:var(--text-secondary); line-height:1.7; }.daily-review .el-button { margin-top:auto; }
.home-summary { display:flex; flex-wrap:wrap; gap:32px; border-top:1px solid var(--border-color); margin-top:28px; padding-top:20px; }
.home-summary a { display:flex; align-items:baseline; gap:8px; }.home-summary strong { font-size:22px; font-weight:600; }.home-summary span { font-size:13px; color:var(--text-secondary); }
@media(max-width:900px) { .home-workspace { grid-template-columns:minmax(0,1fr) 200px; gap:16px; }.daily-review { padding:20px; } }
@media(max-width:600px) { .home-workspace { grid-template-columns:1fr; }.daily-review { gap:10px; }.due-number { font-size:36px; }.recent-item { padding:16px; }.home-summary { gap:20px; } }
</style>
