<template>
  <div class="settings-page">
    <PageHeader :title="t('nav.settings')" />

    <el-tabs v-model="activeTab" class="settings-tabs">
      <el-tab-pane :label="t('license.title')" name="license"><LicensePanel /></el-tab-pane>
      <!-- General tab -->
      <el-tab-pane :label="t('settings.general')" name="general">
        <el-form class="settings-form" label-width="130px">
          <h3 class="settings-section-title">{{ t('ui.appearance') }}</h3>
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

          <h3 class="settings-section-title">{{ t('ui.exportSettings') }}</h3>
          <el-form-item v-if="!isAndroid" :label="t('settings.exportFolder')">
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
                {{ t('ui.pdfStep' + n) }}
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

          <h3 class="settings-section-title">{{ t('ui.backupSettings') }}</h3>
          <el-form-item :label="t('settings.autoBackup')">
            <el-radio-group v-model="autoBackupLocal" @change="onAutoBackupChange">
              <el-radio-button value="off">{{ t('settings.off') }}</el-radio-button>
              <el-radio-button value="daily">{{ t('settings.daily') }}</el-radio-button>
              <el-radio-button value="weekly">{{ t('settings.weekly') }}</el-radio-button>
              <el-radio-button value="monthly">{{ t('settings.monthly') }}</el-radio-button>
            </el-radio-group>
            <span class="backup-hint inline-hint">{{ t('settings.autoBackupHint') }}</span>
          </el-form-item>

          <h3 class="settings-section-title">{{ t('ui.reading') }}</h3>
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
          <el-form-item :label="t('settings.deeplEndpoint')">
            <el-input
              v-model="deeplEndpointLocal"
              :placeholder="t('settings.deeplEndpointPlaceholder')"
              style="max-width: 420px"
              clearable
              @change="onDeeplEndpointChange"
            />
            <span class="backup-hint inline-hint">{{ t('settings.deeplEndpointHint') }}</span>
          </el-form-item>

          <h3 class="settings-section-title">{{ t('ui.reviewSettings') }}</h3>
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

          <el-form-item :label="t('settings.dailyNewWordLimit')">
            <el-input-number
              :model-value="settingsStore.dailyNewWordLimit"
              :min="0"
              :max="500"
              size="small"
              style="width: 120px"
              @change="onDailyNewWordLimitChange"
            />
            <span class="backup-hint inline-hint">{{ t('settings.dailyNewWordLimitHint') }}</span>
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
      <!-- TTS 朗读 tab -->
      <el-tab-pane :label="t('settings.ttsTab')" name="tts">
        <el-form class="settings-form" label-width="140px">
          <el-form-item :label="t('settings.ttsVoiceMode')">
            <el-select v-model="ttsModeLocal" style="width: 280px" @change="onTtsModeChange">
              <el-option value="single" :label="t('settings.ttsModeSingle')" />
              <el-option value="dialogue" :label="t('settings.ttsModeDialogue')" />
            </el-select>
            <span class="backup-hint inline-hint">
              {{ ttsModeLocal === 'single' ? t('settings.ttsModeSingleHint') : t('settings.ttsModeDialogueHint') }}
            </span>
          </el-form-item>
          <el-form-item :label="t('settings.ttsProfile')">
            <div class="profile-row">
              <el-select
                v-model="profileActiveLocal"
                :placeholder="t('settings.ttsProfileNone')"
                clearable
                class="profile-select"
                @change="onApplyProfile"
              >
                <el-option
                  v-for="p in settingsStore.ttsProfiles"
                  :key="p.id"
                  :value="p.id"
                  :label="p.name"
                />
              </el-select>
              <el-button
                :icon="Plus"
                :title="t('settings.ttsProfileSave')"
                @click="onSaveProfile"
              />
              <el-button
                :icon="EditPen"
                :disabled="!profileActiveLocal"
                :title="t('settings.ttsProfileUpdate')"
                @click="onUpdateProfile"
              />
              <el-button
                :icon="Delete"
                type="danger"
                plain
                :disabled="!profileActiveLocal"
                :title="t('settings.ttsProfileDelete')"
                @click="onDeleteProfile"
              />
            </div>
            <span class="backup-hint inline-hint">{{ t('settings.ttsProfileHint') }}</span>
          </el-form-item>
          <el-form-item :label="t('settings.ttsProvider')">
            <el-select v-model="ttsProviderLocal" style="width: 320px" @change="onTtsProviderChange">
              <el-option value="edge" :label="t('settings.ttsEdge')" />
              <el-option value="system" :label="t('settings.ttsSystem')" />
              <el-option value="dashscope" :label="t('settings.ttsDashscope')" />
              <el-option value="minimax" :label="t('settings.ttsMinimax')" />
              <el-option value="volcengine" :label="t('settings.ttsVolcengine')" />
              <el-option value="mimo" :label="t('settings.ttsMimo')" />
              <el-option value="sapi" :label="t('settings.ttsSapi')" />
            </el-select>
            <span class="backup-hint inline-hint">{{ ttsProviderHint }}</span>
          </el-form-item>
          <el-form-item v-if="ttsProviderLocal === 'dashscope'" :label="t('settings.ttsDashKey')">
            <el-input
              v-model="ttsDashKeyLocal"
              type="password"
              show-password
              style="width: 280px"
              :placeholder="t('settings.ttsDashKey')"
              @change="onTtsKeyChange"
            />
            <span class="backup-hint inline-hint">{{ t('settings.ttsKeyHint') }}</span>
          </el-form-item>
          <el-form-item v-if="ttsProviderLocal === 'minimax'" :label="t('settings.ttsMinimaxKey')">
            <el-input
              v-model="ttsMinimaxKeyLocal"
              type="password"
              show-password
              style="width: 280px"
              :placeholder="t('settings.ttsMinimaxKey')"
              @change="onTtsKeyChange"
            />
          </el-form-item>
          <el-form-item v-if="ttsProviderLocal === 'minimax'" :label="t('settings.ttsMinimaxGroupId')">
            <el-input
              v-model="ttsMinimaxGroupIdLocal"
              style="width: 280px"
              :placeholder="t('settings.ttsMinimaxGroupId')"
              @change="onTtsKeyChange"
            />
          </el-form-item>
          <el-form-item v-if="ttsProviderLocal === 'volcengine'" :label="t('settings.ttsVolcKey')">
            <el-input
              v-model="ttsVolcKeyLocal"
              type="password"
              show-password
              style="width: 280px"
              :placeholder="t('settings.ttsVolcKey')"
              @change="onTtsKeyChange"
            />
            <span class="backup-hint inline-hint">{{ t('settings.ttsKeyHint') }}</span>
          </el-form-item>
          <el-form-item v-if="ttsProviderLocal === 'mimo'" :label="t('settings.ttsMimoKey')">
            <el-input
              v-model="ttsMimoKeyLocal"
              type="password"
              show-password
              style="width: 280px"
              :placeholder="t('settings.ttsMimoKey')"
              @change="onTtsKeyChange"
            />
            <span class="backup-hint inline-hint">{{ t('settings.ttsKeyHint') }}</span>
          </el-form-item>
          <el-form-item :label="ttsModeLocal === 'single' ? t('settings.ttsVoiceOnly') : t('settings.ttsVoice')">
            <el-select
              v-model="ttsVoiceLocal"
              filterable
              allow-create
              default-first-option
              style="width: 320px"
              @change="onTtsVoiceChange"
            >
              <el-option-group v-for="g in voiceGroups" :key="g.label" :label="g.label">
                <el-option
                  v-for="v in g.voices"
                  :key="v.id"
                  :value="v.id"
                  :label="voiceOptionLabel(v)"
                >
                  <span class="voice-row">
                    <span>{{ voiceOptionLabel(v) }}</span>
                    <span v-if="v.description" class="voice-desc">{{ v.description }}</span>
                  </span>
                </el-option>
              </el-option-group>
            </el-select>
            <el-button size="small" link type="primary" @click="previewVoice(ttsVoiceLocal)">
              {{ t('settings.ttsPreviewPlay') }}
            </el-button>
            <span v-if="isCloudProvider" class="backup-hint inline-hint">{{ t('settings.ttsVoiceCustomHint') }}</span>
          </el-form-item>
          <el-form-item v-if="ttsModeLocal === 'dialogue'" :label="t('settings.ttsMaleVoice')">
            <el-select
              v-model="ttsMaleVoiceLocal"
              filterable
              allow-create
              default-first-option
              clearable
              style="width: 320px"
              @change="onTtsGenderVoiceChange"
            >
              <el-option value="" :label="t('settings.ttsVoiceOff')" />
              <el-option-group v-for="g in voiceGroups" :key="g.label" :label="g.label">
                <el-option
                  v-for="v in g.voices"
                  :key="v.id"
                  :value="v.id"
                  :label="voiceOptionLabel(v)"
                />
              </el-option-group>
            </el-select>
            <el-button v-if="ttsMaleVoiceLocal" size="small" link type="primary" @click="previewVoice(ttsMaleVoiceLocal)">
              {{ t('settings.ttsPreviewPlay') }}
            </el-button>
            <span class="backup-hint inline-hint">{{ t('settings.ttsGenderVoiceHint') }}</span>
          </el-form-item>
          <el-form-item v-if="ttsModeLocal === 'dialogue'" :label="t('settings.ttsFemaleVoice')">
            <el-select
              v-model="ttsFemaleVoiceLocal"
              filterable
              allow-create
              default-first-option
              clearable
              style="width: 320px"
              @change="onTtsGenderVoiceChange"
            >
              <el-option value="" :label="t('settings.ttsVoiceOff')" />
              <el-option-group v-for="g in voiceGroups" :key="g.label" :label="g.label">
                <el-option
                  v-for="v in g.voices"
                  :key="v.id"
                  :value="v.id"
                  :label="voiceOptionLabel(v)"
                />
              </el-option-group>
            </el-select>
            <el-button v-if="ttsFemaleVoiceLocal" size="small" link type="primary" @click="previewVoice(ttsFemaleVoiceLocal)">
              {{ t('settings.ttsPreviewPlay') }}
            </el-button>
          </el-form-item>
          <el-form-item v-if="ttsModeLocal === 'dialogue'" :label="t('settings.ttsQuoteStyles')">
            <el-checkbox-group v-model="ttsQuoteStylesLocal" @change="onTtsQuoteStylesChange">
              <el-checkbox value="“">{{ t('settings.ttsQuoteDouble') }}</el-checkbox>
              <el-checkbox value="‘">{{ t('settings.ttsQuoteSingle') }}</el-checkbox>
              <el-checkbox value="「">{{ t('settings.ttsQuoteCorner') }}</el-checkbox>
              <el-checkbox value="『">{{ t('settings.ttsQuoteDoubleCorner') }}</el-checkbox>
            </el-checkbox-group>
          </el-form-item>
          <el-form-item :label="t('settings.ttsRate')">
            <el-slider v-model="ttsRateLocal" :min="0.5" :max="2" :step="0.05" style="width: 280px" @change="onTtsParamsChange" />
          </el-form-item>
          <el-form-item :label="t('settings.ttsPitch')">
            <el-slider v-model="ttsPitchLocal" :min="0.5" :max="2" :step="0.05" style="width: 280px" @change="onTtsParamsChange" />
          </el-form-item>
          <el-form-item :label="t('settings.ttsVolume')">
            <el-slider v-model="ttsVolumeLocal" :min="0" :max="100" :step="5" style="width: 280px" @change="onTtsParamsChange" />
          </el-form-item>
          <el-form-item :label="t('settings.ttsAutoNext')">
            <el-switch v-model="ttsAutoNextLocal" @change="onTtsAutoNextChange" />
            <span class="backup-hint inline-hint">{{ t('settings.ttsAutoNextHint') }}</span>
          </el-form-item>
          <el-form-item :label="t('settings.ttsPauseSentence')">
            <el-slider
              v-model="ttsPauseSentenceLocal"
              :min="0"
              :max="1200"
              :step="50"
              style="width: 280px"
              @change="onTtsPauseChange"
            />
            <span class="backup-hint inline-hint">{{ ttsPauseSentenceLocal === 0 ? t('settings.ttsPauseOff') : `${ttsPauseSentenceLocal}ms` }}</span>
          </el-form-item>
          <el-form-item :label="t('settings.ttsTestConn')">
            <el-button type="primary" plain size="small" :loading="ttsTesting" @click="testTtsConnection">
              {{ t('settings.ttsTestConn') }}
            </el-button>
            <span class="backup-hint inline-hint">{{ t('settings.ttsTestHint') }}</span>
          </el-form-item>
          <el-form-item label=" ">
            <div class="preview-block">
              <div class="preview-row-title">{{ t('settings.ttsPreviewZh') }}</div>
              <el-input
                v-model="previewZhLocal"
                type="textarea"
                :rows="3"
                :placeholder="DEFAULT_PREVIEW_ZH"
              />
              <div class="preview-actions">
                <el-button
                  type="primary"
                  plain
                  size="small"
                  :loading="previewLang === 'zh'"
                  :disabled="ttsPreviewBusy"
                  @click="previewSampleText('zh')"
                >
                  {{ t('settings.ttsPreviewPlay') }}
                </el-button>
                <el-button size="small" @click="resetPreviewText('zh')">
                  {{ t('settings.ttsPreviewReset') }}
                </el-button>
              </div>
              <div class="preview-row-title">{{ t('settings.ttsPreviewEn') }}</div>
              <el-input
                v-model="previewEnLocal"
                type="textarea"
                :rows="3"
                :placeholder="DEFAULT_PREVIEW_EN"
              />
              <div class="preview-actions">
                <el-button
                  type="primary"
                  plain
                  size="small"
                  :loading="previewLang === 'en'"
                  :disabled="ttsPreviewBusy"
                  @click="previewSampleText('en')"
                >
                  {{ t('settings.ttsPreviewPlay') }}
                </el-button>
                <el-button size="small" @click="resetPreviewText('en')">
                  {{ t('settings.ttsPreviewReset') }}
                </el-button>
                <el-button
                  v-if="ttsPreviewBusy"
                  type="warning"
                  plain
                  size="small"
                  @click="stopPreview"
                >
                  {{ t('settings.ttsPreviewStop') }}
                </el-button>
              </div>
              <span class="backup-hint">{{ t('settings.ttsPreviewHint') }}</span>
            </div>
          </el-form-item>
        </el-form>
      </el-tab-pane>

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
import LicensePanel from '@/components/license/LicensePanel.vue'
import PageHeader from '@/components/common/PageHeader.vue'
import { computed, ref, onMounted, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, EditPen, Delete } from '@element-plus/icons-vue'
import { invoke } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'
import { useSettingsStore } from '@/stores/settingsStore'
import type { PdfBackground, AutoBackup } from '@/stores/settingsStore'
import { useVocabBookStore } from '@/stores/vocabBookStore'
import { type StepNum } from '@/types/pdfSteps'
import { speakWord, type SpeechAccent } from '@/utils/speech'
import { ttsPlayer, splitSentenceSpans, type TtsProvider } from '@/utils/ttsPlayer'
import { buildVoiceOverrides, guessGenders } from '@/utils/dialogue'
import {
  getVoices,
  groupVoices,
  voiceOptionLabel,
  type TtsVoice,
  type VoiceGroup,
} from '@/utils/ttsVoices'
import { ttsProfileAutoLabel, type TtsVoiceMode } from '@/utils/ttsProfile'
import { isAndroid } from '@/utils/platform'
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

