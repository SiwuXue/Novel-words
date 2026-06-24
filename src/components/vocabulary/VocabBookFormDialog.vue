<template>
  <el-dialog
    v-model="visible"
    :title="isEdit ? '编辑词汇本' : '新建词汇本'"
    width="460px"
    :close-on-click-modal="false"
  >
    <el-form ref="formRef" :model="form" :rules="rules" label-width="60px">
      <el-form-item label="名称" prop="name">
        <el-input v-model="form.name" placeholder="请输入词汇本名称" maxlength="100" show-word-limit />
      </el-form-item>
      <el-form-item label="描述" prop="description">
        <el-input
          v-model="form.description"
          type="textarea"
          :rows="3"
          placeholder="可选：简要描述词汇本用途"
          maxlength="200"
          show-word-limit
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visible = false">取消</el-button>
      <el-button type="primary" @click="handleSubmit" :loading="submitting">
        {{ isEdit ? '保存' : '创建' }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import type { FormInstance, FormRules } from 'element-plus'
import type { VocabBook } from '@/types/vocabBook'

const props = defineProps<{
  modelValue: boolean
  book?: VocabBook | null
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', v: boolean): void
  (e: 'submit', data: { name: string; description: string }): void
}>()

const visible = ref(props.modelValue)
watch(() => props.modelValue, (v) => { visible.value = v })
watch(visible, (v) => { emit('update:modelValue', v) })

const isEdit = ref(false)
const submitting = ref(false)
const formRef = ref<FormInstance>()

const form = reactive({
  name: '',
  description: '',
})

const rules: FormRules = {
  name: [{ required: true, message: '请输入词汇本名称', trigger: 'blur' }],
}

watch(
  () => props.book,
  (b) => {
    if (b) {
      isEdit.value = true
      form.name = b.name
      form.description = b.description
    } else {
      isEdit.value = false
      form.name = ''
      form.description = ''
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
    emit('submit', { name: form.name, description: form.description })
    visible.value = false
  } finally {
    submitting.value = false
  }
}
</script>
