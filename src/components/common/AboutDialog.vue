<template>
  <el-dialog
    v-model="visible"
    class="about-dialog"
    :title="t('about.title')"
    width="min(540px, calc(100vw - 32px))"
    :close-on-click-modal="false"
    :append-to-body="true"
    align-center
  >
    <div class="about-body">
      <section class="brand-card">
        <div class="brand-mark" aria-hidden="true">
          <el-icon><Reading /></el-icon>
        </div>
        <div class="brand-copy">
          <div class="brand-heading">
            <h2>{{ t('about.appName') }}</h2>
            <span class="version-badge">v{{ version || '—' }}</span>
          </div>
          <p>{{ t('about.description') }}</p>
        </div>
      </section>

      <section class="feature-grid" :aria-label="t('about.features')">
        <article class="feature-card feature-reading">
          <div class="feature-icon"><el-icon><Document /></el-icon></div>
          <h3>{{ t('about.readingTitle') }}</h3>
          <p>{{ t('about.readingDesc') }}</p>
        </article>
        <article class="feature-card feature-vocabulary">
          <div class="feature-icon"><el-icon><Collection /></el-icon></div>
          <h3>{{ t('about.vocabularyTitle') }}</h3>
          <p>{{ t('about.vocabularyDesc') }}</p>
        </article>
        <article class="feature-card feature-export">
          <div class="feature-icon"><el-icon><Printer /></el-icon></div>
          <h3>{{ t('about.exportTitle') }}</h3>
          <p>{{ t('about.exportDesc') }}</p>
        </article>
      </section>

      <section class="data-card">
        <div class="data-card-header">
          <div>
            <h3>{{ t('about.localData') }}</h3>
            <p>{{ t('about.localDataHint') }}</p>
          </div>
          <el-icon class="data-icon"><FolderOpened /></el-icon>
        </div>
        <div class="data-path-row">
          <code class="data-path" :title="dataDir">{{ dataDir || t('about.loading') }}</code>
          <div class="data-actions">
            <el-button size="small" :disabled="!dataDir" @click="copyDataDir">
              <el-icon><CopyDocument /></el-icon>
              {{ t('about.copy') }}
            </el-button>
            <el-button v-if="!isAndroid" size="small" :disabled="!dataDir" @click="openDataDir">
              <el-icon><FolderOpened /></el-icon>
              {{ t('about.open') }}
            </el-button>
          </div>
        </div>
      </section>

      <footer class="about-meta">
        <span class="privacy-note">
          <el-icon><Lock /></el-icon>
          {{ t('about.localFirst') }}
        </span>
        <span class="tech-stack">Tauri · Vue · Rust · SQLite</span>
      </footer>
    </div>

    <template #footer>
      <el-button type="primary" @click="visible = false">{{ t('about.close') }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import {
  Collection,
  CopyDocument,
  Document,
  FolderOpened,
  Lock,
  Printer,
  Reading,
} from '@element-plus/icons-vue'
import { invoke } from '@tauri-apps/api/core'
import { openPath } from '@tauri-apps/plugin-opener'
import { ElMessage } from 'element-plus'
import { t } from '@/i18n'
import { isAndroid } from '@/utils/platform'

const props = defineProps<{
  modelValue: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', v: boolean): void
}>()

const visible = ref(props.modelValue)
watch(() => props.modelValue, (value) => { visible.value = value })
watch(visible, (value) => { emit('update:modelValue', value) })

const version = ref('')
const dataDir = ref('')

onMounted(async () => {
  try {
    const info = await invoke<{ version: string; dataDir: string }>('get_app_info')
    version.value = info.version
    dataDir.value = info.dataDir
  } catch (error) {
    console.error('[AboutDialog] get_app_info failed:', error)
  }
})

async function copyDataDir() {
  if (!dataDir.value) return
  try {
    await navigator.clipboard.writeText(dataDir.value)
    ElMessage.success(t('about.copied'))
  } catch {
    ElMessage.error(t('about.copyFailed'))
  }
}

async function openDataDir() {
  if (!dataDir.value) return
  try {
    await openPath(dataDir.value)
  } catch (error: any) {
    ElMessage.error(`${t('about.openFailed')}: ${String(error?.message || error)}`)
  }
}
</script>

<style scoped>
.about-body {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.brand-card {
  display: flex;
  align-items: center;
  gap: 18px;
  padding: 20px;
  border: 1px solid color-mix(in srgb, var(--accent-color, #409eff) 18%, var(--border-color));
  border-radius: 16px;
  background:
    radial-gradient(circle at 92% 10%, rgba(64, 158, 255, 0.16), transparent 38%),
    var(--bg-secondary, #f5f7fa);
}

.brand-mark {
  display: grid;
  place-items: center;
  width: 64px;
  height: 64px;
  flex: 0 0 64px;
  border-radius: 18px;
  color: #fff;
  background: linear-gradient(145deg, #66b1ff, #337ecc);
  box-shadow: 0 10px 24px rgba(64, 158, 255, 0.28);
}

.brand-mark .el-icon {
  font-size: 34px;
}

.brand-copy {
  min-width: 0;
}

.brand-heading {
  display: flex;
  align-items: center;
  gap: 10px;
}

.brand-heading h2 {
  margin: 0;
  color: var(--text-primary, #303133);
  font-size: 24px;
  line-height: 1.25;
}

.version-badge {
  padding: 3px 8px;
  border: 1px solid rgba(64, 158, 255, 0.28);
  border-radius: 999px;
  color: var(--accent-color, #409eff);
  background: rgba(64, 158, 255, 0.09);
  font-size: 11px;
  font-weight: 600;
}

.brand-copy p {
  margin: 6px 0 0;
  color: var(--text-secondary, #909399);
  font-size: 13px;
  line-height: 1.6;
}

.feature-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
}

.feature-card {
  min-width: 0;
  padding: 14px;
  border: 1px solid var(--border-color);
  border-radius: 12px;
  background: var(--bg-primary, #fff);
  transition: border-color 0.2s, transform 0.2s;
}

.feature-card:hover {
  border-color: color-mix(in srgb, var(--feature-color) 42%, var(--border-color));
  transform: translateY(-2px);
}

.feature-icon {
  display: grid;
  place-items: center;
  width: 32px;
  height: 32px;
  margin-bottom: 10px;
  border-radius: 9px;
  color: var(--feature-color);
  background: color-mix(in srgb, var(--feature-color) 12%, transparent);
  font-size: 17px;
}

.feature-reading { --feature-color: #409eff; }
.feature-vocabulary { --feature-color: #67c23a; }
.feature-export { --feature-color: #9b6cff; }

.feature-card h3 {
  margin: 0;
  color: var(--text-primary, #303133);
  font-size: 13px;
  font-weight: 650;
}

.feature-card p {
  margin: 5px 0 0;
  color: var(--text-secondary, #909399);
  font-size: 11px;
  line-height: 1.55;
}

.data-card {
  padding: 15px;
  border: 1px solid var(--border-color);
  border-radius: 12px;
  background: var(--bg-primary, #fff);
}

.data-card-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.data-card h3 {
  margin: 0;
  color: var(--text-primary, #303133);
  font-size: 13px;
}

.data-card-header p {
  margin: 3px 0 0;
  color: var(--text-secondary, #909399);
  font-size: 11px;
}

.data-icon {
  color: var(--accent-color, #409eff);
  font-size: 18px;
}

.data-path-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 11px;
}

.data-path {
  min-width: 0;
  flex: 1;
  overflow: hidden;
  padding: 8px 10px;
  border-radius: 7px;
  color: var(--text-regular, #606266);
  background: var(--bg-secondary, #f5f7fa);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
  user-select: text;
}

.data-actions {
  display: flex;
  flex: 0 0 auto;
  gap: 6px;
}

.data-actions .el-button {
  margin-left: 0;
}

.about-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 2px 2px 0;
  color: var(--text-placeholder, #a8abb2);
  font-size: 10px;
}

.privacy-note {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  color: var(--text-secondary, #909399);
}

@media (max-width: 560px) {
  .brand-card {
    padding: 16px;
  }
  .feature-grid {
    grid-template-columns: 1fr;
  }
  .feature-card {
    display: grid;
    grid-template-columns: 36px 1fr;
    column-gap: 10px;
    padding: 10px 12px;
  }
  .feature-icon {
    grid-row: 1 / span 2;
    margin-bottom: 0;
  }
  .data-path-row,
  .about-meta {
    align-items: stretch;
    flex-direction: column;
  }
  .data-actions .el-button {
    flex: 1;
  }
}
</style>