function onDailyNewWordLimitChange(v: number | undefined) {
  if (typeof v === 'number') settingsStore.setDailyNewWordLimit(Math.floor(v))
}

function onAccentChange(accent: SpeechAccent) {
  settingsStore.setSpeechAccent(accent)
}

function onTestAccent() {
  speakWord('hello', settingsStore.speechAccent)
}

// ===== TTS 朗读设置 =====
const ttsProviderLocal = ref<TtsProvider>(settingsStore.ttsProvider)
const ttsVoiceLocal = ref(settingsStore.ttsVoice)
const ttsRateLocal = ref(settingsStore.ttsRate)
const ttsPitchLocal = ref(settingsStore.ttsPitch)
const ttsVolumeLocal = ref(settingsStore.ttsVolume)
const ttsAutoNextLocal = ref(settingsStore.ttsAutoNext)
const ttsPauseSentenceLocal = ref(settingsStore.ttsPauseSentence)
const ttsMaleVoiceLocal = ref(settingsStore.ttsMaleVoice)
const ttsFemaleVoiceLocal = ref(settingsStore.ttsFemaleVoice)
const ttsQuoteStylesLocal = ref<string[]>([...settingsStore.ttsQuoteStyles])
const ttsTesting = ref(false)
const ttsDashKeyLocal = ref(settingsStore.ttsDashKey)
const ttsMinimaxKeyLocal = ref(settingsStore.ttsMinimaxKey)
const ttsMinimaxGroupIdLocal = ref(settingsStore.ttsMinimaxGroupId)
const ttsVolcKeyLocal = ref(settingsStore.ttsVolcKey)
const ttsMimoKeyLocal = ref(settingsStore.ttsMimoKey)
/** 当前服务商音色目录（TtsVoice 结构，含分组/性别/描述） */
const catalogVoices = ref<TtsVoice[]>([])
const previewLang = ref<'' | 'zh' | 'en'>('')

