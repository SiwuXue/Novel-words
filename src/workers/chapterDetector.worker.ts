import { detectChaptersInBatches } from '@/utils/chapterDetector'

type WorkerRequest = { text: string }

const workerScope = self as unknown as {
  onmessage: ((event: MessageEvent<WorkerRequest>) => void) | null
  postMessage: (message: unknown) => void
}

workerScope.onmessage = async (event) => {
  try {
    const chapters = await detectChaptersInBatches(event.data.text, (progress) => {
      workerScope.postMessage({ type: 'progress', progress })
    })
    workerScope.postMessage({ type: 'result', chapters })
  } catch (error) {
    workerScope.postMessage({
      type: 'error',
      message: error instanceof Error ? error.message : String(error),
    })
  }
}

export {}
