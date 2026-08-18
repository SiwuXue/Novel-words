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

          <el-form-item label="精读版导出步骤">
            <el-checkbox-group v-model="localSteps" @change="onStepsChange">
              <el-checkbox
                v-for="n in stepNums"
                :key="n"
                :label="n"
                :value="n"
              >
                {{ STEP_LABELS[n] }}
              </el-checkbox>
            </el-checkbox-group>
          </el-form-item>
        </el-form>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { ElMessage } from 'element-plus'
import { useSettingsStore } from '@/stores/settingsStore'
import { useVocabBookStore } from '@/stores/vocabBookStore'
import { open } from '@tauri-apps/plugin-dialog'
import { STEP_LABELS, type StepNum } from '@/types/pdfSteps'

const settingsStore = useSettingsStore()
const vocabBookStore = useVocabBookStore()

const activeTab = ref('general')
const stepNums: StepNum[] = [1, 2, 3]
const localSteps = ref<StepNum[]>([...settingsStore.pdfIntensiveSteps])

watch(
  () => settingsStore.pdfIntensiveSteps,
  (v) => {
    // Store 先 loaded（异步）→ 同步本地缓冲
    localSteps.value = [...v]
  },
  { once: true },
)

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

async function onStepsChange(next: StepNum[]) {
  if (next.length === 0) {
    ElMessage.warning('至少勾选一个步骤')
    // rollback: keep local buffer in sync with the last good value
    localSteps.value = [...settingsStore.pdfIntensiveSteps]
    return
  }
  await settingsStore.setPdfIntensiveSteps(next)
  localSteps.value = [...settingsStore.pdfIntensiveSteps]
}

onMounted(() => {
  if (vocabBookStore.books.length === 0) {
    vocabBookStore.fetchAll()
  }
  // In case the store was fully loaded before setup() ran.
  localSteps.value = [...settingsStore.pdfIntensiveSteps]
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
