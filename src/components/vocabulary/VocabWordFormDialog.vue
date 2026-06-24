<template>
  <el-dialog
    v-model="visible"
    :title="isEdit ? '编辑单词' : '添加单词'"
    width="480px"
    :close-on-click-modal="false"
  >
    <el-form ref="formRef" :model="form" :rules="rules" label-width="70px">
      <el-form-item label="单词" prop="word">
        <el-input v-model="form.word" placeholder="请输入单词" maxlength="200" show-word-limit />
      </el-form-item>
      <el-form-item label="音标" prop="phonetic">
        <el-input v-model="form.phonetic" placeholder="可选" />
      </el-form-item>
      <el-form-item label="释义" prop="definition">
        <el-input v-model="form.definition" placeholder="可选" />
      </el-form-item>
      <el-form-item label="例句" prop="exampleSentence">
        <el-input v-model="form.exampleSentence" placeholder="可选" />
      </el-form-item>
      <el-form-item label="熟练度" prop="proficiency">
        <el-select v-model="form.proficiency" style="width: 100%">
          <el-option label="生疏" value="unknown" />
          <el-option label="熟悉" value="familiar" />
          <el-option label="已掌握" value="mastered" />
        </el-select>
      </el-form-item>
      <el-form-item label="标签" prop="memoryTag">
        <el-input v-model="form.memoryTag" placeholder="可选：自定义标签" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visible = false">取消</el-button>
      <el-button type="primary" @click="handleSubmit" :loading="submitting">
        {{ isEdit ? '保存' : '添加' }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import type { FormInstance, FormRules } from 'element-plus'
import type { VocabWord } from '@/types/vocabWord'

const props = defineProps<{
  modelValue: boolean
  word?: VocabWord | null
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', v: boolean): void
  (e: 'submit', data: {
    word: string
    definition: string
    phonetic: string
    exampleSentence: string
    proficiency: 'unknown' | 'familiar' | 'mastered'
    memoryTag: string
  }): void
}>()

const visible = ref(props.modelValue)
watch(() => props.modelValue, (v) => { visible.value = v })
watch(visible, (v) => { emit('update:modelValue', v) })

const isEdit = ref(false)
const submitting = ref(false)
const formRef = ref<FormInstance>()

const form = reactive({
  word: '',
  definition: '',
  phonetic: '',
  exampleSentence: '',
  proficiency: 'unknown' as 'unknown' | 'familiar' | 'mastered',
  memoryTag: '',
})

const rules: FormRules = {
  word: [{ required: true, message: '请输入单词', trigger: 'blur' }],
}

watch(
  () => props.word,
  (w) => {
    if (w) {
      isEdit.value = true
      form.word = w.word
      form.definition = w.definition
      form.phonetic = w.phonetic
      form.exampleSentence = w.exampleSentence
      form.proficiency = w.proficiency
      form.memoryTag = w.memoryTag
    } else {
      isEdit.value = false
      form.word = ''
      form.definition = ''
      form.phonetic = ''
      form.exampleSentence = ''
      form.proficiency = 'unknown'
      form.memoryTag = ''
    }
  },
  { immediate: true },
)

async function handleSubmit() {
  if (!formRef.value) return
  const valid = await formRef.value.validate().catch(() => false)
  if (!valid) return
  submitting.value = true
  try {
    emit('submit', {
      word: form.word,
      definition: form.definition,
      phonetic: form.phonetic,
      exampleSentence: form.exampleSentence,
      proficiency: form.proficiency,
      memoryTag: form.memoryTag,
    })
    visible.value = false
  } finally {
    submitting.value = false
  }
}
</script>