const isCloudProvider = computed(() =>
  ['dashscope', 'minimax', 'volcengine', 'mimo'].includes(ttsProviderLocal.value),
)

const ttsProviderHint = computed(() =>
  ttsProviderLocal.value === 'sapi' ? t('settings.ttsSapiOnlyWindows') : t('settings.ttsProviderHint'),
)

/** 音色按服务商记忆（localStorage），切换服务商自动带回上次选择 */
const DEFAULT_VOICE: Record<TtsProvider, string> = {
  edge: 'zh-CN-XiaoxiaoNeural',
  system: '',
  dashscope: 'Cherry',
  minimax: 'female-shaonv',
  volcengine: 'zh_female_vv_uranus_bigtts',
  mimo: 'mimo_default',
  sapi: '',
}
const VOICE_KEY_PREFIX = 'tts-voice-'
function saveProviderVoice(provider: TtsProvider, voice: string): void {
  localStorage.setItem(VOICE_KEY_PREFIX + provider, voice)
}

/** 分组渲染：火山音色多，非中英分组折叠进「更多语言」 */
const voiceGroups = computed<VoiceGroup[]>(() => {
  const groups = groupVoices(catalogVoices.value)
  if (ttsProviderLocal.value !== 'volcengine') return groups
  const main: VoiceGroup[] = []
  const rest: TtsVoice[] = []
  for (const g of groups) {
    if (g.label.startsWith('中文') || g.label.startsWith('普通话') || g.label.startsWith('方言') || g.label.startsWith('英语')) {
      main.push(g)
    } else {
      rest.push(...g.voices)
    }
  }
  if (rest.length) main.push({ label: t('settings.ttsVoicesMore'), voices: rest })
  return main
})

