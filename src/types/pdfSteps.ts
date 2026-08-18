/**精读版步骤编号：1=Step1 语境背词, 2=Step2 回忆词义, 3=Step3 单词列表 */
export type StepNum = 1 | 2 | 3

export const ALL_STEPS: StepNum[] = [1, 2, 3]

export const STEP_LABELS: Record<StepNum, string> = {
  1: 'Step 1：在语境中背单词',
  2: 'Step 2：看单词回忆词义',
  3: 'Step 3：单词列表',
}

/**把任意输入规范化为合法 StepNum 数组，空/非法时返回 [1,2,3]。
 * 对重复值去重、非 1/2/3 元素过滤、按升序输出。
 */
export function normalizeSteps(value: unknown): StepNum[] {
  if (!Array.isArray(value)) return [...ALL_STEPS]
  const set = new Set<StepNum>()
  for (const v of value) {
    const n = Number(v)
    if (n === 1 || n === 2 || n === 3) set.add(n)
  }
  const arr = [...set].sort((a, b) => a - b)
  return arr.length ? arr : [...ALL_STEPS]
}

/**把合法步骤数组序列化为 app_setting 的 value 字符串（JSON 数组，去重排序）。 */
export function serializeSteps(steps: StepNum[]): string {
  return JSON.stringify([...new Set(steps)].sort((a, b) => a - b))
}
