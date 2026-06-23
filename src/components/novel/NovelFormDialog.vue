<template>
  <el-dialog
    v-model="visible"
    :title="isEdit ? '编辑小说' : '新建小说'"
    width="500px"
    :close-on-click-modal="false"
  >
    <el-form ref="formRef" :model="form" :rules="rules" label-width="60px">
      <el-form-item label="书名" prop="title">
        <el-input v-model="form.title" placeholder="请输入书名" />
      </el-form-item>
      <el-form-item label="作者" prop="author">
        <el-input v-model="form.author" placeholder="请输入作者" />
      </el-form-item>
      <el-form-item label="分类" prop="category">
        <el-select v-model="form.category" placeholder="请选择分类" style="width:100%">
          <el-option label="玄幻" value="玄幻" />
          <el-option label="言情" value="言情" />
          <el-option label="科幻" value="科幻" />
          <el-option label="武侠" value="武侠" />
          <el-option label="都市" value="都市" />
          <el-option label="历史" value="历史" />
          <el-option label="悬疑" value="悬疑" />
          <el-option label="其他" value="其他" />
        </el-select>
      </el-form-item>
      <el-form-item v-if="!isEdit" label="正文" prop="rawText">
        <el-input v-model="form.rawText" type="textarea" :rows="6" placeholder="粘贴或输入小说正文（也可在编辑器中录入）" />
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
import type { Novel } from '@/types/novel'

const props = defineProps<{
  modelValue: boolean
  novel?: Novel | null
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', v: boolean): void
  (e: 'submit', data: { title: string; author: string; category: string; rawText: string }): void
}>()

const visible = ref(props.modelValue)
watch(() => props.modelValue, (v) => { visible.value = v })
watch(visible, (v) => { emit('update:modelValue', v) })

const isEdit = ref(false)
const submitting = ref(false)
const formRef = ref<FormInstance>()

const form = reactive({
  title: '',
  author: '',
  category: '',
  rawText: '',
})

const rules: FormRules = {
  title: [{ required: true, message: '请输入书名', trigger: 'blur' }],
  author: [{ required: true, message: '请输入作者', trigger: 'blur' }],
  category: [{ required: true, message: '请选择分类', trigger: 'change' }],
}

watch(() => props.novel, (n) => {
  if (n) {
    isEdit.value = true
    form.title = n.title
    form.author = n.author
    form.category = n.category
    form.rawText = ''
  } else {
    isEdit.value = false
    form.title = ''
    form.author = ''
    form.category = ''
    form.rawText = ''
  }
}, { immediate: true })

async function handleSubmit() {
  if (!formRef.value) return
  const valid = await formRef.value.validate().catch(() => false)
  if (!valid) return
  submitting.value = true
  try {
    emit('submit', { ...form })
    visible.value = false
  } finally {
    submitting.value = false
  }
}
</script>
