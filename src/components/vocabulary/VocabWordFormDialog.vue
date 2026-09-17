<template>
  <el-dialog v-model="visible" :title="isEdit ? t('wordForm.edit') : t('vocabDetail.addWord')" width="min(480px, calc(100vw - 32px))" :close-on-click-modal="false">
    <el-form class="responsive-dialog-form" ref="formRef" :model="form" :rules="rules" label-width="80px">
      <el-form-item :label="t('vocabDetail.word')" prop="word">
        <el-input v-model="form.word" :placeholder="t('wordForm.wordPlaceholder')" :aria-label="t('vocabDetail.word')" maxlength="200" show-word-limit />
      </el-form-item>
      <el-alert v-if="inherited" class="learning-state" :title="t('wordForm.inherited', { status: t(`vocabDetail.${inherited.proficiency}`) })" type="info" :closable="false" show-icon />
      <p v-else-if="lookupLoading" class="lookup-hint" role="status">{{ t('wordForm.checking') }}</p>
      <p v-else-if="lookupError" class="lookup-hint" role="status">{{ t('wordForm.lookupFailed') }}</p>
      <el-form-item :label="t('vocabDetail.phonetic')" prop="phonetic"><el-input v-model="form.phonetic" :placeholder="t('wordForm.optional')" /></el-form-item>
      <el-form-item :label="t('vocabDetail.definition')" prop="definition"><el-input v-model="form.definition" :placeholder="t('wordForm.optional')" /></el-form-item>
      <el-form-item :label="t('vocabDetail.example')" prop="exampleSentence"><el-input v-model="form.exampleSentence" :placeholder="t('wordForm.optional')" /></el-form-item>
      <el-form-item :label="t('vocabDetail.proficiency')" prop="proficiency">
        <el-select v-model="form.proficiency" :disabled="!!inherited || lookupLoading" :aria-label="t('vocabDetail.proficiency')" style="width: 100%" @change="proficiencyChanged = true">
          <el-option v-for="value in proficiencyValues" :key="value" :label="t(`vocabDetail.${value}`)" :value="value" />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('wordForm.tag')" prop="memoryTag"><el-input v-model="form.memoryTag" :placeholder="t('wordForm.tagPlaceholder')" /></el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visible = false">{{ t('preset.cancel') }}</el-button>
      <el-button type="primary" @click="handleSubmit" :loading="submitting" :disabled="lookupLoading">{{ isEdit ? t('wordForm.save') : t('wordForm.add') }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch, computed, onBeforeUnmount } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { FormInstance, FormRules } from 'element-plus'
import type { Proficiency, UserVocabEntry, VocabWord, VocabWordFormData } from '@/types/vocabWord'
import { parseMemoryTag } from '@/utils/srs'
import { t } from '@/i18n'

const props = defineProps<{ modelValue: boolean; word?: VocabWord | null }>()
const emit = defineEmits<{ (e: 'update:modelValue', v: boolean): void; (e: 'submit', data: VocabWordFormData): void }>()
const visible = computed({ get: () => props.modelValue, set: value => emit('update:modelValue', value) })
const isEdit = computed(() => !!props.word)
const submitting = ref(false)
const formRef = ref<FormInstance>()
const inherited = ref<UserVocabEntry | null>(null)
const lookupLoading = ref(false)
const lookupError = ref(false)
const proficiencyChanged = ref(false)
const proficiencyValues: Proficiency[] = ['unknown', 'familiar', 'mastered']
const form = reactive<VocabWordFormData>({ word: '', definition: '', phonetic: '', exampleSentence: '', proficiency: 'unknown', memoryTag: '' })
const rules = computed<FormRules>(() => ({ word: [{ required: true, whitespace: true, message: t('wordForm.wordPlaceholder'), trigger: 'blur' }] }))
let lookupGeneration = 0
let lookupTimer: ReturnType<typeof setTimeout> | null = null
let disposed = false

function resetForm() {
  ++lookupGeneration
  if (lookupTimer) clearTimeout(lookupTimer)
  inherited.value = null; lookupLoading.value = false; lookupError.value = false; proficiencyChanged.value = false
  const word = props.word
  Object.assign(form, word ? { word: word.word, definition: word.definition, phonetic: word.phonetic, exampleSentence: word.exampleSentence, proficiency: word.proficiency, memoryTag: parseMemoryTag(word.memoryTag).tag } : { word: '', definition: '', phonetic: '', exampleSentence: '', proficiency: 'unknown', memoryTag: '' })
  formRef.value?.clearValidate()
}

watch([() => props.word, () => props.modelValue], resetForm, { immediate: true })
watch(() => form.word, raw => {
  const request = ++lookupGeneration
  if (lookupTimer) clearTimeout(lookupTimer)
  inherited.value = null; lookupError.value = false; lookupLoading.value = false
  if (!visible.value) return
  const word = raw.trim()
  const key = (value: string) => value.replace(/[‘’]/g, "'").trim().replace(/\s+/g, ' ').toLowerCase()
  if (props.word && key(word) === key(props.word.word)) {
    // 词汇本条目不存 ignore 档（个人级状态），回退为 unknown
    form.proficiency = props.word.proficiency === 'ignore' ? 'unknown' : props.word.proficiency
    proficiencyChanged.value = false
    return
  }
  form.proficiency = 'unknown'; proficiencyChanged.value = false
  if (!word) return
  lookupLoading.value = true
  lookupTimer = setTimeout(async () => {
    try {
      const state = await invoke<UserVocabEntry | null>('lookup_user_vocab', { word })
      if (disposed || request !== lookupGeneration || !visible.value) return
      inherited.value = state
      if (state) {
        form.proficiency = state.proficiency === 'ignore' ? 'unknown' : state.proficiency
      }
    } catch {
      if (!disposed && request === lookupGeneration) lookupError.value = true
    } finally { if (!disposed && request === lookupGeneration) lookupLoading.value = false }
  }, 300)
})

async function handleSubmit() {
  if (!formRef.value || lookupLoading.value) return
  const valid = await formRef.value.validate().catch(() => false)
  if (!valid) return
  submitting.value = true
  try {
    emit('submit', { ...form, word: form.word.trim(), proficiencyChanged: proficiencyChanged.value && !inherited.value })
    visible.value = false
  } finally { submitting.value = false }
}
onBeforeUnmount(() => { disposed = true; ++lookupGeneration; if (lookupTimer) clearTimeout(lookupTimer) })
</script>

<style scoped>
.learning-state { margin-bottom: 18px; }
.lookup-hint { font-size: 13px; color: var(--text-secondary); margin: 0 0 18px; }
@media (max-width: 520px) {
  .responsive-dialog-form :deep(.el-form-item) { display: block; }
  .responsive-dialog-form :deep(.el-form-item__label) { width: 100% !important; height: auto; justify-content: flex-start; margin-bottom: 6px; }
  .responsive-dialog-form :deep(.el-form-item__content) { margin-left: 0 !important; }
}
</style>
