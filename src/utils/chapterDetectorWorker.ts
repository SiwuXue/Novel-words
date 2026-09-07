import { detectChaptersInBatches } from './chapterDetector'
import type { Chapter } from '@/types/novel'

type WorkerMessage =
  | { type: 'progress'; progress: number }
  | { type: 'result'; chapters: Chapter[] }
  | { type: 'error'; message: string }

const WORKER_THRESHOLD = 128 * 1024

export async function detectChaptersOffMainThread(
  text: string,
  onProgress?: (progress: number) => void,
): Promise<Chapter[]> {
  if (text.length < WORKER_THRESHOLD || typeof Worker === 'undefined') {
    return detectChaptersInBatches(text, onProgress)
  }

  return new Promise<Chapter[]>((resolve, reject) => {
    const worker = new Worker(
      new URL('../workers/chapterDetector.worker.ts', import.meta.url),
      { type: 'module' },
    )
    const finish = () => worker.terminate()
    worker.onmessage = (event: MessageEvent<WorkerMessage>) => {
      const message = event.data
      if (message.type === 'progress') {
        onProgress?.(message.progress)
      } else if (message.type === 'result') {
        finish()
        resolve(message.chapters)
      } else {
        finish()
        reject(new Error(message.message))
      }
    }
    worker.onerror = (event) => {
      finish()
      reject(new Error(event.message || '章节解析 Worker 失败'))
    }
    worker.postMessage({ text })
  })
}
