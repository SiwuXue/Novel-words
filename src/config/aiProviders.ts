export interface AiProviderPreset {
  id: string
  name: string
  baseUrl: string
  models: string[]
  local?: boolean
}

/**
 * OpenAI-compatible text providers. Model IDs are only starter suggestions;
 * the settings page can fetch the account's current list from `/models` and
 * always allows a custom ID.
 */
export const AI_PROVIDER_PRESETS: AiProviderPreset[] = [
  {
    id: 'openai',
    name: 'OpenAI',
    baseUrl: 'https://api.openai.com/v1',
    models: ['gpt-5.6-luna', 'gpt-5.6-terra', 'gpt-5.6-sol'],
  },
  {
    id: 'deepseek',
    name: 'DeepSeek',
    baseUrl: 'https://api.deepseek.com',
    models: ['deepseek-v4-flash', 'deepseek-v4-pro'],
  },
  {
    id: 'openrouter',
    name: 'OpenRouter',
    baseUrl: 'https://openrouter.ai/api/v1',
    models: [],
  },
  {
    id: 'gemini',
    name: 'Google Gemini',
    baseUrl: 'https://generativelanguage.googleapis.com/v1beta/openai',
    models: [],
  },
  {
    id: 'minimax-cn',
    name: 'MiniMax CN',
    baseUrl: 'https://api.minimaxi.com/v1',
    models: ['MiniMax-M2.7', 'MiniMax-M2.7-highspeed'],
  },
  {
    id: 'dashscope-cn',
    name: '阿里云百炼（中国）',
    baseUrl: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
    models: [],
  },
  {
    id: 'dashscope-global',
    name: 'Alibaba Model Studio (Global)',
    baseUrl: 'https://dashscope-intl.aliyuncs.com/compatible-mode/v1',
    models: [],
  },
  {
    id: 'siliconflow',
    name: '硅基流动 SiliconFlow',
    baseUrl: 'https://api.siliconflow.cn/v1',
    models: [],
  },
  {
    id: 'bigmodel',
    name: '智谱 BigModel',
    baseUrl: 'https://open.bigmodel.cn/api/paas/v4',
    models: [],
  },
  {
    id: 'groq',
    name: 'Groq',
    baseUrl: 'https://api.groq.com/openai/v1',
    models: [],
  },
  {
    id: 'fireworks',
    name: 'Fireworks AI',
    baseUrl: 'https://api.fireworks.ai/inference/v1',
    models: [],
  },
  {
    id: 'together',
    name: 'Together AI',
    baseUrl: 'https://api.together.ai/v1',
    models: [],
  },
  {
    id: 'mistral',
    name: 'Mistral AI',
    baseUrl: 'https://api.mistral.ai/v1',
    models: ['mistral-small-latest', 'mistral-large-latest'],
  },
  {
    id: 'ollama',
    name: 'Ollama（本地）',
    baseUrl: 'http://localhost:11434/v1',
    models: [],
    local: true,
  },
  {
    id: 'lm-studio',
    name: 'LM Studio（本地）',
    baseUrl: 'http://localhost:1234/v1',
    models: [],
    local: true,
  },
  {
    id: 'custom',
    name: '自定义 OpenAI 兼容服务',
    baseUrl: '',
    models: [],
  },
]

export function getAiProvider(id: string): AiProviderPreset {
  return AI_PROVIDER_PRESETS.find((provider) => provider.id === id)
    ?? AI_PROVIDER_PRESETS[AI_PROVIDER_PRESETS.length - 1]
}
