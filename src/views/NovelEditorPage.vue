<template>
  <div class="editor-page" :class="{ 'reading-mode': readingMode }" :style="readingStyle">
    <div class="editor-topbar" :data-tauri-drag-region="readingMode && !isMobile ? '' : undefined">
      <el-button link @click="goBack">
        <el-icon><ArrowLeft /></el-icon> {{ t('editor.back') }}
      </el-button>
      <span class="novel-title" :data-tauri-drag-region="readingMode && !isMobile ? '' : undefined">{{ topbarTitle }}</span>
      <span v-if="!readingMode" class="save-status">
        <el-icon v-if="editorStore.saving" class="is-loading"><Loading /></el-icon>
        <template v-else>{{ editorStore.isDirty ? t('editor.unsaved') : t('editor.saved') }}</template>
        <el-button
          v-if="loadState === 'loaded'"
          size="small"
          link
          :disabled="!editorStore.isDirty"
          @click="handleManualSave"
        >
          {{ t('editor.save') }}
        </el-button>
      </span>
      <template v-if="!readingMode">
        <el-select v-if="loadState === 'loaded'" :model-value="store.currentNovel?.language" size="small" style="width:110px" :aria-label="t('settings.language')" @change="onLanguageChange"><el-option value="zh" :label="t('editor.langZh')" /><el-option value="en" :label="t('editor.langEn')" /></el-select>
        <el-button v-if="loadState === 'loaded'" size="small" @click="openExportConfig"><el-icon><Printer /></el-icon>{{ t('editor.exportPdf') }}</el-button>
        <el-button v-if="loadState === 'loaded'" size="small" type="primary" plain @click="enterReadingMode"><el-icon><Reading /></el-icon>{{ t('reading.enter') }}</el-button>
      </template>
      <template v-else>
        <div class="reading-topbar-meta">
          <span class="reading-chapter-label">{{ currentChapterTitle }}</span>
          <span>{{ readingPercent }}%</span>
          <span>{{ readingRemainingLabel }}</span>
          <span class="reading-hotkey-hint">{{ t('reading.shortcutHint') }}</span>
        </div>
        <el-popover v-model:visible="readingSettingsOpen" placement="bottom-end" :width="320" trigger="click">
          <template #reference>
            <el-button size="small" circle :aria-label="t('reading.settings')" :title="t('reading.settings')">
              <el-icon><Setting /></el-icon>
            </el-button>
          </template>
          <div class="reading-settings-panel">
            <div class="reading-settings-title">{{ t('reading.settings') }}</div>
            <label class="reading-setting-row">
              <span>{{ t('reading.font') }}</span>
              <el-select v-model="readingFont" size="small" style="width: 180px">
                <el-option value="system" :label="t('reading.fontSystem')" />
                <el-option value="serif" :label="t('reading.fontSerif')" />
                <el-option value="mono" :label="t('reading.fontMono')" />
              </el-select>
            </label>
            <label class="reading-setting-row">
              <span>{{ t('reading.fontSize') }} {{ readingFontSize }}px</span>
              <el-slider v-model="readingFontSize" :min="15" :max="30" :step="1" style="width: 150px" />
            </label>
            <label class="reading-setting-row">
              <span>{{ t('reading.lineHeight') }} {{ readingLineHeight.toFixed(1) }}</span>
              <el-slider v-model="readingLineHeight" :min="1.4" :max="2.4" :step="0.1" style="width: 150px" />
            </label>
            <label class="reading-setting-row">
              <span>{{ t('reading.width') }}</span>
              <el-radio-group v-model="readingWidth" size="small">
                <el-radio-button :value="620">{{ t('ui.widthNarrow') }}</el-radio-button>
                <el-radio-button :value="760">{{ t('ui.widthMedium') }}</el-radio-button>
                <el-radio-button :value="920">{{ t('ui.widthWide') }}</el-radio-button>
              </el-radio-group>
            </label>
          </div>
        </el-popover>
        <ReadingPanels teleport>
          <template #directory="{ close }"><ChapterList :chapters="editorStore.chapterList" :active-index="editorStore.activeChapterIndex" @select="index => { scrollToChapter(index); close() }" /></template>
          <template #tools>
            <div class="reading-tools-form">
              <p>{{ t('ui.learningHint') }}</p>
              <label>{{ t('settings.language') }}</label><el-select :model-value="store.currentNovel?.language" @change="onLanguageChange"><el-option value="zh" :label="t('editor.langZh')" /><el-option value="en" :label="t('editor.langEn')" /></el-select>
              <label>{{ t('ui.selectBook') }}</label>
              <el-select v-model="highlightBookId" clearable :placeholder="t('ui.noHighlight')" :aria-label="t('ui.selectBook')"><el-option v-for="book in vocabBookStore.books" :key="book.id" :label="book.name" :value="book.id" /></el-select>
              <el-alert v-if="highlightLoadState === 'loading'" :title="t('ui.loading')" :closable="false" />
              <el-alert v-if="highlightLoadState === 'error'" :title="highlightLoadError" type="error" :closable="false" />
              <el-button @click="openExportConfig"><el-icon><Printer /></el-icon>{{ t('editor.exportPdf') }}</el-button>
            </div>
          </template>
        </ReadingPanels>
        <el-button size="small" @click="openExportConfig" :aria-label="t('editor.exportPdf')"><el-icon><Printer /></el-icon></el-button>
        <el-button
          v-if="isEnglishMode"
          size="small"
          :type="wordTapMode ? 'primary' : 'default'"
          @click="wordTapMode = !wordTapMode"
        >
          {{ t('wordTap.toggle') }}
        </el-button>
        <el-button
          v-if="!wordTapMode"
          size="small"
          :type="ttsState === 'playing' ? 'warning' : 'default'"
          @click="toggleTtsReading"
        >
          {{ ttsLabel }}
        </el-button>
        <el-button v-if="!wordTapMode && ttsState !== 'idle'" size="small" @click="stopTtsReading">
          {{ t('reading.ttsStop') }}
        </el-button>
        <span v-if="!wordTapMode && ttsState !== 'idle'" class="tts-progress-label" role="status">
          {{ ttsProgressLabel }}
        </span>
        <el-button v-if="!wordTapMode && store.currentNovel" size="small" @click="charPanel?.open()">
          {{ t('characters.open') }}
        </el-button>
        <el-button size="small" @click="exitReadingMode">
          {{ t('ui.editContent') }}
        </el-button>
      </template>
      <WindowControls v-if="readingMode" />
    </div>

    <!-- 角色音色面板（多角色对白分音色，普通阅读模式） -->
    <CharacterVoicePanel
      v-if="store.currentNovel"
      ref="charPanel"
      :novel-id="store.currentNovel.id"
      :chapter-text="plainChapterText"
      @updated="onCharVoicesUpdated"
    />

    <el-dialog v-model="exportConfigOpen" :title="t('ui.exportConfig')" width="520px" append-to-body :close-on-click-modal="!exportingPdf" :close-on-press-escape="!exportingPdf" :show-close="!exportingPdf">
      <p class="export-config-hint">{{ t('ui.exportHint') }}</p>
      <el-form label-position="top" :disabled="exportingPdf">
        <el-form-item :label="t('ui.selectBook')"><el-select v-model="highlightBookId" clearable :placeholder="t('ui.noHighlight')"><el-option v-for="book in vocabBookStore.books" :key="book.id" :label="book.name" :value="book.id" /></el-select></el-form-item>
        <el-form-item :label="t('editor.template', { name:pdfTemplateLabel })"><el-radio-group :model-value="pdfTemplateType" @change="(value: string | number | boolean | undefined) => onTemplateSelect(value as TemplateType)"><el-radio-button value="intensive">{{ t('editor.templateIntensive') }}</el-radio-button><el-radio-button value="card" :disabled="!isEnglishMode">{{ t('editor.templateCard') }}</el-radio-button></el-radio-group></el-form-item>
        <el-form-item v-if="pdfTemplateType === 'intensive'" :label="t('editor.exportSteps', { n:pdfSteps.length })"><el-checkbox-group v-model="pdfSteps" @change="onPdfStepsChange"><el-checkbox v-for="n in stepNums" :key="n" :value="n">{{ t('ui.pdfStep' + n) }}</el-checkbox></el-checkbox-group></el-form-item>
        <el-checkbox v-model="coverEnabled">{{ t('editor.cover') }}</el-checkbox>
        <el-checkbox v-if="pdfTemplateType === 'card'" v-model="pageNumbersEnabled">{{ t('editor.pageNumbers') }}</el-checkbox>
      </el-form>
      <div v-if="exportingPdf" role="status" aria-live="polite" class="export-config-progress"><el-progress :percentage="pdfPercent" /><p>{{ pdfMessage }}</p></div>
      <el-alert v-if="exportError" :title="exportError" type="error" :closable="false" show-icon />
      <el-alert v-if="exportResult" :title="t('ui.exportDone', { path:exportResult })" type="success" :closable="false" show-icon />
      <template #footer><el-button :disabled="exportingPdf" @click="exportConfigOpen = false">{{ t('ui.close') }}</el-button><el-button type="primary" :loading="exportingPdf" @click="handleExportPdf">{{ t(exportError ? 'ui.retry' : 'ui.exportStart') }}</el-button></template>
    </el-dialog>

    <!-- Loaded: three-column body with draggable splitters -->
    <div class="editor-body" :class="{ 'reading-body': readingMode }" v-if="loadState === 'loaded'">
      <div
        v-if="!readingMode"
        class="editor-pane left-pane"
        :style="{ width: split.state.leftWidth + 'px' }"
        v-show="split.state.leftWidth > 0"
      >
        <ChapterList
          :chapters="editorStore.chapterList"
          :active-index="editorStore.activeChapterIndex"
          @select="scrollToChapter"
        />
      </div>
      <div
        v-if="!readingMode"
        class="split-divider"
        :class="{ collapsed: split.state.leftWidth === 0 }"
        @mousedown="split.startLeftDrag"
        @dblclick="split.toggleLeft"
        title="拖拽调整宽度 · 双击折叠/恢复"
      >
        <el-icon v-if="split.state.leftWidth === 0" class="divider-icon">
          <component :is="DArrowRight" />
        </el-icon>
      </div>
      <div class="editor-pane center-pane">
        <!-- 英文逐词阅读模式：只读逐词渲染 -->
        <WordTapReader
          v-if="readingMode && wordTapMode"
          :content="editorContent"
          :novel-id="currentNovelId"
          :chapter-id="editorStore.chapterList[editorStore.activeChapterIndex]?.id ?? null"
          @tts-next="onWordTapTtsNext"
        />
        <NovelEditor
          v-else
          :key="`${editorStore.chapterList[editorStore.activeChapterIndex]?.id || 'draft'}-${editorStore.activeChapterIndex}`"
          ref="editorRef"
          :novel-id="currentNovelId"
          :chapter-id="editorStore.chapterList[editorStore.activeChapterIndex]?.id ?? null"
          :content="editorContent"
          :highlight-words="highlightWords"
          :highlight-book-id="highlightBookId"
          :read-only="readingMode"
          @update:content="onEditorContentChange"
          @update:highlight-book-id="highlightBookId = $event"
          @ready="handleEditorReady"
        />
      </div>
      <div
        v-if="!readingMode"
        class="split-divider"
        :class="{ collapsed: split.state.rightWidth === 0 }"
        @mousedown="split.startRightDrag"
        @dblclick="split.toggleRight"
        title="拖拽调整宽度 · 双击折叠/恢复"
      >
        <el-icon v-if="split.state.rightWidth === 0" class="divider-icon">
          <component :is="DArrowLeft" />
        </el-icon>
      </div>
      <div
        v-if="!readingMode"
        class="editor-pane right-pane"
        :style="{ width: split.state.rightWidth + 'px' }"
        v-show="split.state.rightWidth > 0"
      >
        <el-alert
          v-if="highlightLoadState === 'loading'"
          type="info"
          :closable="false"
          show-icon
          style="margin: 8px;"
          title="正在加载词汇本…"
        />
        <el-alert
          v-else-if="highlightLoadState === 'error'"
          type="error"
          :closable="false"
          show-icon
          style="margin: 8px;"
        >
          <template #title>
            词汇本加载失败：{{ highlightLoadError }}
          </template>
        </el-alert>
        <el-alert
          v-else-if="loadState === 'loaded' && highlightBookId && highlightLoadState === 'loaded' && highlightWords.length === 0"
          type="warning"
          :closable="false"
          show-icon
          style="margin: 8px;"
        >
          <template #title>
            该词汇本没有可用词条，请返回词汇本检查数据。
          </template>
        </el-alert>
        <PreviewPanel
          ref="previewRef"
          :html="previewScope === 'current' ? previewHtml : ''"
          :html-chunks="allPreviewHtmlChunks"
          :preview-scope="previewScope"
          :loaded-chapters="Math.min(allPreviewLimit, editorStore.chapterList.length)"
          :total-chapters="editorStore.chapterList.length"
          :fullscreen="previewFullscreen"
          :reading-mode="previewReadingMode"
          :chapter-title="currentChapterTitle"
          :chapter-index="editorStore.activeChapterIndex"
          :reading-overall-progress="previewOverallProgress"
          :reading-remaining-minutes="previewRemainingMinutes"
          @toggle-fullscreen="togglePreviewFullscreen"
          @update:preview-scope="previewScope = $event"
          @load-more="loadMorePreviewChapters"
          @enter-reading-mode="enterPreviewReadingMode"
          @exit-reading-mode="exitPreviewReadingMode"
          @previous-chapter="goToPreviousPreviewChapter"
          @next-chapter="goToNextPreviewChapter"
          @reading-progress="onPreviewReadingProgress"
        />
      </div>
    </div>

    <div v-if="readingMode && loadState === 'loaded'" class="reading-bottom-bar" role="navigation" :aria-label="t('reading.navigation')">
      <el-button text :aria-label="t('reading.previousChapter')" :disabled="!hasPreviousChapter" @click="goToPreviousChapter">
        <el-icon><ArrowLeft /></el-icon> {{ t('reading.previousChapter') }}
      </el-button>
      <div class="reading-progress-wrap" :title="t('reading.shortcutHint')">
        <el-progress :percentage="readingPercent" :stroke-width="5" :show-text="false" />
        <span>{{ currentChapterTitle }} · {{ readingPercent }}%</span>
      </div>
      <el-button text :aria-label="t('reading.nextChapter')" :disabled="!hasNextChapter" @click="goToNextChapter">
        {{ t('reading.nextChapter') }} <el-icon><ArrowRight /></el-icon>
      </el-button>
    </div>

    <!-- Loading -->
    <div
      v-else-if="loadState === 'loading'"
      class="editor-state-block editor-loading-state"
      role="status"
      aria-live="polite"
    >
      <el-icon class="is-loading" :size="32"><Loading /></el-icon>
      <div class="loading-stage-title">{{ loadStageLabel }}</div>
      <el-progress
        class="loading-stage-progress"
        :percentage="loadStageProgress"
        :stroke-width="8"
        :show-text="false"
      />
      <div class="loading-stage-detail">{{ loadStageDetail }}</div>
      <div class="loading-stage-elapsed">已等待 {{ elapsedSeconds }}s</div>
    </div>

    <!-- Error / not-found -->
    <div v-else-if="loadState === 'error'" class="editor-state-block error">
      <el-result icon="error" title="无法加载小说" :sub-title="errorMessage">
        <template #extra>
          <el-button type="primary" @click="retry">重试</el-button>
          <el-button @click="goBack">返回列表</el-button>
        </template>
      </el-result>
    </div>


  </div>
