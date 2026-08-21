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

          <el-form-item label="朗读口音">
            <el-radio-group
              :model-value="settingsStore.speechAccent"
              @change="onAccentChange"
            >
              <el-radio-button value="us">美式</el-radio-button>
              <el-radio-button value="uk">英式</el-radio-button>
            </el-radio-group>
            <el-button link type="primary" size="small" style="margin-left:12px;" @click="onTestAccent">试听</el-button>
          </el-form-item>
        </el-form>
      </el-tab-pane>

      <!-- Backup / restore tab -->
      <el-tab-pane label="数据备份" name="backup">
        <el-form label-width="120px">
          <el-form-item label="备份数据">
            <div style="display:flex;gap:12px;align-items:center;width:100%;">
              <el-button type="primary" :loading="backingUp" @click="onBackup">
                导出数据库备份
              </el-button>
              <span class="backup-hint">生成一个 .db 文件，可复制到新电脑用于恢复</span>
            </div>
          </el-form-item>
          <el-form-item label="恢复数据">
            <div style="display:flex;gap:12px;align-items:center;width:100%;">
              <el-button type="danger" :loading="restoring" @click="onRestore">
                从备份文件恢复
              </el-button>
              <span class="backup-hint">将覆盖当前所有数据，恢复后应用会自动重启</span>
            </div>
          </el-form-item>
        </el-form>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'
import { useSettingsStore } from '@/stores/settingsStore'
import { useVocabBookStore } from '@/stores/vocabBookStore'
import { STEP_LABELS, type StepNum } from '@/types/pdfSteps'
import { speakWord, type SpeechAccent } from '@/utils/speech'

const settingsStore = useSettingsStore()
const vocabBookStore = useVocabBookStore()

const activeTab = ref('general')
const stepNums: StepNum[] = [1, 2, 3]
const localSteps = ref<StepNum[]>([...settingsStore.pdfIntensiveSteps])
const backingUp = ref(false)
const restoring = ref(false)

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

function onAccentChange(accent: SpeechAccent) {
  settingsStore.setSpeechAccent(accent)
}

function onTestAccent() {
  speakWord('hello', settingsStore.speechAccent)
}

function backupDefaultName(): string {
  const d = new Date()
  const pad = (n: number) => String(n).padStart(2, '0')
  return (
    `词阅备份-${d.getFullYear()}${pad(d.getMonth() + 1)}${pad(d.getDate())}` +
    `-${pad(d.getHours())}${pad(d.getMinutes())}${pad(d.getSeconds())}.db`
  )
}

async function onBackup() {
  backingUp.value = true
  try {
    const dest = await save({
      defaultPath: backupDefaultName(),
      filters: [{ name: 'SQLite 数据库', extensions: ['db'] }],
    })
    if (!dest) return
    const saved = await invoke<string>('backup_database', { destPath: dest })
    ElMessage.success(`备份成功：${saved}`)
  } catch (e: any) {
    ElMessage.error('备份失败: ' + String(e?.message || e))
  } finally {
    backingUp.value = false
  }
}

async function onRestore() {
  let src: string | null = null
  try {
    src = await open({
      multiple: false,
      filters: [{ name: 'SQLite 数据库', extensions: ['db'] }],
    })
  } catch (e: any) {
    ElMessage.error('打开文件失败: ' + String(e?.message || e))
    return
  }
  if (!src) return

  try {
    await ElMessageBox.confirm(
      '恢复将覆盖当前所有数据，且应用会自动重启。请确认已备份重要数据。',
      '恢复数据',
      { confirmButtonText: '覆盖并恢复', cancelButtonText: '取消', type: 'warning' },
    )
  } catch {
    return
  }

  restoring.value = true
  try {
    await invoke('restore_database', { srcPath: src })
    ElMessage.success('恢复成功，应用即将重启')
    setTimeout(() => window.location.reload(), 800)
  } catch (e: any) {
    ElMessage.error('恢复失败: ' + String(e?.message || e))
  } finally {
    restoring.value = false
  }
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
.backup-hint {
  font-size: 12px;
  color: var(--text-secondary, #909399);
}
</style>