async function loadVoiceOptions(): Promise<void> {
  const provider = ttsProviderLocal.value
  if (provider === 'system') {
    const synth = window.speechSynthesis
    const voices = synth ? synth.getVoices() : []
    catalogVoices.value = [
      { id: '', label: t('settings.ttsSystemDefault'), group: t('settings.ttsSystem'), gender: 'unknown' },
      ...voices.map((v) => ({
        id: v.name,
        label: v.name,
        group: t('settings.ttsSystem'),
        gender: 'unknown' as const,
        description: v.lang,
      })),
    ]
    return
  }
  if (provider === 'sapi') {
    catalogVoices.value = []
    try {
      const list = await invoke<Array<[string, string, string, string]>>('tts_sapi_voices')
      catalogVoices.value = list.map(([id, label, locale, gender]) => ({
        id,
        label,
        group: locale || 'SAPI5',
        gender: gender === 'male' || gender === 'female' ? gender : 'unknown',
        description: locale,
      }))
      if (catalogVoices.value.length === 0) ElMessage.warning(t('settings.ttsSapiNoVoices'))
    } catch (e) {
      ElMessage.error(String(e))
    }
    return
  }
  // 静态全量目录（edge/dashscope/volcengine/mimo/minimax 兜底）
  let list = getVoices(provider)
  if (provider === 'minimax') {
    // MiniMax 有 Key 时优先动态拉取（含克隆音色），失败静默回退静态表
    const key = ttsMinimaxKeyLocal.value.trim()
    if (key) {
      try {
        const rows = await invoke<Array<[string, string, string]>>('tts_voices_v3', {
          provider: 'minimax',
          apiKey: key,
        })
        if (rows.length) {
          list = rows.map(([id, label, groupDesc]) => {
            const idx = groupDesc.indexOf(' · ')
            return {
              id,
              label,
              group: idx >= 0 ? groupDesc.slice(0, idx) : groupDesc,
              gender: 'unknown' as const,
              description: idx >= 0 ? groupDesc.slice(idx + 3) : '',
            }
          })
        }
      } catch {
        /* 拉取失败：保留静态表 */
      }
    }
  }
  catalogVoices.value = list
}