</template>

<script setup lang="ts">
import { readingProgressForChapters } from '@/utils/workspace'
import ReadingPanels from '@/components/novel/ReadingPanels.vue'
import WindowControls from '@/components/layout/WindowControls.vue'
import { useVocabBookStore } from '@/stores/vocabBookStore'
import { isMobile } from '@/utils/platform'

import { ref, computed, watch, onMounted, onBeforeUnmount, nextTick, defineAsyncComponent } from 'vue'
import { useRoute, useRouter, onBeforeRouteLeave } from 'vue-router'
import {
  ArrowLeft,
  ArrowRight,
  Loading,
  Printer,
  Reading,
  Setting,
  DArrowLeft,
  DArrowRight,
} from '@element-plus/icons-vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { save } from '@tauri-apps/plugin-dialog'
import { useNovelStore } from '@/stores/novelStore'
import { useEditorStore } from '@/stores/editorStore'
import { useSettingsStore } from '@/stores/settingsStore'
import type { HighlightWord } from '@/types/vocabWord'
import { normalizeSteps, type StepNum } from '@/types/pdfSteps'
// Keep the editor implementation out of the page shell. The editor pulls in
// Tiptap/ProseMirror and is only needed after this route has rendered.
const NovelEditor = defineAsyncComponent(() => import('@/components/novel/NovelEditor.vue'))
const WordTapReader = defineAsyncComponent(() => import('@/components/novel/WordTapReader.vue'))
import ChapterList from '@/components/novel/ChapterList.vue'
import PreviewPanel from '@/components/novel/PreviewPanel.vue'
import { buildHtml as buildPreviewHtml } from '@/utils/pdfPreview'
import { ttsPlayer, splitSentenceSpans } from '@/utils/ttsPlayer'
import { buildVoiceOverrides } from '@/utils/dialogue'
import CharacterVoicePanel from '@/components/novel/CharacterVoicePanel.vue'
import { useSplitLayout } from '@/composables/useSplitLayout'
import { t } from '@/i18n'

