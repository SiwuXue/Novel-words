<template>
  <div class="word-tap-reader">
    <!-- 计数条 + 读完操作 -->
    <div class="wt-toolbar">
      <span class="wt-counts" role="status" :aria-label="t('wordTap.countsLabel')">
        <span class="wt-c wt-c-new">{{ t('wordTap.new') }} {{ counts.new }}</span>
        <span class="wt-c wt-c-unknown">{{ t('wordTap.unknown') }} {{ counts.unknown }}</span>
        <span class="wt-c wt-c-familiar">{{ t('wordTap.familiar') }} {{ counts.familiar }}</span>
        <span class="wt-c wt-c-mastered">{{ t('wordTap.mastered') }} {{ counts.mastered }}</span>
        <span class="wt-c wt-c-ignore">{{ t('wordTap.ignore') }} {{ counts.ignore }}</span>
      </span>
      <el-button size="small" :loading="finishing" @click="finishChapter">
        {{ t('wordTap.finish') }}
      </el-button>
    </div>

    <!-- 逐词正文：滚动懒渲染 -->
    <div ref="scrollRef" class="wt-scroll" @scroll="onScroll">
      <template v-for="{ block, index } in visibleBlocksWithIndex" :key="index">
        <component :is="block.tag" class="wt-block" :data-block="index">
          <template v-for="(token, ti) in block.tokens" :key="ti">
            <span
              v-if="token.type === 'word'"
              class="wt-word"
              :class="stateClass(states[token.key])"
              :data-key="token.key"
              @click="onWordClick($event, token)"
            >{{ token.text }}</span>
            <template v-else>{{ token.text }}</template>
          </template>
        </component>
      </template>
      <p v-if="visibleCount < blocks.length" class="wt-loading-more">{{ t('ui.loading') }}</p>
    </div>

    <!-- 查词弹窗（复用，含快捷标记） -->
    <DictLookupPopover
      v-if="popover.visible"
      :text="popover.text"
      :position="popover.position"
      :novel-id="novelId"
      :chapter-id="chapterId"
      quick-mark
      @close="popover.visible = false"
      @marked="onMarked"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'
import DictLookupPopover from './DictLookupPopover.vue'
import { useDictionaryStore } from '@/stores/dictionaryStore'
import {
  parseWordTapBlocks,
  collectWordKeys,
  stateClass,
  wordKey,
  type WordTapBlock,
  type WordToken,
} from '@/utils/wordTap'
import type { Proficiency } from '@/types/vocabWord'
import { t } from '@/i18n'

const props = defineProps<{
  content: string
  novelId: number | null
  chapterId: number | null
}>()

const emit = defineEmits<{
  (e: 'mark', payload: { word: string; proficiency: Proficiency }): void
}>()

/** 单次渲染的块数（滚动到底自动加载） */
const PAGE_SIZE = 60
const visibleCount = ref(PAGE_SIZE)
const scrollRef = ref<HTMLElement | null>(null)
const finishing = ref(false)
const disposed = ref(false)

const blocks = computed<WordTapBlock[]>(() => parseWordTapBlocks(props.content))
const visibleBlocks = computed(() => blocks.value.slice(0, visibleCount.value))
const visibleBlocksWithIndex = computed(() =>
  visibleBlocks.value.map((block, index) => ({ block, index })),
)

/** 词形归一化 key → 熟练度 */
const states = ref<Record<string, Proficiency>>({})
/** 唯一词 key → 首次出现的原文（批量标记时用原文更友好） */
const sampleWords = ref<Record<string, string>>({})

const popover = ref<{
  visible: boolean
  text: string
  position: { x: number; y: number }
}>({ visible: false, text: '', position: { x: 0, y: 0 } })

// ---------- 状态加载 ----------

async function loadStates() {
  const keys = collectWordKeys(blocks.value)
  const samples: Record<string, string> = {}
  for (const block of blocks.value) {
    for (const token of block.tokens) {
      if (token.type === 'word' && !samples[token.key]) samples[token.key] = token.text
    }
  }
  sampleWords.value = samples
  const fetched: Record<string, Proficiency> = {}
  for (let i = 0; i < keys.length; i += 300) {
    if (disposed.value) return
    try {
      const list = await invoke<
        Array<{ key: string; word: string; proficiency: string }>
      >('lookup_word_tap_states', { words: keys.slice(i, i + 300) })
      for (const item of list) {
        fetched[item.key] = item.proficiency as Proficiency
      }
    } catch (e) {
      console.error('[wordTap] load states failed:', e)
      return
    }
  }
  if (!disposed.value) states.value = fetched
}

watch(
  () => props.content,
  () => {
    visibleCount.value = PAGE_SIZE
    if (scrollRef.value) scrollRef.value.scrollTop = 0
    void loadStates()
  },
  { immediate: true },
)

