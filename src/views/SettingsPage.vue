<template>
  <div class="settings-page">
    <h2>设置</h2>

    <el-tabs v-model="activeTab" class="settings-tabs">
      <!-- General tab -->
      <el-tab-pane label="通用" name="general">
        <el-form label-width="120px">
          <el-form-item label="主题">
            <el-radio-group
              :model-value="settingsStore.theme"
              @change="onThemeChange"
            >
              <el-radio-button value="light">浅色</el-radio-button>
              <el-radio-button value="dark">深色</el-radio-button>
            </el-radio-group>
          </el-form-item>

          <el-form-item label="默认导出目录">
            <div style="display:flex;gap:8px;width:100%;">
              <el-input
                :model-value="settingsStore.defaultExportFolder"
                placeholder="未设置（默认使用系统下载目录）"
                readonly
                style="flex:1;"
              />
              <el-button @click="pickExportFolder">选择目录</el-button>
            </div>
          </el-form-item>

          <el-form-item label="默认词汇本">
            <el-select
              :model-value="settingsStore.defaultVocabBookId"
              @change="onDefaultVocabBookChange"
              placeholder="未设置"
              clearable
              style="width:240px;"
            >
              <el-option
                v-for="book in vocabBookStore.books"
                :key="book.id"
                :label="book.name"
                :value="book.id"
              />
            </el-select>
          </el-form-item>
        </el-form>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { useSettingsStore } from '@/stores/settingsStore'
import { useVocabBookStore } from '@/stores/vocabBookStore'
import { open } from '@tauri-apps/plugin-dialog'

const settingsStore = useSettingsStore()
const vocabBookStore = useVocabBookStore()

const activeTab = ref('general')

function onThemeChange(t: 'light' | 'dark') {
  settingsStore.setTheme(t)
}

async function pickExportFolder() {
  try {
    const selected = await open({ directory: true, multiple: false })
    if (selected) {
      await settingsStore.setDefaultExportFolder(selected)
    }
  } catch (e: any) {
    ElMessage.error(String(e?.message || e || '选择目录失败'))
  }
}

function onDefaultVocabBookChange(id: number | null) {
  settingsStore.setDefaultVocabBookId(id)
}

onMounted(() => {
  if (vocabBookStore.books.length === 0) {
    vocabBookStore.fetchAll()
  }
})
</script>

<style scoped>
.settings-page {
  padding: 24px;
}
.settings-page h2 {
  margin: 0 0 16px 0;
  font-size: 20px;
}
.settings-tabs {
  margin-top: 8px;
}
</style>