const settingsStore = useSettingsStore()
const vocabBookStore = useVocabBookStore()
const exportConfigOpen = ref(false)
const exportError = ref('')
const exportResult = ref('')
function openExportConfig() { exportError.value = ''; exportResult.value = ''; exportConfigOpen.value = true }


const split = useSplitLayout({
  // 目录与实时预览默认折叠，双击分隔条或拖拽可展开
  left: 0,
  right: 0,
  leftRestored: 200,
  rightRestored: 280,
  min: 50,
  max: 500,
  storageKey: 'novel-editor-layout-v3',
})
const route = useRoute()
const router = useRouter()
const store = useNovelStore()
const editorStore = useEditorStore()
const editorRef = ref<InstanceType<typeof NovelEditor> | null>(null)
const previewRef = ref<InstanceType<typeof PreviewPanel> | null>(null)

const highlightBookId = ref<number | null>(null)
const highlightWords = ref<HighlightWord[]>([])
const highlightLoadState = ref<'idle' | 'loading' | 'loaded' | 'error'>('idle')
const highlightLoadError = ref('')
let highlightRequestId = 0
const exportingPdf = ref(false)
const pdfPercent = ref(0)
const pdfMessage = ref('正在准备导出…')
const previewFullscreen = ref(false)
const previewScope = ref<'current' | 'all'>('all')
const allPreviewLimit = ref(4)
const previewReadingMode = ref(false)
const previewReadingPreviousScope = ref<'current' | 'all'>('all')
const previewChapterProgress = ref(0)
const chapterPreviewCache = new Map<string, string>()
let previewWordsVersion = 0
const stepNums: StepNum[] = [1, 2, 3]
const pdfSteps = ref<StepNum[]>([...settingsStore.pdfIntensiveSteps])

type TemplateType = 'intensive' | 'card'
const pdfTemplateType = ref<TemplateType>('intensive')
const pdfTemplateLabel = computed(() => t(pdfTemplateType.value === 'card' ? 'editor.templateCard' : 'editor.templateIntensive'))
const isEnglishMode = computed(() => store.currentNovel?.language === 'en')
/** 英文逐词阅读模式（仅专注阅读 + 英文小说时可用） */
const wordTapMode = ref(false)

// ===== TTS 朗读（普通阅读模式） =====
const ttsState = computed(() => ttsPlayer.state)
const ttsLabel = computed(() => {
  if (ttsState.value === 'playing') return t('reading.ttsPause')
  if (ttsState.value === 'paused') return t('reading.ttsResume')
  return t('reading.ttsPlay')
})
const ttsProgressLabel = computed(() => {
  const i = ttsPlayer.currentIndex.value
  const n = ttsPlayer.totalSentences.value
  return n > 0 ? `${Math.max(1, i + 1)}/${n}` : ''
})

function htmlToPlainText(html: string): string {
  const doc = new DOMParser().parseFromString(html || '', 'text/html')
  return (doc.body.textContent ?? '').replace(/\s+/g, ' ').trim()
}

/** 当前章纯文本（角色面板与朗读共用） */
const plainChapterText = computed(() => htmlToPlainText(editorContent.value))

function currentTtsSettings() {
  return {
    provider: settingsStore.ttsProvider,
    voice: settingsStore.ttsVoice,
    rate: settingsStore.ttsRate,
    pitch: settingsStore.ttsPitch,
    volume: settingsStore.ttsVolume,
    apiKey:
      settingsStore.ttsProvider === 'dashscope'
        ? settingsStore.ttsDashKey
        : settingsStore.ttsMinimaxKey,
    groupId: settingsStore.ttsMinimaxGroupId,
    sentencePauseMs: settingsStore.ttsPauseSentence,
  }
}

// ---------- 角色分音色（自加载 + CharacterVoicePanel 回传） ----------
const charPanel = ref<InstanceType<typeof CharacterVoicePanel> | null>(null)
const charVoices = ref<Record<string, string>>({})
const charGenders = ref<Record<string, 'male' | 'female' | 'unknown'>>({})
const dialogueVoiceEnabled = ref(false)

/** 面板未打开也要生效：进入编辑页时自行加载角色信息 */
async function loadCharacters(): Promise<void> {
  const novel = store.currentNovel
  if (!novel) return
  try {
    const saved = await invoke<
      Array<{ name: string; gender: string; voice: string }>
    >('list_novel_characters', { novelId: novel.id })
    const voices: Record<string, string> = {}
    const genders: Record<string, 'male' | 'female' | 'unknown'> = {}
    for (const c of saved) {
      if (c.voice) voices[c.name] = c.voice
      genders[c.name] = (c.gender as 'male' | 'female' | 'unknown') ?? 'unknown'
    }
    charVoices.value = voices
    charGenders.value = genders
  } catch {
    /* 静默：朗读走主音色 */
  }
}

void onMounted(loadCharacters)

function onCharVoicesUpdated(payload: {
  charVoices: Record<string, string>
  charGenders: Record<string, 'male' | 'female' | 'unknown'>
  dialogueEnabled: boolean
}): void {
  charVoices.value = payload.charVoices
  charGenders.value = payload.charGenders
  dialogueVoiceEnabled.value = payload.dialogueEnabled
}

async function toggleTtsReading(): Promise<void> {
  if (ttsState.value === 'playing') {
    ttsPlayer.pause()
    return
  }
  if (ttsState.value === 'paused') {
    ttsPlayer.resume()
    return
  }
  await startTtsReading()
}

