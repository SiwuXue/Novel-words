<template>
  <div
    ref="popoverRef"
    class="dict-lookup-popover"
    :style="{ left: posX + 'px', top: posY + 'px' }"
    @mousedown.stop
  >
    <!-- Loading -->
    <div v-if="store.looking" class="dict-state">
      <el-icon class="is-loading"><Loading /></el-icon>
      <span>查询中...</span>
    </div>

    <!-- Error -->
    <div v-else-if="store.lookupError" class="dict-state dict-error">
      <el-icon><WarningFilled /></el-icon>
      <span>{{ store.lookupError }}</span>
    </div>

    <!-- 英→中：单条结果 -->
    <template v-else-if="store.direction === 'english'">
      <template v-if="store.currentWord">
        <div class="dict-header">
          <span class="dict-word">{{ store.currentWord.word }}</span>
          <span v-if="preferredPhonetic" class="dict-phonetic">/{{ preferredPhonetic }}/</span>
          <el-button
            link
            size="small"
            class="dict-speak-btn"
            title="朗读"
            @click="speak(store.currentWord!.word)"
          >
            <el-icon><Microphone /></el-icon>
          </el-button>
        </div>
        <div class="dict-translation">{{ store.currentWord.translation || '（无释义）' }}</div>
        <div class="dict-footer">
          <el-select
            v-model="selectedBookId"
            size="small"
            placeholder="选择词汇本"
            style="flex: 1; min-width: 120px"
          >
            <el-option
              v-for="b in vocabBookStore.books"
              :key="b.id"
              :label="b.name"
              :value="b.id"
            />
          </el-select>
          <el-button
            size="small"
            type="primary"
            :disabled="!selectedBookId || addedWords.has(store.currentWord.word)"
            :loading="addingWord === store.currentWord.word"
            @click="addEnglishWord(store.currentWord)"
          >
            {{ addedWords.has(store.currentWord.word) ? '已加入' : '加入词汇本' }}
          </el-button>
        </div>
      </template>
      <div v-else class="dict-state dict-empty">
        <el-icon><Search /></el-icon>
        <span>词典中无此词</span>
      </div>
    </template>

    <!-- 中→英：列表结果 -->
    <template v-else-if="store.direction === 'chinese'">
      <div class="dict-header">
        <span class="dict-word cn">{{ store.keyword }}</span>
        <span class="dict-count" v-if="!store.looking">
          找到 {{ store.chineseResults.length }} 个匹配
        </span>
      </div>
      <div v-if="store.chineseResults.length === 0" class="dict-state dict-empty">
        <el-icon><Search /></el-icon>
        <span>未找到对应英文单词</span>
      </div>
      <div v-else class="dict-list">
        <div
          v-for="w in store.chineseResults"
          :key="w.word"
          class="dict-list-item"
        >
          <div class="dict-list-main">
            <div class="dict-list-word">
              {{ w.word }}
              <span v-if="w.phonetic_us" class="dict-list-phonetic">/{{ w.phonetic_us }}/</span>
              <el-icon class="dict-list-speak" title="朗读" @click="speak(w.word)">
                <Microphone />
              </el-icon>
            </div>
            <div class="dict-list-translation">{{ w.translation }}</div>
          </div>
          <el-button
            size="small"
            link
            type="primary"
            :disabled="!selectedBookId || addedWords.has(w.word)"
            :loading="addingWord === w.word"
            @click="addEnglishWord(w)"
          >
            {{ addedWords.has(w.word) ? '已加入' : '加入' }}
          </el-button>
        </div>
      </div>
      <div class="dict-footer" v-if="store.chineseResults.length > 0">
        <el-select
          v-model="selectedBookId"
          size="small"
          placeholder="选择词汇本"
          style="flex: 1; min-width: 120px"
        >
          <el-option
            v-for="b in vocabBookStore.books"
            :key="b.id"
            :label="b.name"
            :value="b.id"
          />
        </el-select>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { Loading, WarningFilled, Search, Microphone } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'
import { useDictionaryStore, type DictWord } from '@/stores/dictionaryStore'
import { useVocabBookStore } from '@/stores/vocabBookStore'
import { useSettingsStore } from '@/stores/settingsStore'
import { speakWord } from '@/utils/speech'
import type { VocabWord } from '@/types/vocabWord'

