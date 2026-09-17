<template>
  <section class="license-panel" :aria-label="t('license.title')">
    <div class="license-heading">
      <h2>{{ t(license.authorized ? 'license.activeTitle' : 'license.activateTitle') }}</h2>
      <el-tag v-if="license.authorized" :type="license.status.mode === 'offline' ? 'warning' : 'success'">{{ t('license.' + license.status.mode) }}</el-tag>
    </div>
    <p class="license-hint">{{ t('license.description') }}</p>
    <dl v-if="license.status.maskedCardKey" class="license-details">
      <div><dt>{{ t('license.card') }}</dt><dd>{{ license.status.maskedCardKey }}</dd></div>
      <div><dt>{{ t('license.plan') }}</dt><dd>{{ license.status.plan ? t('license.plan.' + license.status.plan) : '—' }}</dd></div>
      <div><dt>{{ t('license.expires') }}</dt><dd>{{ license.status.plan === 'lifetime' ? t('license.lifetime') : formatTime(license.status.expiresAt) }}</dd></div>
      <div><dt>{{ t('license.lastVerified') }}</dt><dd>{{ formatTime(license.status.lastVerifiedAt) }}</dd></div>
      <div v-if="license.status.mode === 'offline'"><dt>{{ t('license.offlineUntil') }}</dt><dd>{{ formatTime(offlineUntil) }}</dd></div>
    </dl>
    <div v-if="license.authorized && !changingCard" class="license-actions">
      <el-button @click="openReplacement">{{ t('license.replaceCard') }}</el-button>
      <el-button :loading="license.busy === 'verify'" :disabled="requestDisabled" @click="license.verify()">{{ t('license.reverify') }}</el-button>
    </div>
    <form v-else class="license-form" @submit.prevent="activate">
      <label :for="inputId">{{ t(license.authorized ? 'license.newCard' : 'license.cardInput') }}</label>
      <el-input :id="inputId" ref="cardInput" v-model="cardKey" :placeholder="t('license.placeholder', { prefix: license.status.cardPrefix })" autocomplete="off" autocapitalize="off" spellcheck="false" :disabled="!!license.busy" maxlength="256" :aria-describedby="hintId" />
      <p :id="hintId" class="license-hint">{{ t('license.singleDevice') }}</p>
      <div class="license-actions">
        <el-button native-type="submit" type="primary" :loading="license.busy === 'activate'" :disabled="requestDisabled || !cardKey.trim()">{{ license.retryRemaining ? t('license.retryCountdown', { seconds: license.retryRemaining }) : t(license.authorized ? 'license.activateNew' : 'license.activate') }}</el-button>
        <el-button v-if="!license.authorized" :loading="license.busy === 'verify'" :disabled="requestDisabled" @click="license.verify()">{{ t('license.reverify') }}</el-button>
        <el-button v-else :disabled="!!license.busy" @click="cancelReplacement">{{ t('license.cancel') }}</el-button>
      </div>
    </form>
    <p v-if="license.errorCode && license.errorCode !== 'ACTIVATION_REQUIRED'" class="license-error" role="alert">{{ t('license.error.' + license.errorCode) }}<span v-if="license.retryRemaining"> {{ t('license.retryCountdown', { seconds: license.retryRemaining }) }}</span></p>
    <p v-else-if="!license.initialized || license.busy === 'verify'" class="license-hint" role="status">{{ t('license.checking') }}</p>
    <p v-if="notice" class="license-hint" role="status">{{ t(notice) }}</p>
    <div class="license-policy"><p>{{ t('license.offlinePolicy') }}</p><p v-if="!license.authorized">{{ t('license.dataSafe') }}</p></div>
    <div class="license-device">
      <span>{{ t('license.deviceId') }}</span>
      <code>{{ license.status.deviceId || t('license.deviceUnavailable') }}</code>
      <el-button size="small" :disabled="!license.status.deviceId" @click="copyDevice">{{ t('license.copyDevice') }}</el-button>
    </div>
    <el-button v-if="!license.authorized" class="license-backup" :loading="backingUp" @click="backup">{{ t('license.backup') }}</el-button>
  </section>