async function startTtsReading(): Promise<void> {
  const fullText = plainChapterText.value
  const spans = splitSentenceSpans(fullText)
  const sentences = spans.map((s) => s.text)
  if (sentences.length === 0) return
  const overrides = dialogueVoiceEnabled.value
    ? buildVoiceOverrides(
        spans,
        fullText,
        charVoices.value,
        charGenders.value,
        { male: settingsStore.ttsMaleVoice, female: settingsStore.ttsFemaleVoice },
        settingsStore.ttsQuoteStyles,
      )
    : undefined
  await ttsPlayer.start(
    sentences,
    currentTtsSettings(),
    {
      onFinish: (completed) => {
        if (completed && settingsStore.ttsAutoNext && hasNextChapter.value) {
          void (async () => {
            await scrollToChapter(editorStore.activeChapterIndex + 1, { keepTts: true })
            await nextTick()
            if (readingMode.value && !wordTapMode.value) await startTtsReading()
          })()
        }
      },
    },
    overrides,
  )
}

function stopTtsReading(): void {
  ttsPlayer.stop()
}

async function onWordTapTtsNext(): Promise<void> {
  if (hasNextChapter.value) {
    await scrollToChapter(editorStore.activeChapterIndex + 1, { keepTts: true })
  }
}
const coverEnabled = ref(false)
const pageNumbersEnabled = ref(false)

type ReadingFont = 'system' | 'serif' | 'mono'
const readingMode = ref(route.query.mode === 'read')
const readingSettingsOpen = ref(false)
const readingFont = ref<ReadingFont>('system')
const readingFontSize = ref(18)
const readingLineHeight = ref(1.9)
const readingWidth = ref(760)
const readingProgress = ref(0)

const readingStyle = computed(() => ({
  '--reading-font-family':
    readingFont.value === 'serif'
      ? 'Georgia, "Songti SC", "STSong", serif'
      : readingFont.value === 'mono'
        ? 'ui-monospace, SFMono-Regular, Consolas, monospace'
        : 'system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
  '--reading-font-size': `${readingFontSize.value}px`,
  '--reading-line-height': String(readingLineHeight.value),
  '--reading-width': `${readingWidth.value}px`,
}))

const currentChapter = computed(() => editorStore.chapterList[editorStore.activeChapterIndex] ?? null)
const currentChapterTitle = computed(() => currentChapter.value?.title || t('reading.fullText'))
const hasPreviousChapter = computed(() => editorStore.activeChapterIndex > 0)
const hasNextChapter = computed(
  () => editorStore.activeChapterIndex < editorStore.chapterList.length - 1,
)
const readingOverallProgress = computed(() => readingProgressForChapters(editorStore.chapterList, editorStore.activeChapterIndex, readingProgress.value))
const readingPercent = computed(() => Math.round(readingOverallProgress.value * 100))
const readingRemainingMinutes = computed(() => {
  const totalUnits = editorStore.chapterList.reduce(
    (sum, chapter) => sum + Math.max(0, chapter.content?.length || chapter.contentLength || 0),
    0,
  )
  if (!totalUnits || readingOverallProgress.value >= 0.999) return 0
  const unitsPerMinute = store.currentNovel?.language === 'en' ? 220 : 360
  return Math.max(1, Math.ceil((totalUnits * (1 - readingOverallProgress.value)) / unitsPerMinute))
})
const readingRemainingLabel = computed(() =>
  readingRemainingMinutes.value > 0
    ? t('reading.remaining', { n: readingRemainingMinutes.value })
    : t('reading.completed'),
)

const previewOverallProgress = computed(() => {
  const chapters = editorStore.chapterList
  if (!chapters.length) return previewChapterProgress.value
  const total = chapters.reduce(
    (sum, chapter) => sum + Math.max(1, chapter.content?.length || chapter.contentLength || 0),
    0,
  )
  const before = chapters
    .slice(0, editorStore.activeChapterIndex)
    .reduce((sum, chapter) => sum + Math.max(1, chapter.content?.length || chapter.contentLength || 0), 0)
  const currentLength = Math.max(
    1,
    chapters[editorStore.activeChapterIndex]?.content?.length ||
      chapters[editorStore.activeChapterIndex]?.contentLength || 0,
  )
  return Math.min(1, Math.max(0, (before + currentLength * previewChapterProgress.value) / total))
})
const previewRemainingMinutes = computed(() => {
  const total = editorStore.chapterList.reduce(
    (sum, chapter) => sum + Math.max(0, chapter.content?.length || chapter.contentLength || 0),
    0,
  )
  if (!total || previewOverallProgress.value >= 0.999) return 0
  const speed = store.currentNovel?.language === 'en' ? 220 : 360
  return Math.max(1, Math.ceil((total * (1 - previewOverallProgress.value)) / speed))
})

function loadReadingPreferences() {
  try {
    const raw = localStorage.getItem('reading-preferences')
    if (!raw) return
    const saved = JSON.parse(raw) as Partial<{
      font: ReadingFont
      fontSize: number
      lineHeight: number
      width: number
    }>
    if (saved.font === 'system' || saved.font === 'serif' || saved.font === 'mono') {
      readingFont.value = saved.font
    }
    if (typeof saved.fontSize === 'number') {
      readingFontSize.value = Math.min(30, Math.max(15, saved.fontSize))
    }
    if (typeof saved.lineHeight === 'number') {
      readingLineHeight.value = Math.min(2.4, Math.max(1.4, saved.lineHeight))
    }
    if (saved.width === 620 || saved.width === 760 || saved.width === 920) {
      readingWidth.value = saved.width
    }
  } catch {
    /* ignore malformed local preferences */
  }
}

watch(
  [readingFont, readingFontSize, readingLineHeight, readingWidth],
  () => {
    localStorage.setItem(
      'reading-preferences',
      JSON.stringify({
        font: readingFont.value,
        fontSize: readingFontSize.value,
        lineHeight: readingLineHeight.value,
        width: readingWidth.value,
      }),
    )
  },
)

async function setReadingMode(enabled: boolean) {
  const position = editorRef.value?.getScrollPercent() ?? 0
  readingMode.value = enabled
  // 退出阅读时收起逐词模式，回到编辑器视图
  if (!enabled) {
    wordTapMode.value = false
    ttsPlayer.stop()
  }
  readingSettingsOpen.value = false
  const query = { ...route.query }
  if (enabled) query.mode = 'read'
  else delete query.mode
  await router.replace({ query })
  await nextTick()
  editorRef.value?.setScrollPercent(position)
}

function enterReadingMode() {
  setReadingMode(true)
}

function exitReadingMode() {
  setReadingMode(false)
}

function onTemplateSelect(cmd: TemplateType) {
  if (cmd === 'card' && !isEnglishMode.value) {
    ElMessage.warning('单词卡片版仅支持英文小说，请先切换到「英文模式」')
    return
  }
  pdfTemplateType.value = cmd
}

// Once store is loaded (async), sync session buffer once.
watch(
  () => [settingsStore.loaded, settingsStore.pdfIntensiveSteps] as const,
  ([loaded, v]) => {
    if (loaded) pdfSteps.value = [...v]
  },
  { immediate: true },
)

function togglePreviewFullscreen() {
  previewFullscreen.value = !previewFullscreen.value
}

function enterPreviewReadingMode() {
  previewReadingPreviousScope.value = previewScope.value
  previewScope.value = 'current'
  previewChapterProgress.value = 0
  previewReadingMode.value = true
}

function exitPreviewReadingMode() {
  previewReadingMode.value = false
  previewScope.value = previewReadingPreviousScope.value
}

function onPreviewReadingProgress(percent: number) {
  previewChapterProgress.value = Math.min(1, Math.max(0, percent))
  scheduleSaveReadingPos(previewChapterProgress.value, editorStore.activeChapterIndex)
}

async function goToPreviewChapter(delta: -1 | 1) {
  const nextIndex = editorStore.activeChapterIndex + delta
  if (nextIndex < 0 || nextIndex >= editorStore.chapterList.length) return
  editorStore.activeChapterIndex = nextIndex
  previewChapterProgress.value = 0
  await nextTick()
}

function goToPreviousPreviewChapter() {
  void goToPreviewChapter(-1)
}

function goToNextPreviewChapter() {
  void goToPreviewChapter(1)
}



