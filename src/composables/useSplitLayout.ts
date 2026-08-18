/**
 * Reactive split-layout state for the three-column editor page.
 *
 * Two side panels (left = chapter list, right = preview) can be dragged to
 * resize, double-clicked to collapse, and double-clicked again to restore.
 * Widths and the remembered "pre-collapse" values are persisted to
 * localStorage so a refresh keeps the same layout.
 */
import { reactive, onMounted, onBeforeUnmount } from 'vue'

export interface SplitLayoutDefaults {
  left: number
  right: number
  /** Bounds for the draggable range. */
  min: number
  max: number
  /** localStorage key. */
  storageKey: string
}

interface Persisted {
  left: number
  right: number
  leftRestored: number
  rightRestored: number
}

function clamp(n: number, min: number, max: number): number {
  if (n < min) return min
  if (n > max) return max
  return n
}

function loadPersisted(key: string): Persisted | null {
  try {
    const raw = localStorage.getItem(key)
    if (!raw) return null
    const obj = JSON.parse(raw) as Partial<Persisted>
    if (
      typeof obj.left === 'number' &&
      typeof obj.right === 'number' &&
      typeof obj.leftRestored === 'number' &&
      typeof obj.rightRestored === 'number'
    ) {
      return obj as Persisted
    }
  } catch {
    // ignore — fall back to defaults
  }
  return null
}

function savePersisted(key: string, data: Persisted): void {
  try {
    localStorage.setItem(key, JSON.stringify(data))
  } catch {
    // localStorage may be full or disabled — silently ignore
  }
}

export function useSplitLayout(defaults: SplitLayoutDefaults) {
  const state = reactive({
    leftWidth: defaults.left,
    rightWidth: defaults.right,
    /** Snap-to-this values used when restoring from a collapsed state. */
    leftRestored: defaults.left,
    rightRestored: defaults.right,
  })

  function persist() {
    savePersisted(defaults.storageKey, {
      left: state.leftWidth,
      right: state.rightWidth,
      leftRestored: state.leftRestored,
      rightRestored: state.rightRestored,
    })
  }

  function setLeftWidth(n: number) {
    state.leftWidth = clamp(n, 0, defaults.max)
    if (state.leftWidth > 0) state.leftRestored = state.leftWidth
    persist()
  }

  function setRightWidth(n: number) {
    state.rightWidth = clamp(n, 0, defaults.max)
    if (state.rightWidth > 0) state.rightRestored = state.rightWidth
    persist()
  }

  function toggleLeft() {
    if (state.leftWidth > 0) {
      state.leftWidth = 0
    } else {
      state.leftWidth = state.leftRestored || defaults.left
    }
    persist()
  }

  function toggleRight() {
    if (state.rightWidth > 0) {
      state.rightWidth = 0
    } else {
      state.rightWidth = state.rightRestored || defaults.right
    }
    persist()
  }

  /* ----------------------------- Drag handling ----------------------------- */

  let activeDrag: 'left' | 'right' | null = null
  let startX = 0
  let startWidth = 0

  function onMove(e: MouseEvent) {
    if (!activeDrag) return
    const delta = e.clientX - startX
    if (activeDrag === 'left') {
      setLeftWidth(startWidth + delta)
    } else {
      // Right panel: cursor right → shrink
      setRightWidth(startWidth - delta)
    }
  }

  function onUp() {
    if (!activeDrag) return
    activeDrag = null
    document.body.style.cursor = ''
    document.body.style.userSelect = ''
    window.removeEventListener('mousemove', onMove)
    window.removeEventListener('mouseup', onUp)
  }

  function startDrag(side: 'left' | 'right', e: MouseEvent) {
    e.preventDefault()
    activeDrag = side
    startX = e.clientX
    startWidth = side === 'left' ? state.leftWidth : state.rightWidth
    document.body.style.cursor = 'col-resize'
    document.body.style.userSelect = 'none'
    window.addEventListener('mousemove', onMove)
    window.addEventListener('mouseup', onUp)
  }

  function startLeftDrag(e: MouseEvent) {
    startDrag('left', e)
  }
  function startRightDrag(e: MouseEvent) {
    startDrag('right', e)
  }

  function restoreFromStorage() {
    const data = loadPersisted(defaults.storageKey)
    if (!data) return
    state.leftWidth = clamp(data.left, 0, defaults.max)
    state.rightWidth = clamp(data.right, 0, defaults.max)
    state.leftRestored = clamp(data.leftRestored, defaults.min, defaults.max)
    state.rightRestored = clamp(data.rightRestored, defaults.min, defaults.max)
  }

  onMounted(restoreFromStorage)
  onBeforeUnmount(() => {
    if (activeDrag) onUp()
  })

  return {
    state,
    setLeftWidth,
    setRightWidth,
    toggleLeft,
    toggleRight,
    startLeftDrag,
    startRightDrag,
  }
}