</template>
<script setup lang="ts">
import { computed, nextTick, ref, useId } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import type { InputInstance } from 'element-plus'
import { currentLocale, t } from '@/i18n'
import { useLicenseStore } from '@/stores/licenseStore'
const license = useLicenseStore()
const inputId = useId()
const hintId = useId()
const cardKey = ref('')
const cardInput = ref<InputInstance>()
const changingCard = ref(false)
const backingUp = ref(false)
const notice = ref('')
const requestDisabled = computed(() => !!license.busy || license.retryRemaining > 0)
const offlineUntil = computed(() => Math.min(license.status.tokenExpiresAt ?? Infinity, license.status.expiresAt ?? Infinity, (license.status.lastVerifiedAt ?? 0) + 86400))
function formatTime(seconds: number | null) { return seconds != null && Number.isFinite(seconds) ? new Date(seconds * 1000).toLocaleString(currentLocale.value === 'en' ? 'en-US' : 'zh-CN') : '—' }
async function activate() { notice.value = ''; if (await license.activate(cardKey.value)) { cardKey.value = ''; changingCard.value = false; notice.value = 'license.activated' } }
async function openReplacement() { changingCard.value = true; notice.value = ''; await nextTick(); cardInput.value?.focus() }
function cancelReplacement() { cardKey.value = ''; changingCard.value = false }
async function copyDevice() { try { await navigator.clipboard.writeText(license.status.deviceId); notice.value = 'license.deviceCopied' } catch { notice.value = 'license.copyFailed' } }
async function backup() {
  if (backingUp.value) return
  backingUp.value = true; notice.value = ''
  try {
    const destPath = await save({ defaultPath: `ciyue-backup-${new Date().toISOString().replace(/[:.]/g, '-')}.db`, filters: [{ name: 'SQLite', extensions: ['db'] }] })
    if (!destPath) return
    await invoke('backup_database', { destPath })
    notice.value = 'license.backupDone'
  } catch { notice.value = 'license.backupFailed' }
  finally { backingUp.value = false }
}
</script>
<style scoped>
.license-panel { width:100%; min-width:0; color:var(--text-primary); }
.license-heading { display:flex; align-items:center; flex-wrap:wrap; gap:12px; }
.license-heading h2 { margin:0; font-size:23px; line-height:1.4; font-weight:650; }
.license-hint { color:var(--text-secondary); font-size:13px; line-height:1.6; margin:10px 0 16px; }
.license-details { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); gap:16px; margin:22px 0; }
.license-details div { min-width:0; }
.license-details dt { font-size:12px; color:var(--text-secondary); margin-bottom:5px; }
.license-details dd { margin:0; font-size:14px; overflow-wrap:anywhere; }
.license-form { margin-top:24px; }
.license-form label { display:block; font-size:13px; margin-bottom:8px; }
.license-form :deep(.el-input__wrapper) { min-height:38px; }
.license-actions { display:flex; flex-wrap:wrap; gap:8px; margin:16px 0; }
.license-actions :deep(.el-button) { margin-left:0; }
.license-error { color:var(--danger-color); font-size:13px; line-height:1.6; overflow-wrap:anywhere; }
.license-policy { border-top:1px solid var(--border-color); margin-top:24px; padding-top:16px; font-size:12px; line-height:1.7; color:var(--text-secondary); }
.license-policy p { margin:0 0 7px; }
.license-device { display:flex; align-items:center; flex-wrap:wrap; gap:8px; margin-top:16px; font-size:12px; color:var(--text-secondary); }
.license-device code { flex:1; min-width:100px; font-family:inherit; overflow-wrap:anywhere; user-select:text; }
.license-device :deep(.el-button) { margin-left:0; }
.license-backup { margin-top:16px; }
@media(max-width:520px) { .license-details { grid-template-columns:1fr; gap:12px; } .license-heading h2 { font-size:20px; } }
</style>
