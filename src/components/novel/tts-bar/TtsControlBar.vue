<script setup lang="ts">
/**
 * TTS 悬浮控制条（移植自 ColorTxt VoiceReadToolbar.vue）。
 *
 * 结构：双层滑动胶囊（播放层 ⇄ 语速/音量层，translateY(-50%) 翻转 + stagger 交错动画）
 * + 中央 48px 圆形主控键（播放/暂停两态）+ 左右 24px 半悬圆钮（齿轮=角色音色、速度=层切换）。
 *
 * 纯展示组件：全部交互通过 emit 上抛，播放逻辑在调用方（useTtsSession + ttsPlayer）。
 * 相对 ColorTxt 的裁剪：省略 engine prop（词阅全服务商支持语速）与 synthesizing 状态
 * （词阅无 AI 识别阶段）；主控键仅 --play/--pause 两态，呼吸动画预留为扩展点。
 */
import { computed, ref } from 'vue'
import { t } from '@/i18n'
import TtsIconButton from './TtsIconButton.vue'
import TtsRangeSlider from './TtsRangeSlider.vue'
import { icons } from './icons'

type ToolbarLayer = 'playback' | 'settings'

const props = withDefaults(
  defineProps<{
    /** false 时整条隐藏；缺省常驻（idle 态仅主控键与齿轮可用，作为开始朗读的入口） */
    visible?: boolean
    /** off ↔ ttsPlayer.state 'idle' */
    mode: 'off' | 'playing' | 'paused'
    /** 0.5–2.0 */
    toolbarRate: number
    /** 0–100（ColorTxt 是 0–1，词阅域直接使用） */
    toolbarVolume: number
    canPrevLine?: boolean
    canNextLine?: boolean
  }>(),
  { visible: true },
)

const emit = defineEmits<{
  'update:toolbarRate': [v: number]
  'update:toolbarVolume': [v: number]
  togglePlayPause: []
  prevLine: []
  nextLine: []
  regenerate: []
  stop: []
  openSpeakSettings: []
}>()

const toolbarLayer = ref<ToolbarLayer>('playback')

/** idle 态：无朗读会话，仅主控键（开始朗读）与齿轮可用 */
const isOff = computed(() => props.mode === 'off')

const showSettingsLayer = computed(() => toolbarLayer.value === 'settings')

const layerToggleLabel = computed(() =>
  showSettingsLayer.value ? t('ttsBar.backToControls') : t('ttsBar.adjust'),
)

function toggleToolbarLayer(): void {
  toolbarLayer.value = showSettingsLayer.value ? 'playback' : 'settings'
}

const playIcon = computed(() => (props.mode === 'playing' ? icons.pause : icons.play))
const playLabel = computed(() =>
  props.mode === 'playing' ? t('ttsBar.pause') : t('ttsBar.play'),
)
</script>

