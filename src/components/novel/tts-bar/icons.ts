/**
 * TTS 控制条图标（SVG ?raw 导入 + v-html 注入，fill 由组件 CSS 覆盖为 currentColor）。
 * 移植自 ColorTxt src/renderer/src/assets/。
 */
import playIcon from './assets/play.svg?raw'
import pauseIcon from './assets/pause.svg?raw'
import prevIcon from './assets/prev.svg?raw'
import nextIcon from './assets/next.svg?raw'
import stopIcon from './assets/stop.svg?raw'
import refreshIcon from './assets/refresh.svg?raw'
import settingIcon from './assets/setting.svg?raw'
import speedIcon from './assets/speed.svg?raw'

export const icons = {
  /** 朗读：播放 */
  play: playIcon,
  /** 朗读：暂停 */
  pause: pauseIcon,
  /** 上一句 */
  prev: prevIcon,
  /** 下一句 */
  next: nextIcon,
  /** 停止 */
  stop: stopIcon,
  /** 重新合成 */
  refresh: refreshIcon,
  /** 角色音色设置 */
  setting: settingIcon,
  /** 语速/音量层切换 */
  speed: speedIcon,
} as const
