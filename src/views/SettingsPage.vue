<template>
  <div class="settings-page">
    <h2>设置</h2>

    <el-tabs v-model="activeTab" class="settings-tabs">
      <!-- General tab -->
      <el-tab-pane :label="t('settings.general')" name="general">
        <el-form class="settings-form" label-width="130px">
          <el-form-item :label="t('settings.theme')">
            <el-radio-group
              :model-value="settingsStore.theme"
              @change="onThemeChange"
            >
              <el-radio-button value="light">{{ t('settings.light') }}</el-radio-button>
              <el-radio-button value="dark">{{ t('settings.dark') }}</el-radio-button>
            </el-radio-group>
          </el-form-item>

          <el-form-item :label="t('settings.language')">
            <el-radio-group :model-value="currentLocale" @change="onLocaleChange">
              <el-radio-button value="zh">{{ t('settings.zh') }}</el-radio-button>
              <el-radio-button value="en">{{ t('settings.en') }}</el-radio-button>
            </el-radio-group>
          </el-form-item>

          <el-form-item :label="t('settings.exportFolder')">
            <div class="inline-field">
              <el-input
                :model-value="settingsStore.defaultExportFolder"
                :placeholder="t('settings.exportFolderPlaceholder')"
                readonly
                class="fluid-input"
              />
              <el-button @click="pickExportFolder">{{ t('settings.chooseFolder') }}</el-button>
            </div>
          </el-form-item>

          <el-form-item :label="t('settings.defaultBook')">
            <el-select
              :model-value="settingsStore.defaultVocabBookId"
              @change="onDefaultVocabBookChange"
              placeholder="—"
              clearable
              class="book-select"
            >
              <el-option
                v-for="book in vocabBookStore.books"
                :key="book.id"
                :label="book.name"
                :value="book.id"
              />
            </el-select>
          </el-form-item>

          <el-form-item :label="t('settings.steps')">
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

          <el-form-item :label="t('settings.pdfBackground')">
            <el-radio-group
              :model-value="settingsStore.pdfBackground"
              @change="onBackgroundChange"
            >
              <el-radio-button value="grid">{{ t('settings.grid') }}</el-radio-button>
              <el-radio-button value="dots">{{ t('settings.dots') }}</el-radio-button>
              <el-radio-button value="none">{{ t('settings.none') }}</el-radio-button>
            </el-radio-group>
            <span class="backup-hint inline-hint">{{ t('settings.pdfBackgroundHint') }}</span>
          </el-form-item>

          <el-form-item :label="t('settings.autoBackup')">
            <el-radio-group v-model="autoBackupLocal" @change="onAutoBackupChange">
              <el-radio-button value="off">{{ t('settings.off') }}</el-radio-button>
              <el-radio-button value="daily">{{ t('settings.daily') }}</el-radio-button>
              <el-radio-button value="weekly">{{ t('settings.weekly') }}</el-radio-button>
              <el-radio-button value="monthly">{{ t('settings.monthly') }}</el-radio-button>
            </el-radio-group>
            <span class="backup-hint inline-hint">{{ t('settings.autoBackupHint') }}</span>
          </el-form-item>

          <el-form-item :label="t('settings.accent')">
            <el-radio-group
              :model-value="settingsStore.speechAccent"
              @change="onAccentChange"
            >
              <el-radio-button value="us">{{ t('settings.us') }}</el-radio-button>
              <el-radio-button value="uk">{{ t('settings.uk') }}</el-radio-button>
            </el-radio-group>
            <el-button class="inline-hint" link type="primary" size="small" @click="onTestAccent">{{ t('settings.listen') }}</el-button>
          </el-form-item>

          <el-form-item :label="t('settings.reviewGoal')">
            <el-input-number
              :model-value="settingsStore.reviewDailyGoal"
              :min="1"
              :max="999"
              size="small"
              style="width: 120px"
              @change="onReviewGoalChange"
            />
            <span class="backup-hint inline-hint">{{ t('settings.reviewGoalHint') }}</span>
          </el-form-item>
        </el-form>
      </el-tab-pane>

      <!-- AI enhancer tab -->
      <el-tab-pane :label="t('settings.aiEnhancer')" name="ai">
        <el-alert
          :title="t('settings.aiHelpTitle')"
          :description="t('settings.aiHelp')"
          type="info"
          :closable="false"
          show-icon
          style="margin-bottom: 20px;"
        />
        <el-form class="settings-form ai-form" label-width="140px">
          <el-form-item :label="t('settings.aiEnabled')">
            <el-switch v-model="aiEnabled" />
          </el-form-item>
          <el-form-item :label="t('settings.aiProvider')" required>
            <el-select
              v-model="aiProvider"
              filterable
              style="width: 100%;"
              @change="onAiProviderChange"
            >
              <el-option
                v-for="provider in AI_PROVIDER_PRESETS"
                :key="provider.id"
                :label="provider.name"
                :value="provider.id"
              />
            </el-select>
          </el-form-item>
          <el-form-item :label="t('settings.aiModel')" required>
            <div class="model-picker">
              <el-select
                v-model="aiModel"
                filterable
                allow-create
                default-first-option
                :reserve-keyword="false"
                :placeholder="t('settings.aiModelPlaceholder')"
                style="flex: 1;"
              >
                <el-option
                  v-for="model in aiModelOptions"
                  :key="model"
                  :label="model"
                  :value="model"
                />
              </el-select>
              <el-button :loading="loadingAiModels" @click="onLoadAiModels">
                {{ t('settings.aiLoadModels') }}
              </el-button>
            </div>
            <div class="field-hint">{{ t('settings.aiModelHint') }}</div>
          </el-form-item>
          <el-form-item label="API Key">
            <el-input
              v-model="aiApiKey"
              type="password"
              show-password
              :placeholder="aiKeyConfigured ? t('settings.aiKeySaved') : t('settings.aiKeyPlaceholder')"
              autocomplete="new-password"
            />
            <div class="field-hint">{{ t('settings.aiKeyHint') }}</div>
          </el-form-item>
          <el-collapse class="ai-advanced">
            <el-collapse-item :title="t('settings.aiAdvanced')" name="advanced">
              <el-form-item :label="t('settings.aiBaseUrl')" required>
                <el-input
                  v-model="aiBaseUrl"
                  placeholder="https://api.openai.com/v1"
                  autocomplete="off"
                />
                <div class="field-hint">{{ t('settings.aiBaseUrlHint') }}</div>
              </el-form-item>
              <el-form-item label="Temperature">
                <el-input
                  v-model="aiTemperature"
                  :placeholder="t('settings.aiOptionalDefault')"
                  inputmode="decimal"
                />
                <div class="field-hint">{{ t('settings.aiTemperatureHint') }}</div>
              </el-form-item>
              <el-form-item label="Top P">
                <el-input
                  v-model="aiTopP"
                  :placeholder="t('settings.aiOptionalDefault')"
                  inputmode="decimal"
                />
                <div class="field-hint">{{ t('settings.aiTopPHint') }}</div>
              </el-form-item>
              <el-form-item :label="t('settings.aiMaxTokens')">
                <el-input
                  v-model="aiMaxTokens"
                  :placeholder="t('settings.aiOptionalDefault')"
                  inputmode="numeric"
                />
                <div class="field-hint">{{ t('settings.aiMaxTokensHint') }}</div>
              </el-form-item>
            </el-collapse-item>
          </el-collapse>
          <el-form-item>
            <div class="ai-actions">
              <el-button type="primary" :loading="savingAi" @click="onSaveAi">
                {{ t('settings.aiSave') }}
              </el-button>
              <el-button :loading="testingAi" @click="onTestAi">
                {{ t('settings.aiTest') }}
              </el-button>
              <el-button
                v-if="aiKeyConfigured"
                type="danger"
                plain
                :loading="clearingAiKey"
                @click="onClearAiKey"
              >
                {{ t('settings.aiClearKey') }}
              </el-button>
            </div>
          </el-form-item>
        </el-form>
      </el-tab-pane>

      <!-- Backup / restore tab -->
      <el-tab-pane :label="t('settings.backup')" name="backup">
        <el-form class="settings-form" label-width="130px">
          <el-form-item :label="t('settings.backupData')">
            <div class="backup-action">
              <el-button type="primary" :loading="backingUp" @click="onBackup">
                {{ t('settings.backupData') }}
              </el-button>
              <span class="backup-hint">{{ t('settings.backupHint') }}</span>
            </div>
          </el-form-item>
          <el-form-item :label="t('settings.restoreData')">
            <div class="backup-action">
              <el-button type="danger" :loading="restoring" @click="onRestore">
                {{ t('settings.restoreData') }}
              </el-button>
              <span class="backup-hint">{{ t('settings.restoreHint') }}</span>
            </div>
          </el-form-item>
        </el-form>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, onMounted, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'