const props = defineProps<{
  text: string
  position: { x: number; y: number }
  novelId: number | null
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const store = useDictionaryStore()
const vocabBookStore = useVocabBookStore()
const settingsStore = useSettingsStore()

const popoverRef = ref<HTMLElement | null>(null)
const selectedBookId = ref<number | null>(null)
const addingWord = ref<string | null>(null)
const addedWords = ref<Set<string>>(new Set())
const posX = ref(0)
const posY = ref(0)

const preferredPhonetic = computed(() => {
  if (!store.currentWord) return ''
  return settingsStore.speechAccent === 'uk'
    ? store.currentWord.phonetic_uk
    : store.currentWord.phonetic_us
})

/** Manual speak button handler */
function speak(word: string) {
  speakWord(word, settingsStore.speechAccent)
}

/** Adjust position to keep popover inside viewport */
function adjustPosition() {
  if (!popoverRef.value) return
  const el = popoverRef.value
  const rect = el.getBoundingClientRect()
  let x = props.position.x
  let y = props.position.y
  // Right overflow
  if (x + rect.width > window.innerWidth - 8) {
    x = window.innerWidth - rect.width - 8
  }
  // Bottom overflow
  if (y + rect.height > window.innerHeight - 8) {
    y = props.position.y - rect.height - 8
  }
  if (x < 8) x = 8
  if (y < 8) y = 8
  posX.value = x
  posY.value = y
}

async function addEnglishWord(w: DictWord) {
  if (!selectedBookId.value) {
    ElMessage.warning('请先选择词汇本')
    return
  }
  if (addedWords.value.has(w.word)) return
  addingWord.value = w.word
  try {
    await invoke<VocabWord>('create_vocab_word', {
      vocabBookId: selectedBookId.value,
      word: w.word,
      definition: w.translation,
      phonetic: w.phonetic_us || w.phonetic_uk,
      exampleSentence: '',
      novelId: props.novelId,
      proficiency: 'unknown',
      memoryTag: '',
    })
    addedWords.value.add(w.word)
    ElMessage.success(`「${w.word}」已加入词汇本`)
  } catch (e: any) {
    const msg = String(e?.message || e)
    if (msg.includes('已存在')) {
      addedWords.value.add(w.word)
      ElMessage.info(`「${w.word}」已在词汇本中`)
    } else {
      ElMessage.error(msg)
    }
  } finally {
    addingWord.value = null
  }
}

/** Close on Escape key */
function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    emit('close')
  }
}

onMounted(async () => {
  await vocabBookStore.fetchAll()
  // Use default vocab book from settings if set
  if (settingsStore.defaultVocabBookId) {
    selectedBookId.value = settingsStore.defaultVocabBookId
  } else if (vocabBookStore.books.length > 0) {
    selectedBookId.value = vocabBookStore.books[0].id
  }
  document.addEventListener('keydown', onKeydown)
  // Wait for DOM to render then adjust
  await nextTick()
  adjustPosition()
})

onBeforeUnmount(() => {
  document.removeEventListener('keydown', onKeydown)
})
</script>

<style scoped>
.dict-lookup-popover {
  position: fixed;
  z-index: 3000;
  min-width: 280px;
  max-width: 360px;
  max-height: 60vh;
  background: var(--bg-primary, #fff);
  border: 1px solid var(--border-color, #dcdfe6);
  border-radius: 8px;
  box-shadow: 0 6px 24px rgba(0, 0, 0, 0.15);
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  font-size: 13px;
}

.dict-state {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 4px;
  color: var(--text-secondary, #909399);
}
.dict-state.dict-error { color: var(--danger-color, #f56c6c); }
.dict-state.dict-empty { color: var(--text-placeholder, #c0c4cc); }

.dict-header {
  display: flex;
  align-items: baseline;
  gap: 8px;
  flex-wrap: wrap;
}
.dict-word {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary, #303133);
}
.dict-word.cn { font-size: 15px; }
.dict-phonetic {
  color: var(--text-secondary, #909399);
  font-size: 12px;
}
.dict-speak-btn {
  margin-left: auto;
  color: var(--accent-color, #409eff);
  padding: 2px;
}
.dict-list-speak {
  cursor: pointer;
  color: var(--accent-color, #409eff);
  margin-left: 4px;
  font-size: 13px;
}
.dict-list-speak:hover { opacity: 0.7; }
.dict-count {
  color: var(--text-secondary, #909399);
  font-size: 12px;
  margin-left: auto;
}

.dict-translation {
  color: var(--text-regular, #606266);
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 200px;
  overflow-y: auto;
  padding: 4px 0;
}

.dict-list {
  max-height: 280px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.dict-list-item {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 6px 8px;
  border-radius: 4px;
  background: var(--bg-secondary, #fafafa);
}
.dict-list-main { flex: 1; min-width: 0; }
.dict-list-word {
  font-weight: 600;
  color: var(--text-primary, #303133);
}
.dict-list-phonetic {
  color: var(--text-secondary, #909399);
  font-size: 11px;
  font-weight: normal;
}
.dict-list-translation {
  color: var(--text-regular, #606266);
  font-size: 12px;
  margin-top: 2px;
  word-break: break-word;
}

.dict-footer {
  display: flex;
  gap: 8px;
  align-items: center;
  padding-top: 4px;
  border-top: 1px solid var(--border-color, #ebeef5);
}
</style>
