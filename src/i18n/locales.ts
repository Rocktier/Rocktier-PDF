/* 家族标准语言表 —— 单一真源。
   9 个产品共用这套语言顺序与元数据；新增语言只改这里。
   语言顺序按「家族首次落地顺序」而非字母序：en 基准，zh 早期主力，
   其余为全球扩展。`endonym`（语言自称）在切换器里显示 —— 用户看到
   「Deutsch」比看到「German (de)」更容易找到自己的语言。 */

export const LOCALES = [
  { code: 'en', endonym: 'English', english: 'English' },
  { code: 'zh', endonym: '中文', english: 'Chinese' },
  { code: 'ja', endonym: '日本語', english: 'Japanese' },
  { code: 'ko', endonym: '한국어', english: 'Korean' },
  { code: 'de', endonym: 'Deutsch', english: 'German' },
  { code: 'es', endonym: 'Español', english: 'Spanish' },
  { code: 'pt', endonym: 'Português', english: 'Portuguese' },
  { code: 'ar', endonym: 'العربية', english: 'Arabic' },
] as const;

export type LocaleCode = (typeof LOCALES)[number]['code'];

/** 家族标准语言集（不含 Write 独有的 fr）—— 闸门按这个判定「是否达标」。 */
export const FAMILY_LOCALES: readonly string[] = LOCALES.map((l) => l.code);

/** 目标 locale 是否在家族标准集内。用于构建期过滤导入。 */
export function isFamilyLocale(code: string): code is LocaleCode {
  return (LOCALES as readonly { code: string }[]).some((l) => l.code === code);
}