import { useSettingsStore } from '@/stores/settingsStore'
import type { PdfBackground, AutoBackup } from '@/stores/settingsStore'
import { useVocabBookStore } from '@/stores/vocabBookStore'
import { STEP_LABELS, type StepNum } from '@/types/pdfSteps'
import { speakWord, type SpeechAccent } from '@/utils/speech'
import { currentLocale, t, setLocale, type Locale } from '@/i18n'
import { AI_PROVIDER_PRESETS, getAiProvider } from '@/config/aiProviders'

const settingsStore = useSettingsStore()
const vocabBookStore = useVocabBookStore()

const activeTab = ref('general')
const stepNums: StepNum[] = [1, 2, 3]
const localSteps = ref<StepNum[]>([...settingsStore.pdfIntensiveSteps])
const autoBackupLocal = ref<AutoBackup>(settingsStore.autoBackup)
const backingUp = ref(false)
const restoring = ref(false)
const aiEnabled = ref(false)
const aiProvider = ref('openai')
const aiBaseUrl = ref('https://api.openai.com/v1')
const aiModel = ref('')
const aiApiKey = ref('')
const aiKeyConfigured = ref(false)
const savedAiKeyConfigured = ref(false)
const savedAiProvider = ref('openai')
const savedAiBaseUrl = ref('https://api.openai.com/v1')
const discoveredAiModels = ref<string[]>([])
const aiTemperature = ref('')
const aiTopP = ref('')
const aiMaxTokens = ref('')
const savingAi = ref(false)
const testingAi = ref(false)
const loadingAiModels = ref(false)
const clearingAiKey = ref(false)