function onTtsProviderChange(v: TtsProvider): void {
  saveProviderVoice(ttsProviderLocal.value, ttsVoiceLocal.value)
  ttsProviderLocal.value = v
  void settingsStore.setTtsSettings({ ttsProvider: v })
  const remembered = localStorage.getItem(VOICE_KEY_PREFIX + v)
  ttsVoiceLocal.value = remembered ?? DEFAULT_VOICE[v]
  void settingsStore.setTtsSettings({ ttsVoice: ttsVoiceLocal.value })
  void loadVoiceOptions()
}

function onTtsVoiceChange(v: string): void {
  saveProviderVoice(ttsProviderLocal.value, v)
  void settingsStore.setTtsSettings({ ttsVoice: v })
}

function onTtsKeyChange(): void {
  void settingsStore.setTtsSettings({
    ttsDashKey: ttsDashKeyLocal.value.trim(),
    ttsMinimaxKey: ttsMinimaxKeyLocal.value.trim(),
    ttsMinimaxGroupId: ttsMinimaxGroupIdLocal.value.trim(),
    ttsVolcKey: ttsVolcKeyLocal.value.trim(),
    ttsMimoKey: ttsMimoKeyLocal.value.trim(),
  })
}

// ===== 朗读方案（配置方案）管理 =====
const profileActiveLocal = ref(settingsStore.ttsActiveProfile)
const ttsModeLocal = ref<TtsVoiceMode>(settingsStore.ttsVoiceMode)