<template>
  <div v-if="visible" class="tts-control-bar" role="toolbar" :aria-label="t('ttsBar.adjust')">
    <div class="pill">
      <div class="barCore">
        <div class="layersViewport">
          <div class="layersTrack" :class="{ 'layersTrack--settings': showSettingsLayer }">
            <!-- 朗读控制 -->
            <div class="layer layerPlayback" :class="{ 'layer--hidden': showSettingsLayer }">
              <div class="layerPlaybackInner">
                <TtsIconButton
                  class="layerBtn layerBtn--stagger"
                  :icon-html="icons.prev"
                  :title="t('ttsBar.prev')"
                  :aria-label="t('ttsBar.prev')"
                  :disabled="isOff || !canPrevLine"
                  @click="emit('prevLine')"
                />
                <TtsIconButton
                  class="layerBtn layerBtn--stagger"
                  :icon-html="icons.refresh"
                  :title="t('ttsBar.regenerate')"
                  :aria-label="t('ttsBar.regenerate')"
                  :disabled="isOff"
                  @click="emit('regenerate')"
                />
                <div class="playSpacer" aria-hidden="true" />
                <TtsIconButton
                  class="layerBtn layerBtn--stagger layerBtn--stop"
                  :icon-html="icons.stop"
                  :title="t('ttsBar.stop')"
                  :aria-label="t('ttsBar.stop')"
                  :disabled="isOff"
                  @click="emit('stop')"
                />
                <TtsIconButton
                  class="layerBtn layerBtn--stagger"
                  :icon-html="icons.next"
                  :title="t('ttsBar.next')"
                  :aria-label="t('ttsBar.next')"
                  :disabled="isOff || !canNextLine"
                  @click="emit('nextLine')"
                />
              </div>
            </div>

            <!-- 朗读调节 -->
            <div class="layer layerSettings" :class="{ 'layer--hidden': !showSettingsLayer }">
              <div class="layerSettingsInner">
                <div class="side side--stagger">
                  <span class="lbl">{{ t('ttsBar.rate') }}</span>
                  <TtsRangeSlider
                    class="rateSlider"
                    :model-value="toolbarRate"
                    :min="0.5"
                    :max="2"
                    :step="0.05"
                    :aria-label="t('ttsBar.rate')"
                    @update:model-value="emit('update:toolbarRate', $event)"
                  />
                </div>
                <div class="playSpacer" aria-hidden="true" />
                <div class="side side--stagger">
                  <span class="lbl">{{ t('ttsBar.volume') }}</span>
                  <TtsRangeSlider
                    class="volumeSlider"
                    :model-value="toolbarVolume"
                    :min="0"
                    :max="100"
                    :step="5"
                    :aria-label="t('ttsBar.volumePercent', { n: Math.round(toolbarVolume) })"
                    @update:model-value="emit('update:toolbarVolume', $event)"
                  />
                </div>
              </div>
            </div>
          </div>
        </div>

        <button
          type="button"
          class="layerToggle layerToggle--left"
          :title="t('ttsBar.openVoices')"
          :aria-label="t('ttsBar.openVoices')"
          @click="emit('openSpeakSettings')"
        >
          <span class="layerToggleIcon" v-html="icons.setting" />
        </button>

        <TtsIconButton
          class="playPauseBtn"
          :class="{
            'playPauseBtn--play': mode !== 'playing',
            'playPauseBtn--pause': mode === 'playing',
          }"
          :icon-html="playIcon"
          :title="playLabel"
          :aria-label="playLabel"
          @click="emit('togglePlayPause')"
        />

        <button
          type="button"
          class="layerToggle"
          :title="layerToggleLabel"
          :aria-label="layerToggleLabel"
          @click="toggleToolbarLayer"
        >
          <span class="layerToggleIcon" v-html="icons.speed" />
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.tts-control-bar {
  /* 主题变量别名（--tts-*）：dark/light 切换自动跟随 */
  --tts-bg: var(--bg-primary);
  --tts-border: var(--border-color);
  --tts-primary: var(--accent-color);
  --tts-primary-hover: color-mix(in srgb, var(--accent-color) 85%, #000);
  --tts-danger: var(--danger-color);
  --tts-danger-hover: color-mix(in srgb, var(--danger-color) 85%, #000);
  --tts-warning: var(--warning-color); /* 扩展点：合成中呼吸态（ColorTxt --synth-*） */
  --tts-on-accent: var(--on-accent);
  --tts-fg-muted: var(--text-secondary);
  --tts-icon-fg: var(--text-primary);
  --tts-icon-hover-bg: var(--hover-bg);
  --tts-thumb-bg: var(--bg-primary);

  position: absolute;
  left: 0;
  right: 0;
  /* 页面级挂载可覆盖（NovelEditorPage 置 64px 避开章节导航条） */
  bottom: var(--tts-bar-bottom, 12px);
  display: flex;
  justify-content: center;
  pointer-events: none;
  z-index: 6;
}

.pill {
  position: relative;
  pointer-events: auto;
  max-width: min(560px, calc(100% - 24px));
  padding-right: 20px;
  padding-left: 20px;
}

.barCore {
  position: relative;
  display: inline-block;
  max-width: 100%;
}

.layersViewport {
  overflow: hidden;
  height: 36px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--tts-bg) 95%, transparent);
  border: 1px solid var(--tts-border);
  box-shadow: 0 4px 18px color-mix(in srgb, #000 18%, transparent);
}

.layersTrack {
  display: flex;
  flex-direction: column;
  transition: transform 0.34s cubic-bezier(0.4, 0, 0.2, 1);
  will-change: transform;
}

.layersTrack--settings {
  transform: translateY(-50%);
}

.layer {
  height: 36px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: opacity 0.28s ease;
}

.layer--hidden {
  opacity: 0.35;
  pointer-events: none;
}

.layerPlaybackInner,
.layerSettingsInner {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  padding: 0 12px;
  gap: 20px;
}

.layerSettingsInner {
  gap: 12px;
  padding: 0 24px;
}

.playSpacer {
  flex-shrink: 0;
  width: 52px;
}

.side {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  min-width: 0;
}

.lbl {
  font-size: 12px;
  color: var(--tts-fg-muted);
  flex-shrink: 0;
}

.layerBtn--stagger {
  transition:
    transform 0.32s cubic-bezier(0.4, 0, 0.2, 1),
    opacity 0.28s ease;
}

/* 层翻转时的 stagger 交错动画（nth-child 与播放层按钮布局对应：1/2/4/5，3 是占位） */
.layersTrack--settings .layerPlayback .layerBtn--stagger:nth-child(1) {
  transition-delay: 0ms;
  transform: translateY(-10px);
  opacity: 0;
}
.layersTrack--settings .layerPlayback .layerBtn--stagger:nth-child(2) {
  transition-delay: 40ms;
  transform: translateY(-10px);
  opacity: 0;
}
.layersTrack--settings .layerPlayback .layerBtn--stagger:nth-child(4) {
  transition-delay: 80ms;
  transform: translateY(-10px);
  opacity: 0;
}
.layersTrack--settings .layerPlayback .layerBtn--stagger:nth-child(5) {
  transition-delay: 120ms;
  transform: translateY(-10px);
  opacity: 0;
}

.layerPlayback .layerBtn--stagger:nth-child(1) {
  transition-delay: 0ms;
}
.layerPlayback .layerBtn--stagger:nth-child(2) {
  transition-delay: 40ms;
}
.layerPlayback .layerBtn--stagger:nth-child(4) {
  transition-delay: 80ms;
}
.layerPlayback .layerBtn--stagger:nth-child(5) {
  transition-delay: 120ms;
}

.side--stagger {
  transition:
    transform 0.32s cubic-bezier(0.4, 0, 0.2, 1),
    opacity 0.28s ease;
}

.layersTrack--settings .layerSettings .side--stagger:first-child {
  transition-delay: 60ms;
  transform: translateY(0);
  opacity: 1;
}
.layersTrack--settings .layerSettings .side--stagger:last-child {
  transition-delay: 120ms;
  transform: translateY(0);
  opacity: 1;
}

.layersTrack:not(.layersTrack--settings)
  .layerSettings
  .side--stagger:first-child {
  transition-delay: 0ms;
  transform: translateY(10px);
  opacity: 0;
}
.layersTrack:not(.layersTrack--settings)
  .layerSettings
  .side--stagger:last-child {
  transition-delay: 60ms;
  transform: translateY(10px);
  opacity: 0;
}

.playPauseBtn.ttsIconBtn {
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  z-index: 3;
  width: 48px;
  height: 48px;
  border-radius: 50%;
  box-shadow: 0 2px 10px color-mix(in srgb, #000 14%, transparent);
  transition:
    background 0.42s cubic-bezier(0.4, 0, 0.2, 1),
    box-shadow 0.42s cubic-bezier(0.4, 0, 0.2, 1);
}

.playPauseBtn.ttsIconBtn :deep(.icon) {
  width: 24px;
  height: 24px;
  color: var(--tts-on-accent);
}

.playPauseBtn.ttsIconBtn :deep(.icon svg) {
  width: 24px;
  height: 24px;
}

.playPauseBtn.ttsIconBtn.playPauseBtn--pause {
  background: var(--tts-danger);
}

.playPauseBtn.ttsIconBtn.playPauseBtn--play {
  background: var(--tts-primary);
}

.playPauseBtn.ttsIconBtn.playPauseBtn--pause:hover:not(:disabled) {
  background: var(--tts-danger-hover);
}

.playPauseBtn.ttsIconBtn.playPauseBtn--play:hover:not(:disabled) {
  background: var(--tts-primary-hover);
}

.layerBtn.ttsIconBtn {
  background: transparent !important;
}

.layerBtn.ttsIconBtn :deep(.icon) {
  width: 16px;
  height: 16px;
  color: var(--tts-icon-fg);
  transition: color 0.16s ease;
}

.layerBtn.ttsIconBtn:hover:not(:disabled) :deep(.icon) {
  color: var(--tts-primary);
}

.layerBtn.ttsIconBtn.layerBtn--stop:hover:not(:disabled) :deep(.icon) {
  color: var(--tts-danger);
}

.layerToggle {
  position: absolute;
  right: 0;
  top: 50%;
  z-index: 4;
  width: 24px;
  height: 24px;
  padding: 0;
  border: none;
  border-radius: 50%;
  background: color-mix(in srgb, var(--tts-primary) 98%, transparent);
  box-shadow: 0 2px 10px color-mix(in srgb, #000 14%, transparent);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transform: translate(50%, -50%);
  transition:
    background 0.16s ease,
    box-shadow 0.16s ease;
}

.layerToggle--left {
  right: auto;
  left: 0;
  transform: translate(-50%, -50%);
}

.layerToggle:hover {
  background: var(--tts-primary-hover);
  box-shadow: 0 3px 12px color-mix(in srgb, #000 18%, transparent);
}

.layerToggleIcon {
  width: 16px;
  height: 16px;
  display: inline-flex;
  color: var(--tts-on-accent);
}

.layerToggleIcon :deep(svg) {
  width: 16px;
  height: 16px;
  display: block;
}

.layerToggleIcon :deep(svg path) {
  fill: currentColor;
}

.rateSlider,
.volumeSlider {
  width: 50px;
}

@media (max-width: 560px) {
  .rateSlider,
  .volumeSlider {
    width: 40px;
  }

  .playPauseBtn.ttsIconBtn {
    width: 44px;
    height: 44px;
  }
}
</style>