const aiModelOptions = computed(() => {
  const presetModels = getAiProvider(aiProvider.value).models
  return [...new Set([...presetModels, ...discoveredAiModels.value])]
})

interface AiSettingsView {
  enabled: boolean
  provider: string
  baseUrl: string
  model: string
  apiKeyConfigured: boolean
  temperature: number | null
  topP: number | null
  maxTokens: number | null
}

watch([aiProvider, aiBaseUrl], ([provider, baseUrl]) => {
  aiKeyConfigured.value = savedAiKeyConfigured.value
    && provider === savedAiProvider.value
    && baseUrl.trim().replace(/\/+$/, '') === savedAiBaseUrl.value.trim().replace(/\/+$/, '')
})

function onLocaleChange(l: Locale) {
  void setLocale(l)
}

watch(
  () => settingsStore.autoBackup,
  (v) => {
    autoBackupLocal.value = v
  },
)

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

function onBackgroundChange(bg: PdfBackground) {
  settingsStore.setPdfBackground(bg)
}

function onAutoBackupChange(v: AutoBackup) {
  autoBackupLocal.value = v
  settingsStore.setAutoBackup(v)
}

function onReviewGoalChange(v: number | undefined) {
  if (typeof v === 'number') settingsStore.setReviewDailyGoal(v)
}

function onAccentChange(accent: SpeechAccent) {
  settingsStore.setSpeechAccent(accent)
}

function onTestAccent() {
  speakWord('hello', settingsStore.speechAccent)
}