watch(
  () => settingsStore.ttsVoiceMode,
  (v) => {
    ttsModeLocal.value = v
  },
)

function onTtsModeChange(v: TtsVoiceMode): void {
  void settingsStore.setTtsSettings({ ttsVoiceMode: v })
}

watch(
  () => settingsStore.ttsActiveProfile,
  (v) => {
    profileActiveLocal.value = v
  },
)

/** 套用方案后把 store 当前值同步回本地编辑态 */
function syncTtsLocals(): void {
  ttsProviderLocal.value = settingsStore.ttsProvider
  ttsVoiceLocal.value = settingsStore.ttsVoice
  ttsRateLocal.value = settingsStore.ttsRate
  ttsPitchLocal.value = settingsStore.ttsPitch
  ttsVolumeLocal.value = settingsStore.ttsVolume
  ttsModeLocal.value = settingsStore.ttsVoiceMode
  ttsMaleVoiceLocal.value = settingsStore.ttsMaleVoice
  ttsFemaleVoiceLocal.value = settingsStore.ttsFemaleVoice
  ttsQuoteStylesLocal.value = [...settingsStore.ttsQuoteStyles]
}

/** 当前设置的自动标签（保存方案时的默认名） */
function currentAutoLabel(): string {
  return ttsProfileAutoLabel({
    settings: {
      provider: ttsProviderLocal.value,
      voice: ttsVoiceLocal.value,
      rate: ttsRateLocal.value,
      pitch: ttsPitchLocal.value,
      volume: ttsVolumeLocal.value,
      voiceMode: ttsModeLocal.value,
      maleVoice: ttsMaleVoiceLocal.value,
      femaleVoice: ttsFemaleVoiceLocal.value,
      quoteStyles: ttsQuoteStylesLocal.value,
    },
  })
}

async function onApplyProfile(id: string | undefined): Promise<void> {
  if (!id) {
    // 清空选择无意义：回显当前激活方案
    profileActiveLocal.value = settingsStore.ttsActiveProfile
    return
  }
  try {
    await settingsStore.applyTtsProfile(id)
    syncTtsLocals()
    void loadVoiceOptions()
    ElMessage.success(t('settings.ttsProfileApplied'))
  } catch (e) {
    ElMessage.error(String(e))
  }
}

async function onSaveProfile(): Promise<void> {
  let name = ''
  try {
    const r = await ElMessageBox.prompt(
      t('settings.ttsProfileNamePrompt'),
      t('settings.ttsProfileNameTitle'),
      {
        inputValue: currentAutoLabel(),
        confirmButtonText: t('settings.ttsProfileSave'),
        cancelButtonText: t('preset.cancel'),
        inputPattern: /\S+/,
        inputErrorMessage: t('settings.ttsProfileNameRequired'),
      },
    )
    name = (r.value ?? '').trim()
  } catch {
    return
  }
  try {
    await settingsStore.saveTtsProfile(name)
    profileActiveLocal.value = settingsStore.ttsActiveProfile
    ElMessage.success(t('settings.ttsProfileSaved'))
  } catch (e) {
    ElMessage.error(String(e))
  }
}

async function onUpdateProfile(): Promise<void> {
  const id = profileActiveLocal.value
  if (!id) return
  try {
    await settingsStore.updateTtsProfile(id)
    ElMessage.success(t('settings.ttsProfileUpdated'))
  } catch (e) {
    ElMessage.error(String(e))
  }
}