function onPdfStepsChange(next: StepNum[]) {
  if (next.length === 0) {
    ElMessage.warning('至少勾选一个步骤')
    // rollback to a normalized safe value without affecting settingsStore
    pdfSteps.value = normalizeSteps(pdfSteps.value)
    return
  }
  pdfSteps.value = next
}


async function onLanguageChange(lang: string) {
  const novel = store.currentNovel
  if (!novel || (lang !== 'zh' && lang !== 'en')) return
  if (novel.language === lang) return
  try {
    await store.updateMetadata(novel.id, { language: lang })
    // 中文小说没有英文正文，单词卡片版不可用：自动切回精读版并提示。
    if (lang === 'zh' && pdfTemplateType.value === 'card') {
      pdfTemplateType.value = 'intensive'
      ElMessage.info('已切回「精读版」：单词卡片版仅支持英文小说')
    }
    ElMessage.success(`已切换为${lang === 'en' ? '英文' : '中文'}模式`)
  } catch (e: any) {
    ElMessage.error('切换模式失败: ' + String(e?.message || e))
  }
}

type LoadState = 'loading' | 'loaded' | 'error'
const loadState = ref<LoadState>('loading')
const errorMessage = ref('')
const elapsedSeconds = ref(0)

type LoadStage = 'reading' | 'parsing' | 'preparing'
const loadStage = ref<LoadStage>('reading')
const loadStageProgressOverride = ref<number | null>(null)
const loadStageProgress = computed(() => {
  if (loadStageProgressOverride.value != null) return loadStageProgressOverride.value
  if (loadStage.value === 'reading') return 28
  if (loadStage.value === 'parsing') return 62
  return 88
})
const loadStageLabel = computed(() => {
  if (loadStage.value === 'reading') return '正在读取小说'
  if (loadStage.value === 'parsing') return '正在解析正文'
  return '正在准备编辑器'
})
const loadStageDetail = computed(() => {
  if (loadStage.value === 'reading') return '正在从本地数据库读取小说内容…'
  if (loadStage.value === 'parsing') return '正在识别章节并整理正文结构…'
  return '正在准备章节导航、预览和编辑区域…'
})

let loadStartedAt = 0
let elapsedTimer: number | null = null
let hardTimeoutTimer: number | null = null

const currentNovelId = computed(() => {
  const n = Number(route.params.id)
  return Number.isFinite(n) ? n : 0
})

const editorContentOverride = ref<string | null>(null)
const editorContent = computed<string>({
  get: () =>
    editorContentOverride.value ?? currentChapter.value?.content ?? store.currentNovel?.cleanedText ?? '',
  set: (v) => {
    editorContentOverride.value = v
  },
})
function onEditorContentChange(html: string) {
  editorContentOverride.value = html
  editorStore.setChapterContent(currentChapter.value?.id || null, html)
}

const topbarTitle = computed(() => {
  if (loadState.value === 'loaded') return store.currentNovel?.title || ''
  if (loadState.value === 'error') return '加载失败'
  return '加载中...'
})

function buildChapterPreview(chapter: any, chapterIndex: number): string {
  if (chapter.id && !chapter.content) return '<p>正在加载章节…</p>'
  const chapterList = editorStore.chapterList
  const cacheKey = [
    currentNovelId.value,
    chapter.id,
    chapter.content?.length || chapter.contentLength || 0,
    chapterIndex,
    previewWordsVersion,
    highlightBookId.value || 0,
    pdfSteps.value.join(','),
    store.currentNovel?.language || '',
    pdfTemplateType.value,
    settingsStore.pdfBackground,
    coverEnabled.value ? 1 : 0,
  ].join('|')
  const cached = chapterPreviewCache.get(cacheKey)
  if (cached) return cached
  const html = buildPreviewHtml({
    chapters: [chapter],
    words: highlightWords.value as any,
    novelTitle: store.currentNovel?.title,
    steps: normalizeSteps(pdfSteps.value),
    language: store.currentNovel?.language,
    templateType: pdfTemplateType.value,
    background: settingsStore.pdfBackground,
    cover: coverEnabled.value && chapterIndex === 0,
    coverChapters: chapterList,
  })
  chapterPreviewCache.set(cacheKey, html)
  return html
}

const previewHtml = computed(() => {
  const content = editorContent.value || store.currentNovel?.cleanedText || ''
  if (!content) return '<p>无内容</p>'
  const chapterList = editorStore.chapterList
  const activeChapter = chapterList[editorStore.activeChapterIndex]
  if (activeChapter) return buildChapterPreview(activeChapter, editorStore.activeChapterIndex)
  return buildChapterPreview(
    { id: 0, novelId: 0, title: '', content, sortOrder: 0, startIndex: 0, createdAt: '' },
    0,
  )
})

const allPreviewHtmlChunks = computed(() => {
  if (previewScope.value !== 'all') return []
  return editorStore.chapterList
    .slice(0, allPreviewLimit.value)
    .map((chapter, index) => buildChapterPreview(chapter, index))
})

function loadMorePreviewChapters() {
  const nextLimit = Math.min(
    allPreviewLimit.value + 4,
    editorStore.chapterList.length,
  )
  void ensurePreviewChapters(allPreviewLimit.value, nextLimit).then(() => {
    allPreviewLimit.value = nextLimit
  })
}

async function ensurePreviewChapters(start: number, end: number) {
  const chapters = editorStore.chapterList.slice(start, end)
  await Promise.all(
    chapters
      .filter((chapter) => chapter.id > 0 && !chapter.content)
      .map((chapter) => editorStore.loadChapterContent(chapter.id)),
  )
}

watch(highlightWords, () => {
  previewWordsVersion++
  chapterPreviewCache.clear()
  allPreviewLimit.value = 4
})