async function loadAiSettings() {
  try {
    const settings = await invoke<AiSettingsView>('get_ai_settings')
    aiEnabled.value = settings.enabled
    aiProvider.value = settings.provider || 'openai'
    aiBaseUrl.value = settings.baseUrl || 'https://api.openai.com/v1'
    aiModel.value = settings.model
    aiKeyConfigured.value = settings.apiKeyConfigured
    savedAiKeyConfigured.value = settings.apiKeyConfigured
    savedAiProvider.value = aiProvider.value
    savedAiBaseUrl.value = aiBaseUrl.value
    aiTemperature.value = settings.temperature == null ? '' : String(settings.temperature)
    aiTopP.value = settings.topP == null ? '' : String(settings.topP)
    aiMaxTokens.value = settings.maxTokens == null ? '' : String(settings.maxTokens)
    aiApiKey.value = ''
  } catch (e: any) {
    ElMessage.error(t('settings.aiLoadFailed') + ': ' + String(e?.message || e))
  }
}

function parseOptionalNumber(value: string, label: string): number | null {
  const trimmed = value.trim()
  if (!trimmed) return null
  const parsed = Number(trimmed)
  if (!Number.isFinite(parsed)) throw new Error(`${label}: ${t('settings.aiInvalidNumber')}`)
  return parsed
}

function currentAiParameters() {
  const maxTokens = parseOptionalNumber(aiMaxTokens.value, t('settings.aiMaxTokens'))
  if (maxTokens != null && !Number.isInteger(maxTokens)) {
    throw new Error(`${t('settings.aiMaxTokens')}: ${t('settings.aiIntegerRequired')}`)
  }
  return {
    temperature: parseOptionalNumber(aiTemperature.value, 'Temperature'),
    topP: parseOptionalNumber(aiTopP.value, 'Top P'),
    maxTokens,
  }
}

function onAiProviderChange(providerId: string) {
  const preset = getAiProvider(providerId)
  discoveredAiModels.value = []
  if (preset.baseUrl) aiBaseUrl.value = preset.baseUrl
  aiModel.value = preset.models[0] || ''
  const sameSavedConnection = providerId === savedAiProvider.value
    && aiBaseUrl.value === savedAiBaseUrl.value
  aiKeyConfigured.value = sameSavedConnection
  aiApiKey.value = ''
}

async function onLoadAiModels() {
  if (loadingAiModels.value) return
  loadingAiModels.value = true
  try {
    const models = await invoke<string[]>('list_ai_models', {
      provider: aiProvider.value,
      baseUrl: aiBaseUrl.value,
      apiKey: aiApiKey.value.trim() || null,
    })
    discoveredAiModels.value = models
    if (!aiModel.value && models.length > 0) aiModel.value = models[0]
    ElMessage.success(t('settings.aiModelsLoaded', { n: models.length }))
  } catch (e: any) {
    ElMessage.warning(String(e?.message || e || t('settings.aiLoadModelsFailed')))
  } finally {
    loadingAiModels.value = false
  }
}

async function saveAiSettings(clearApiKey = false) {
  const parameters = currentAiParameters()
  await invoke('save_ai_settings', {
    enabled: aiEnabled.value,
    provider: aiProvider.value,
    baseUrl: aiBaseUrl.value,
    apiKey: aiApiKey.value.trim() || null,
    clearApiKey,
    model: aiModel.value,
    ...parameters,
  })
  const connectionChanged = savedAiProvider.value !== aiProvider.value
    || savedAiBaseUrl.value !== aiBaseUrl.value
  if (clearApiKey || (connectionChanged && !aiApiKey.value.trim())) {
    aiKeyConfigured.value = false
  } else if (aiApiKey.value.trim()) {
    aiKeyConfigured.value = true
  }
  savedAiProvider.value = aiProvider.value
  savedAiBaseUrl.value = aiBaseUrl.value
  savedAiKeyConfigured.value = aiKeyConfigured.value
  aiApiKey.value = ''
}

async function onSaveAi() {
  if (savingAi.value) return
  savingAi.value = true
  try {
    await saveAiSettings()
    ElMessage.success(t('settings.aiSaved'))
  } catch (e: any) {
    ElMessage.error(String(e?.message || e || t('settings.aiSaveFailed')))
  } finally {
    savingAi.value = false
  }
}

