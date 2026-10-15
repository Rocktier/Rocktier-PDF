/* 家族标准语言表 —— 单一真源。
   2026-10-13 起全家收敛为 en / zh 两门（family.json globalRules.$locales_2026_10_13）：
   非中英语种均为未审校机翻基线，按家族裁定移除；内置多语言架构保留不变，
   将来具备审校能力再逐门加回 —— 加门只改这里。
   语言顺序按「家族首次落地顺序」而非字母序：en 基准，zh 早期主力。
   `endonym`（语言自称）在切换器里显示。 */

export const LOCALES = [
  { code: 'en', endonym: 'English', english: 'English' },
  { code: 'zh', endonym: '中文', english: 'Chinese' },
] as const;

export type LocaleCode = (typeof LOCALES)[number]['code'];

/** @deprecated 保留导出以兼容旧 import；值等同 LOCALES.map(l => l.code）。 */
export const FAMILY_LOCALES: readonly string[] = LOCALES.map((l) => l.code);

/** 目标 locale 是否在家族标准集内。用于构建期过滤导入。 */
export function isFamilyLocale(code: string): code is LocaleCode {
  return (LOCALES as readonly { code: string }[]).some((l) => l.code === code);
}