/** Remove characters that are invalid in Windows file names. */
function sanitizeFilename(name: string): string {
  return name
    .replace(/[<>:"/\\|?*]/g, '_')
    .replace(/[\x00-\x1f]/g, '')
    .trim()
    .replace(/\.+$/, '')
    .slice(0, 200) || 'export'
}

/** e.g. [1,2,3] -> "S1+2+3"; [3] -> "S3" */
function pdfStepsMark(): string {
  return 'S' + normalizeSteps(pdfSteps.value).join('+')
}

/** Local date as YYYYMMDD, e.g. 20260818 */
function dateStamp(): string {
  const d = new Date()
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}${pad(d.getMonth() + 1)}${pad(d.getDate())}`
}

async function handleExportPdf() {
  exportError.value = ''; exportResult.value = ''
  const novel = store.currentNovel
  if (!novel) {
    ElMessage.error('请先打开小说')
    return
  }
  let filePath: string | null = null
  try {
    const tag = pdfTemplateType.value === 'card' ? '卡片版' : pdfStepsMark()
    filePath = await save({
      defaultPath: `${sanitizeFilename(novel.title || 'export')}_${tag}_${dateStamp()}.pdf`,
      filters: [{ name: 'PDF', extensions: ['pdf'] }],
    })
  } catch (e: any) {
    console.error('[PdfExport] save dialog failed:', e)
    exportError.value = t('ui.exportError', { message:String(e?.message || e) }); exportConfigOpen.value = true
    return
  }
  if (!filePath) return

  // Export must use the latest editor buffer, even when the 30s autosave
  // debounce has not fired yet.
  if (editorStore.isDirty) {
    try {
      await editorStore.flushSave(novel.id, editorContent.value, currentChapter.value?.id || null)
    } catch (e: any) {
      exportError.value = t('ui.exportError', { message:String(e?.message || e) }); exportConfigOpen.value = true
      return
    }
    if (editorStore.isDirty) {
      exportError.value = t('ui.exportError', { message:t('editor.unsaved') }); exportConfigOpen.value = true
      return
    }
  }

  exportingPdf.value = true
  pdfPercent.value = 0
  pdfMessage.value = t('ui.loading')

  // Listen for progress events emitted by the Rust backend during generation.
  let unlisten: UnlistenFn | null = null
  try {
    unlisten = await listen<{ percent: number; message: string }>(
      'pdf-export-progress',
      (event) => {
        pdfPercent.value = event.payload.percent
        pdfMessage.value = event.payload.message
      },
    )
  } catch (e) {
    console.warn('[PdfExport] listen failed:', e)
  }

  try {
    const resp = await invoke<{
      path: string
      total_vocab: number
      matched_words: number
      chapter_count: number
      steps_used: string
    }>('export_pdf', {
      novelId: novel.id,
      templateType: pdfTemplateType.value,
      vocabBookId: highlightBookId.value,
      steps: normalizeSteps(pdfSteps.value),
      cover: coverEnabled.value,
      pageNumbers: pageNumbersEnabled.value,
      outputPath: filePath,
    })
    exportResult.value = resp.path
  } catch (e: any) {
    console.error('[PdfExport] export_pdf failed:', e)
    exportError.value = t('ui.exportError', { message:String(e?.message || e) }); exportConfigOpen.value = true
  } finally {
    exportingPdf.value = false
    pdfPercent.value = 0
    pdfMessage.value = ''
    unlisten?.()
  }
}

async function loadNovel() {
  loadStartedAt = Date.now()
  elapsedSeconds.value = 0
  readingPosRestored = false
  restoringReadingPos = true
  loadStage.value = 'reading'
  loadStageProgressOverride.value = null
  errorMessage.value = ''
  editorContentOverride.value = null

  const id = currentNovelId.value
  if (!id) {
    errorMessage.value = `无效的小说 ID：${route.params.id}`
    loadState.value = 'error'
    return
  }

  elapsedTimer = window.setInterval(() => {
    elapsedSeconds.value = Math.floor((Date.now() - loadStartedAt) / 1000)
  }, 1000)

  hardTimeoutTimer = window.setTimeout(() => {
    if (loadState.value === 'loading') {
      errorMessage.value = '加载超时（10s）—— 请检查网络或重启应用'
      loadState.value = 'error'
      ElMessage.error('加载超时')
    }
  }, 10000)

  try {
    editorStore.reset()
    // These requests do not depend on each other. Start them together so the
    // database read for the novel, persisted chapters, and settings overlap.
    const [, storedChapters] = await Promise.all([
      store.fetchMeta(id),
      editorStore.loadStoredChapters(id),
      settingsStore.load(),
    ])
    if (loadState.value === 'error') return
    if (!store.currentNovel) {
      errorMessage.value = '小说不存在'
      loadState.value = 'error'
      return
    }
    if (storedChapters.length > 0) {
      loadStage.value = 'preparing'
      await editorStore.loadChapters(id, '', storedChapters)
      prepareSourceChapterFromQuery()
      const initialChapter = editorStore.chapterList[editorStore.activeChapterIndex]
      if (initialChapter?.id) {
        await editorStore.loadChapterContent(initialChapter.id)
        editorContentOverride.value = editorStore.chapterList[editorStore.activeChapterIndex]?.content || ''
      }
    } else {
      loadStage.value = 'parsing'
      const text = await store.fetchContent(id)
      editorContentOverride.value = text
      if (text) {
        await editorStore.loadChapters(id, text, storedChapters, (progress) => {
          loadStageProgressOverride.value = 35 + Math.round(progress * 30)
        })
        prepareSourceChapterFromQuery()
        editorContentOverride.value = editorStore.chapterList[editorStore.activeChapterIndex]?.content || text
      }
    }
    loadStage.value = 'preparing'
    loadStageProgressOverride.value = null
    loadState.value = 'loaded'
    void ensurePreviewChapters(0, allPreviewLimit.value)
    await nextTick()
    attachScrollListener()
  } catch (e: any) {
    if (loadState.value === 'error') return
    errorMessage.value = String(e?.message || e || '未知错误')
    loadState.value = 'error'
    ElMessage.error(errorMessage.value)
  } finally {
    cleanupTimers()
  }
}

function cleanupTimers() {
  if (elapsedTimer) {
    clearInterval(elapsedTimer)
    elapsedTimer = null
  }
  if (hardTimeoutTimer) {
    clearTimeout(hardTimeoutTimer)
    hardTimeoutTimer = null
  }
}

function retry() {
  loadState.value = 'loading'
  readingPosRestored = false
  loadNovel()
}

onMounted(async () => {
  void vocabBookStore.fetchAll()
  loadReadingPreferences()
  loadNovel()
})

watch(
  () => route.query.mode,
  (mode) => {
    readingMode.value = mode === 'read'
  },
)

watch(highlightBookId, async (bookId) => {
  const requestId = ++highlightRequestId
  if (!bookId) {
    highlightWords.value = []
    highlightLoadState.value = 'idle'
    highlightLoadError.value = ''
    return
  }
  highlightLoadState.value = 'loading'
  highlightLoadError.value = ''
  try {
    const normalizedBookId = Number(bookId)
    if (!Number.isSafeInteger(normalizedBookId) || normalizedBookId <= 0) {
      throw new Error(`无效的词汇本 ID：${String(bookId)}`)
    }
    const words = await invoke<HighlightWord[]>('get_highlight_words', {
      vocabBookId: normalizedBookId,
    })
    if (requestId !== highlightRequestId) return
    highlightWords.value = words
    highlightLoadState.value = 'loaded'
  } catch (e) {
    if (requestId !== highlightRequestId) return
    console.error('[NovelEditorPage] get_highlight_words failed:', e)
    highlightWords.value = []
    highlightLoadState.value = 'error'
    highlightLoadError.value = e instanceof Error ? e.message : String(e)
  }
})

onBeforeUnmount(() => {
  cleanupTimers()
  if (posSaveTimer != null) window.clearTimeout(posSaveTimer)
  scrollElCleanup?.()
  // Flush pending autosave before unmounting
  if (loadState.value === 'loaded') {
    const id = currentNovelId.value
    if (id && editorStore.isDirty) {
      void editorStore.flushSave(id, editorContent.value, currentChapter.value?.id || null)
    }
  }
  window.removeEventListener('keydown', onKeyDown)
  document.removeEventListener('visibilitychange', onVisibilityChange)
  editorStore.reset()
})

/** Manual save: Ctrl+S or Cmd+S. */
async function handleManualSave() {
  if (loadState.value !== 'loaded') return
  const id = currentNovelId.value
  if (!id) return
  await editorStore.flushSave(id, editorContent.value, currentChapter.value?.id || null)
  if (!editorStore.isDirty) {
    ElMessage.success('已保存')
  }
}

function getSourceFocusWord(): string {
  return typeof route.query.focusWord === 'string'
    ? route.query.focusWord.trim()
    : ''
}

function prepareSourceChapterFromQuery() {
  const focusChapterId = Number(route.query.focusChapterId)
  if (!Number.isSafeInteger(focusChapterId) || focusChapterId <= 0) return
  const index = editorStore.chapterList.findIndex((chapter) => chapter.id === focusChapterId)
  if (index >= 0) editorStore.activeChapterIndex = index
}

async function focusSourceWordFromQuery() {
  const focusWord = getSourceFocusWord()
  if (!focusWord) return

  prepareSourceChapterFromQuery()
  // Tiptap can finish its first DOM paint one frame after it emits ready.
  // Retry briefly so a slow/large novel cannot lose the one-shot highlight.
  for (let attempt = 0; attempt < 10; attempt += 1) {
    await nextTick()
    await new Promise<void>((resolve) =>
      requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
    )
    if (editorRef.value?.highlightText(focusWord, 5000)) return
    await new Promise<void>((resolve) => window.setTimeout(resolve, 120))
  }
  console.warn(`[NovelEditorPage] source word not found: ${focusWord}`)
}

function handleEditorReady() {
  attachScrollListener()
  if (!readingPosRestored && loadState.value === 'loaded') {
    readingPosRestored = true
    void restoreReadingPos()
  }
  void focusSourceWordFromQuery()
}

function onKeyDown(e: KeyboardEvent) {
  if (e.defaultPrevented) return
  const target = e.target instanceof Element ? e.target : null
  const interactive = !!target?.closest('input, textarea, select, button, a, [role="combobox"], [role="dialog"], .el-popper, .dict-lookup-popover')
  const overlayOpen = exportConfigOpen.value || readingSettingsOpen.value || !!document.querySelector('.reading-side-panel, .dict-lookup-popover')

  if (readingMode.value && loadState.value === 'loaded' && !interactive && !overlayOpen) {
    if (e.altKey && e.key === 'ArrowLeft') {
      e.preventDefault()
      void goToPreviousChapter()
      return
    }
    if (e.altKey && e.key === 'ArrowRight') {
      e.preventDefault()
      void goToNextChapter()
      return
    }
    if (e.key === ' ' || e.key === 'PageDown') {
      e.preventDefault()
      scrollReadingBy(1)
      return
    }
    if (e.key === 'ArrowDown' || e.key === 'ArrowRight') {
      e.preventDefault()
      scrollReadingBy(1)
      return
    }
    if (e.key === 'ArrowUp' || e.key === 'ArrowLeft' || e.key === 'PageUp') {
      e.preventDefault()
      scrollReadingBy(-1)
      return
    }
  }
  if (overlayOpen) return
  const mod = e.ctrlKey || e.metaKey
  if (!mod) return
  const key = e.key.toLowerCase()
  if (key === 's') {
    e.preventDefault()
    void handleManualSave()
  } else if (key === 'p') {
    e.preventDefault()
    if (loadState.value === 'loaded') openExportConfig()
  } else if (key === 'w') {
    e.preventDefault()
    if (loadState.value === 'loaded') togglePreviewFullscreen()
  }
}

/** Flush pending save when window becomes hidden (user switches apps). */
function onVisibilityChange() {
  if (document.hidden && loadState.value === 'loaded') {
    const id = currentNovelId.value
    if (id) {
      if (editorStore.isDirty) {
        void editorStore.flushSave(id, editorContent.value, currentChapter.value?.id || null)
      }
      void saveReadingPos()
    }
  }
}

// Register global keyboard + visibility listeners
window.addEventListener('keydown', onKeyDown)
document.addEventListener('visibilitychange', onVisibilityChange)

function goBack() {
  router.push('/novels')
}

onBeforeRouteLeave(async (_to, _from, next) => {
  if (editorStore.isDirty && loadState.value === 'loaded') {
    // Flush pending autosave so the user doesn't lose content even if they
    // confirm leaving. This covers the case where the 30s autosave hasn't
    // fired yet.
    const id = currentNovelId.value
    if (id) {
      try {
        await editorStore.flushSave(id, editorContent.value, currentChapter.value?.id || null)
      } catch {
        // If flush fails we still ask for confirmation
      }
    }
  }
  await saveReadingPos()
  if (editorStore.isDirty) {
    try {
      await ElMessageBox.confirm(
        '你有未保存的修改，确定离开吗？',
        '未保存',
        { confirmButtonText: '离开', cancelButtonText: '留下', type: 'warning' },
      )
      next()
    } catch {
      next(false)
    }
  } else {
    next()
  }
})

async function scrollToChapter(index: number, opts?: { keepTts?: boolean }) {
  if (changingChapter) return
  changingChapter = true
  if (!opts?.keepTts) ttsPlayer.stop()
  try {
    const previousChapter = currentChapter.value
    if (editorStore.isDirty) await editorStore.flushSave(currentNovelId.value, editorContent.value, previousChapter?.id || null)
    const ch = editorStore.chapterList[index]
    if (!ch) return
    if (ch.id) await editorStore.loadChapterContent(ch.id)
    editorStore.activeChapterIndex = index
    editorContentOverride.value = editorStore.chapterList[index]?.content || ch.content || ''
    await nextTick()
    const target = editorRef.value
    await target?.waitUntilReady()
    if (editorRef.value !== target || !target?.isContentReady()) return
    target?.setScrollPercent(0)
    previewRef.value?.scrollToText(ch.title)
    updateReadingState()
  } finally {
    changingChapter = false
  }
  scheduleSaveReadingPos()
}

async function goToPreviousChapter() {
  if (hasPreviousChapter.value) {
    await scrollToChapter(editorStore.activeChapterIndex - 1)
  }
}

async function goToNextChapter() {
  if (hasNextChapter.value) {
    await scrollToChapter(editorStore.activeChapterIndex + 1)
  }
}

function updateReadingState() {
  const percent = editorRef.value?.getScrollPercent() ?? 0
  readingProgress.value = Math.min(1, Math.max(0, percent))
}

function scrollReadingBy(direction: 1 | -1) {
  const el = editorRef.value?.getScrollEl()
  if (!el) return
  el.scrollBy({ top: direction * Math.max(160, el.clientHeight * 0.82), behavior: 'smooth' })
}

// ===== 阅读进度记忆（存 app_settings: reading_pos_{novelId}） =====
let posSaveTimer: number | null = null
let scrollElCleanup: (() => void) | null = null
let readingPosRestored = false
let restoringReadingPos = true
let changingChapter = false

async function saveReadingPos(percentOverride?: number, chapterIndexOverride?: number) {
  const id = currentNovelId.value
  if (!id || loadState.value !== 'loaded' || restoringReadingPos || changingChapter || !editorRef.value?.isContentReady()) return
  const percent = percentOverride ?? editorRef.value?.getScrollPercent() ?? 0
  const chapterIndex = chapterIndexOverride ?? editorStore.activeChapterIndex
  try {
    await invoke('set_setting', {
      key: `reading_pos_${id}`,
      value: JSON.stringify({ chapterIndex, percent }),
    })
  } catch {
    /* ignore */
  }
}

function scheduleSaveReadingPos(percentOverride?: number, chapterIndexOverride?: number) {
  if (restoringReadingPos || changingChapter || !editorRef.value?.isContentReady()) return
  if (posSaveTimer != null) window.clearTimeout(posSaveTimer)
  posSaveTimer = window.setTimeout(
    () => void saveReadingPos(percentOverride, chapterIndexOverride),
    800,
  )
}

async function restoreReadingPos() {
  const id = currentNovelId.value
  if (!id || loadState.value !== 'loaded') return
  restoringReadingPos = true
  try {
    const raw = await invoke<string>('get_setting', { key: `reading_pos_${id}` })
    if (!raw) return
    const pos = JSON.parse(raw) as { chapterIndex?: number; percent?: number }
    const idx = Math.max(0, Math.min(editorStore.chapterList.length - 1, pos.chapterIndex ?? 0))
    if (idx !== editorStore.activeChapterIndex) await scrollToChapter(idx)
    const target = editorRef.value
    await target?.waitUntilReady()
    await new Promise<void>(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve())))
    if (editorRef.value === target && target?.isContentReady() && typeof pos.percent === 'number') target?.setScrollPercent(pos.percent)
    updateReadingState()
  } catch {
    /* A malformed position must not prevent reading. */
  } finally {
    restoringReadingPos = false
  }
}

function attachScrollListener() {
  scrollElCleanup?.()
  const el = editorRef.value?.getScrollEl()
  if (!el) return
  const onScroll = () => {
    updateReadingState()
    scheduleSaveReadingPos()
  }
  el.addEventListener('scroll', onScroll, { passive: true })
  updateReadingState()
  scrollElCleanup = () => el.removeEventListener('scroll', onScroll)
}
</script>

<style scoped>
.editor-page {
  position: relative;
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}
.editor-topbar {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px clamp(8px, 1.5vw, 16px);
  padding: 8px 16px;
  border-bottom: 1px solid var(--border-color, #ebeef5);
  background: var(--bg-secondary, #fafafa);
  flex-shrink: 0;
}
.novel-title {
  font-size: 15px;
  font-weight: 600;
  min-width: 0;
  max-width: min(34vw, 420px);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.save-status {
  margin-left: auto;
  font-size: 12px;
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  gap: 4px;
}
.pdf-steps-dropdown {
  min-width: 260px;
  padding: 8px 12px 10px;
}
.pdf-steps-dropdown-title {
  font-size: 12px;
  color: var(--text-secondary, #909399);
  padding-bottom: 6px;
  border-bottom: 1px solid var(--border-color, #ebeef5);
  margin-bottom: 4px;
}
.editor-body {
  display: flex;
  flex: 1;
  overflow: hidden;
  min-height: 0;
}
.editor-pane {
  overflow: hidden;
  flex-shrink: 0;
}
.editor-pane.left-pane,
.editor-pane.right-pane {
  min-width: 0;
}
.editor-pane.center-pane {
  flex: 1;
  min-width: 200px;
}

.editor-page.reading-mode {
  background: var(--bg-primary, #fff);
}
.reading-mode .editor-topbar {
  min-height: 48px;
  padding: 8px clamp(14px, 4vw, 48px);
  background: var(--bg-primary, #fff);
  border-bottom-color: color-mix(in srgb, var(--border-color, #ebeef5) 70%, transparent);
}
.reading-mode .novel-title {
  max-width: min(30vw, 360px);
}
.reading-topbar-meta {
  display: flex;
  align-items: center;
  gap: 14px;
  margin-left: auto;
  color: var(--text-secondary, #909399);
  font-size: 12px;
}
.reading-chapter-label {
  max-width: min(28vw, 260px);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-regular, #303133);
  font-weight: 600;
}
.reading-hotkey-hint {
  color: var(--text-placeholder, #c0c4cc);
}
.reading-body {
  background: var(--bg-primary, #fff);
}
.reading-body .center-pane {
  width: 100%;
  min-width: 0;
}
.reading-mode :deep(.novel-editor-wrapper) {
  background: var(--bg-primary, #fff);
}
.reading-mode :deep(.tiptap-editor) {
  padding: 28px 24px 110px;
}
.reading-mode :deep(.tiptap-editor .ProseMirror) {
  max-width: var(--reading-width, 760px);
  margin: 0 auto;
  color: var(--text-primary, #1f2937);
  font-family: var(--reading-font-family);
  font-size: var(--reading-font-size);
  line-height: var(--reading-line-height);
  letter-spacing: 0.01em;
}
.reading-mode :deep(.tiptap-editor .ProseMirror p) {
  margin-bottom: 1.1em;
  text-indent: 2em;
}
.reading-mode :deep(.tiptap-editor .ProseMirror h1),
.reading-mode :deep(.tiptap-editor .ProseMirror h2),
.reading-mode :deep(.tiptap-editor .ProseMirror h3) {
  text-indent: 0;
  margin-top: 2em;
  margin-bottom: 1em;
}
.reading-mode :deep(.tiptap-editor .ProseMirror h1) {
  font-size: calc(var(--reading-font-size) * 1.65);
}
.reading-mode :deep(.tiptap-editor .ProseMirror h2) {
  font-size: calc(var(--reading-font-size) * 1.4);
}
.reading-mode :deep(.tiptap-editor .ProseMirror h3) {
  font-size: calc(var(--reading-font-size) * 1.2);
}
.reading-settings-panel {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.reading-settings-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary, #303133);
}
.reading-setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  color: var(--text-secondary, #606266);
  font-size: 12px;
}
.reading-bottom-bar {
  position: absolute;
  right: 0;
  bottom: 16px;
  left: 0;
  z-index: 5;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: clamp(8px, 2vw, 24px);
  pointer-events: none;
}
.reading-bottom-bar > * {
  pointer-events: auto;
}
.reading-progress-wrap {
  width: min(36vw, 420px);
  padding: 8px 14px;
  border: 1px solid var(--border-color, #ebeef5);
  border-radius: 8px;
  background: var(--bg-primary);
  box-shadow: none;
}
.reading-progress-wrap span {
  display: block;
  margin-top: 4px;
  overflow: hidden;
  color: var(--text-secondary, #909399);
  font-size: 11px;
  text-align: center;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* Draggable divider between panels */
.split-divider {
  width: 6px;
  flex-shrink: 0;
  cursor: col-resize;
  background: var(--border-color, #e0e0e0);
  transition: background 0.15s;
  display: flex;
  align-items: center;
  justify-content: center;
  user-select: none;
}
.split-divider:hover,
.split-divider:active {
  background: var(--accent-color, #409eff);
}
.split-divider.collapsed {
  width: 22px;
  cursor: pointer;
  background: var(--bg-secondary, #fafafa);
}
.split-divider.collapsed:hover {
  background: var(--accent-light, #ecf5ff);
}
.divider-icon {
  font-size: 14px;
  color: var(--text-secondary, #909399);
}
.editor-state-block {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  color: var(--text-secondary);
  font-size: 14px;
}
.editor-loading-state {
  min-height: 260px;
  padding: 32px 20px;
}
.loading-stage-title {
  color: var(--text-primary, #303133);
  font-size: 16px;
  font-weight: 600;
}
.loading-stage-progress {
  width: min(320px, 72vw);
}
.loading-stage-detail,
.loading-stage-elapsed {
  color: var(--text-secondary, #909399);
  font-size: 13px;
}
.loading-stage-elapsed {
  color: var(--text-placeholder, #a8abb2);
  font-size: 12px;
}
.editor-state-block.error {
  color: var(--danger-color, #f56c6c);
}

@media (prefers-reduced-motion: reduce) {
  .editor-loading-state .is-loading {
    animation: none;
  }
}

@media (max-width: 900px) {
  .editor-topbar {
    max-height: 34vh;
    padding: 7px 10px;
    overflow-y: auto;
  }
  .novel-title {
    max-width: 45vw;
  }
  .editor-body {
    flex-direction: column;
  }
  .left-pane,
  .split-divider {
    display: none !important;
  }
  .editor-pane.center-pane {
    width: 100%;
    min-width: 0;
    min-height: 0;
    flex: 1 1 58%;
  }
  .editor-pane.right-pane {
    width: 100% !important;
    height: 42%;
    min-height: 160px;
    flex: 0 1 42%;
    border-top: 1px solid var(--border-color, #ebeef5);
  }
}

@media (max-width: 560px) {
  .editor-topbar {
    align-content: flex-start;
  }
  .novel-title {
    max-width: calc(100vw - 150px);
  }
  .save-status {
    width: 100%;
    margin-left: 0;
    order: 3;
  }
  .reading-mode .editor-topbar {
    gap: 6px;
    padding: 7px 10px;
  }
  .reading-mode .novel-title {
    max-width: 40vw;
  }
  .reading-topbar-meta {
    gap: 6px;
    font-size: 11px;
  }
  .reading-chapter-label {
    max-width: 24vw;
  }
  .reading-hotkey-hint {
    display: none;
  }
  .reading-bottom-bar {
    bottom: 8px;
    gap: 2px;
  }
  .reading-bottom-bar :deep(.el-button) {
    padding: 8px 5px;
  }

  .reading-progress-wrap {
    width: min(48vw, 260px);
  }
  .pdf-export-dialog {
    width: calc(100vw - 28px);
    max-width: none;
    padding: 20px 18px;
  }
}

.export-config-hint { color:var(--text-secondary); margin-bottom:20px; line-height:1.7; }
.export-config-progress { margin:20px 0; }
.editor-topbar { min-height:48px; height:auto; flex-wrap:wrap; gap:8px; }
.editor-topbar .novel-title { flex:1; min-width:80px; }
.reading-topbar-meta { display:none; }
@media(max-width:600px) { .editor-topbar { padding:8px; }.reading-mode .save-status { display:none; } }
</style>