async function onTestAi() {
  if (testingAi.value) return
  testingAi.value = true
  try {
    const parameters = currentAiParameters()
    const message = await invoke<string>('test_ai_connection', {
      provider: aiProvider.value,
      baseUrl: aiBaseUrl.value,
      apiKey: aiApiKey.value.trim() || null,
      model: aiModel.value,
      ...parameters,
    })
    ElMessage.success(message)
  } catch (e: any) {
    ElMessage.error(String(e?.message || e || t('settings.aiTestFailed')))
  } finally {
    testingAi.value = false
  }
}

async function onClearAiKey() {
  if (clearingAiKey.value) return
  try {
    await ElMessageBox.confirm(
      t('settings.aiClearKeyConfirm'),
      t('settings.aiClearKey'),
      { type: 'warning', confirmButtonText: t('settings.aiClearKey'), cancelButtonText: t('preset.cancel') },
    )
  } catch {
    return
  }
  clearingAiKey.value = true
  try {
    await saveAiSettings(true)
    ElMessage.success(t('settings.aiKeyCleared'))
  } catch (e: any) {
    ElMessage.error(String(e?.message || e))
  } finally {
    clearingAiKey.value = false
  }
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
  void loadAiSettings()
})
</script>

<style scoped>
.settings-page {
  width: 100%;
  min-width: 0;
  min-height: 100%;
  padding: clamp(8px, 1.5vw, 24px);
}
.settings-page h2 {
  margin: 0 0 16px 0;
  font-size: 20px;
}
.settings-tabs {
  margin-top: 8px;
  min-width: 0;
}
.settings-form,
.ai-form {
  width: 100%;
  max-width: none;
}
.inline-field,
.backup-action {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  min-width: 0;
}
.inline-field {
  gap: 8px;
}
.fluid-input {
  flex: 1;
  min-width: 0;
}
.book-select {
  width: min(100%, 320px);
}
.inline-hint {
  margin-left: 12px;
}
.backup-hint {
  font-size: 12px;
  color: var(--text-secondary, #909399);
}
.field-hint {
  width: 100%;
  margin-top: 4px;
  font-size: 12px;
  line-height: 1.5;
  color: var(--text-secondary, #909399);
}
.ai-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.model-picker {
  display: flex;
  gap: 8px;
  width: 100%;
}
.ai-advanced {
  margin: 0 0 20px 140px;
  border-top: none;
}
.ai-advanced :deep(.el-collapse-item__content) {
  padding-top: 12px;
}
.ai-advanced :deep(.el-form-item) {
  margin-left: -140px;
}

@media (max-width: 760px) {
  .settings-page {
    padding: 4px;
  }
  .settings-page h2 {
    margin-bottom: 10px;
  }
  .settings-page :deep(.el-form-item) {
    display: block;
    margin-bottom: 18px;
  }
  .settings-page :deep(.el-form-item__label) {
    width: 100% !important;
    height: auto;
    justify-content: flex-start;
    margin-bottom: 7px;
    line-height: 1.4;
  }
  .settings-page :deep(.el-form-item__content) {
    width: 100%;
    min-width: 0;
    margin-left: 0 !important;
  }
  .settings-page :deep(.el-radio-group),
  .settings-page :deep(.el-checkbox-group) {
    display: flex;
    flex-wrap: wrap;
  }
  .inline-hint {
    display: block;
    width: 100%;
    margin: 6px 0 0;
    line-height: 1.5;
  }
  .ai-advanced {
    margin: 0 0 18px;
  }
  .ai-advanced :deep(.el-form-item) {
    margin-left: 0;
  }
  .backup-action {
    align-items: flex-start;
    flex-wrap: wrap;
  }
}

@media (max-width: 520px) {
  .inline-field,
  .model-picker {
    flex-direction: column;
    align-items: stretch;
  }
  .inline-field .el-button,
  .model-picker .el-button {
    width: 100%;
  }
  .ai-actions {
    width: 100%;
  }
  .ai-actions .el-button {
    flex: 1 1 calc(50% - 8px);
    margin-left: 0;
  }
}
</style>