// ---------- 懒渲染 ----------

function onScroll() {
  const el = scrollRef.value
  if (!el) return
  if (el.scrollTop + el.clientHeight >= el.scrollHeight - 120) {
    if (visibleCount.value < blocks.value.length) {
      visibleCount.value = Math.min(visibleCount.value + PAGE_SIZE, blocks.value.length)
    }
  }
}

// ---------- 点击查词 ----------

function onWordClick(e: MouseEvent, token: WordToken) {
  const target = e.currentTarget as HTMLElement
  const rect = target.getBoundingClientRect()
  popover.value = {
    visible: true,
    text: token.text,
    position: { x: rect.left, y: rect.bottom + 6 },
  }
  void useDictionaryStore().lookupAuto(token.text)
}

// ---------- 快捷标记 ----------

function onMarked(payload: { word: string; proficiency: Proficiency }) {
  const key = wordKey(payload.word)
  states.value = { ...states.value, [key]: payload.proficiency }
  emit('mark', payload)
}

// ---------- 计数 ----------

const counts = computed(() => {
  const result = { new: 0, unknown: 0, familiar: 0, mastered: 0, ignore: 0 }
  for (const key of collectWordKeys(blocks.value)) {
    const st = states.value[key]
    if (!st) result.new += 1
    else if (st in result) result[st as keyof typeof result] += 1
  }
  return result
})

// ---------- 读完本章 ----------

async function finishChapter() {
  const remaining: string[] = []
  for (const key of collectWordKeys(blocks.value)) {
    const st = states.value[key]
    if (!st || st === 'unknown') remaining.push(sampleWords.value[key] ?? key)
  }
  if (remaining.length === 0) {
    ElMessage.info(t('wordTap.nothingToIgnore'))
    return
  }
  try {
    await ElMessageBox.confirm(
      t('wordTap.finishConfirm', { n: remaining.length }),
      t('wordTap.finish'),
      { type: 'warning', confirmButtonText: t('wordTap.finishYes'), cancelButtonText: t('ui.cancel') },
    )
  } catch {
    return
  }
  finishing.value = true
  try {
    await invoke('mark_word_tap_proficiency', { words: remaining, proficiency: 'ignore' })
    const next = { ...states.value }
    for (const key of collectWordKeys(blocks.value)) {
      if (!next[key] || next[key] === 'unknown') next[key] = 'ignore'
    }
    states.value = next
    ElMessage.success(t('wordTap.finishDone'))
  } catch (e: any) {
    ElMessage.error(String(e?.message || e))
  } finally {
    finishing.value = false
  }
}

onMounted(() => void loadStates())
onBeforeUnmount(() => {
  disposed.value = true
})
</script>

<style scoped>
.word-tap-reader {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.wt-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 6px 10px;
  border-bottom: 1px solid var(--border-color, #ebeef5);
  flex-wrap: wrap;
}
.wt-counts {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  font-size: 12px;
}
.wt-c::before {
  content: '';
  display: inline-block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  margin-right: 4px;
  vertical-align: middle;
}
.wt-c-new::before { background: var(--wt-new, #409eff); }
.wt-c-unknown::before { background: var(--wt-unknown, #e05252); }
.wt-c-familiar::before { background: var(--wt-familiar, #e6a23c); }
.wt-c-mastered::before { background: var(--wt-mastered, #67a35f); }
.wt-c-ignore::before { background: var(--wt-ignore, #b8bfc9); }

.wt-scroll {
  flex: 1;
  overflow-y: auto;
  padding: 16px clamp(16px, 6vw, 64px);
  user-select: none;
}
.wt-block {
  line-height: 2;
  margin: 0 0 12px;
  color: var(--text-primary, #303133);
  font-size: var(--reading-font-size, 17px);
}

.wt-word {
  cursor: pointer;
  border-radius: 3px;
  padding: 0 1px;
  transition: background-color 0.15s;
}
.wt-word:hover {
  background: var(--wt-hover, rgba(64, 158, 255, 0.18));
}

/* 状态着色（背景微高亮 + 文本色） */
.wt-st-new {
  background: color-mix(in srgb, var(--wt-new, #409eff) 22%, transparent);
}
.wt-st-unknown {
  background: color-mix(in srgb, var(--wt-unknown, #e05252) 22%, transparent);
  color: var(--wt-unknown, #c45656);
  font-weight: 600;
}
.wt-st-familiar {
  background: color-mix(in srgb, var(--wt-familiar, #e6a23c) 20%, transparent);
  color: var(--wt-familiar, #b88230);
}
.wt-st-mastered {
  color: var(--wt-mastered-text, #6b7280);
}
.wt-st-ignore {
  color: var(--wt-ignore, #b8bfc9);
}

.wt-loading-more {
  text-align: center;
  color: var(--text-secondary, #909399);
  font-size: 12px;
}
</style>