async function onDeleteProfile(): Promise<void> {
  const id = profileActiveLocal.value
  if (!id) return
  try {
    await ElMessageBox.confirm(
      t('settings.ttsProfileDeleteConfirm'),
      t('settings.ttsProfileDelete'),
      {
        type: 'warning',
        confirmButtonText: t('settings.ttsProfileDelete'),
        cancelButtonText: t('preset.cancel'),
      },
    )
  } catch {
    return
  }
  try {
    await settingsStore.deleteTtsProfile(id)
    profileActiveLocal.value = settingsStore.ttsActiveProfile
  } catch (e) {
    ElMessage.error(String(e))
  }
}

function onTtsParamsChange(): void {
  void settingsStore.setTtsSettings({
    ttsRate: ttsRateLocal.value,
    ttsPitch: ttsPitchLocal.value,
    ttsVolume: ttsVolumeLocal.value,
  })
}

function onTtsAutoNextChange(v: boolean | string | number | undefined): void {
  void settingsStore.setTtsSettings({ ttsAutoNext: Boolean(v) })
}

function onTtsPauseChange(): void {
  void settingsStore.setTtsSettings({ ttsPauseSentence: ttsPauseSentenceLocal.value })
}

function onTtsGenderVoiceChange(): void {
  void settingsStore.setTtsSettings({
    ttsMaleVoice: ttsMaleVoiceLocal.value,
    ttsFemaleVoice: ttsFemaleVoiceLocal.value,
  })
}

function onTtsQuoteStylesChange(v: string[]): void {
  // 至少保留一个样式（复选框组允许清空，这里回退为全启用）
  const value = v.length ? [...v] : ['“', '‘', '「', '『']
  ttsQuoteStylesLocal.value = value
  void settingsStore.setTtsSettings({ ttsQuoteStyles: value })
}

/** 当前服务商对应的 API Key（edge/system/sapi 无需 Key） */
function providerApiKey(): string {
  switch (ttsProviderLocal.value) {
    case 'dashscope':
      return ttsDashKeyLocal.value.trim()
    case 'minimax':
      return ttsMinimaxKeyLocal.value.trim()
    case 'volcengine':
      return ttsVolcKeyLocal.value.trim()
    case 'mimo':
      return ttsMimoKeyLocal.value.trim()
    default:
      return ''
  }
}

function currentGroupId(): string | null {
  return ttsProviderLocal.value === 'minimax' ? ttsMinimaxGroupIdLocal.value.trim() : null
}

async function testTtsConnection(): Promise<void> {
  if (ttsProviderLocal.value === 'system') {
    const ok = 'speechSynthesis' in window
    if (ok) ElMessage.success(t('settings.ttsTestOk'))
    else ElMessage.error(t('settings.ttsTestFail'))
    return
  }
  ttsTesting.value = true
  try {
    await invoke('tts_test_connection', {
      provider: ttsProviderLocal.value,
      apiKey: providerApiKey(),
      groupId: currentGroupId(),
      voice: ttsVoiceLocal.value,
    })
    ElMessage.success(t('settings.ttsTestOk'))
  } catch (e) {
    ElMessage.error(`${t('settings.ttsTestFail')}: ${String(e)}`)
  } finally {
    ttsTesting.value = false
  }
}

/** 指定音色试听一句（主音色/男声/女声默认音色共用） */
async function previewVoice(voice: string): Promise<void> {
  const sample = voice.startsWith('zh')
    ? '你好，这是词阅的语音朗读试听。'
    : 'Hello, this is a voice reading preview from CiYue.'
  ttsPreviewing.value = true
  try {
    await ttsPlayer.start([sample], {
      provider: ttsProviderLocal.value,
      voice,
      rate: ttsRateLocal.value,
      pitch: ttsPitchLocal.value,
      volume: ttsVolumeLocal.value,
      apiKey: providerApiKey(),
      groupId: currentGroupId() ?? undefined,
      sentencePauseMs: 0,
    })
  } catch (e) {
    ElMessage.error(String(e && (e as Error).message ? (e as Error).message : e))
  } finally {
    ttsPreviewing.value = false
  }
}

// ===== 试听文本（可编辑，localStorage 持久化） =====
const DEFAULT_PREVIEW_ZH =
  '男生一脸关切的问道："身体不舒服吗？要多喝热水。"\n' +
  '女生淡淡道："你人还怪好的嘞。"\n' +
  '"贾君鹏，妈妈喊你回家吃饭！"这时外面传来一道声音。'
const DEFAULT_PREVIEW_EN =
  'Tom said, "The weather is lovely today."\n' +
  'Mary replied, "Yes, let\'s go for a walk in the park."\n' +
  '"Look out!" someone shouted from across the street.'
const PREVIEW_ZH_KEY = 'tts-preview-zh'
const PREVIEW_EN_KEY = 'tts-preview-en'

const previewZhLocal = ref(localStorage.getItem(PREVIEW_ZH_KEY) ?? DEFAULT_PREVIEW_ZH)
const previewEnLocal = ref(localStorage.getItem(PREVIEW_EN_KEY) ?? DEFAULT_PREVIEW_EN)

function resetPreviewText(lang: 'zh' | 'en'): void {
  if (lang === 'zh') {
    previewZhLocal.value = DEFAULT_PREVIEW_ZH
    localStorage.removeItem(PREVIEW_ZH_KEY)
  } else {
    previewEnLocal.value = DEFAULT_PREVIEW_EN
    localStorage.removeItem(PREVIEW_EN_KEY)
  }
}

function persistPreviewText(lang: 'zh' | 'en'): void {
  if (lang === 'zh') {
    if (previewZhLocal.value.trim() === DEFAULT_PREVIEW_ZH.trim()) {
      localStorage.removeItem(PREVIEW_ZH_KEY)
    } else {
      localStorage.setItem(PREVIEW_ZH_KEY, previewZhLocal.value)
    }
  } else if (previewEnLocal.value.trim() === DEFAULT_PREVIEW_EN.trim()) {
    localStorage.removeItem(PREVIEW_EN_KEY)
  } else {
    localStorage.setItem(PREVIEW_EN_KEY, previewEnLocal.value)
  }
}

/** 是否正在播放试听（任一来源：textarea 试听或音色试听） */
const ttsPreviewing = ref(false)
const ttsPreviewBusy = computed(() => ttsPlayer.state !== 'idle')

/** 按试听区文本合成朗读（切句后交给播放队列，含句间停顿） */
async function previewSampleText(lang: 'zh' | 'en'): Promise<void> {
  persistPreviewText(lang)
  const text = (lang === 'zh' ? previewZhLocal.value : previewEnLocal.value).trim()
  if (!text) return
  const spans = splitSentenceSpans(text)
  const sentences = spans.map((s) => s.text)
  if (sentences.length === 0) return
  // 多音色模式：对白按说话人称呼词启发式套用男/女默认音色，旁白走主音色
  let overrides: Array<string | undefined> | undefined
  if (ttsModeLocal.value === 'dialogue') {
    const male = ttsMaleVoiceLocal.value
    const female = ttsFemaleVoiceLocal.value
    if (!male && !female) {
      ElMessage.warning(t('settings.ttsModeDialogueNeedGenders'))
    } else {
      overrides = buildVoiceOverrides(
        spans,
        text,
        {},
        guessGenders(text, ttsQuoteStylesLocal.value),
        { male, female },
        ttsQuoteStylesLocal.value,
      )
    }
  }
  previewLang.value = lang
  try {
    await ttsPlayer.start(
      sentences,
      {
        provider: ttsProviderLocal.value,
        voice: ttsVoiceLocal.value,
        rate: ttsRateLocal.value,
        pitch: ttsPitchLocal.value,
        volume: ttsVolumeLocal.value,
        apiKey: providerApiKey(),
        groupId: currentGroupId() ?? undefined,
        sentencePauseMs: ttsPauseSentenceLocal.value,
      },
      {},
      overrides,
    )
  } catch (e) {
    ElMessage.error(String(e && (e as Error).message ? (e as Error).message : e))
  } finally {
    previewLang.value = ''
  }
}

function stopPreview(): void {
  ttsPlayer.stop()
}

/** DeepLX 端点本地编辑态（change 时持久化，留空恢复默认） */
const deeplEndpointLocal = ref(settingsStore.deeplEndpoint)

function onDeeplEndpointChange(value: string) {
  settingsStore.setDeeplEndpoint(value.trim())
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
  void loadVoiceOptions()
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
.preview-block {
  width: 100%;
  max-width: 560px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.preview-row-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary, #303133);
}
.preview-actions {
  display: flex;
  gap: 8px;
}
.profile-row {
  display: flex;
  gap: 8px;
  width: 100%;
  min-width: 0;
}
.profile-select {
  flex: 1;
  min-width: 200px;
}
.voice-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  max-width: 460px;
}
.voice-desc {
  font-size: 12px;
  color: var(--text-secondary, #909399);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
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
