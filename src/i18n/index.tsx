import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import type { ReactNode } from 'react';
import { LOCALES, type LocaleCode } from './locales';
import { en } from './en';
import { zh } from './zh';
import { ja } from './ja';
import { ko } from './ko';
import { de } from './de';
import { es } from './es';
import { pt } from './pt';
import { ar } from './ar';

export type Lang = LocaleCode;

const DICTS = { en, zh, ja, ko, de, es, pt, ar } as const;
const STORAGE_KEY = 'rocktier.pdf.lang';

type Vars = Record<string, string | number>;

function lookup(dict: unknown, path: string): string | undefined {
  const value = path.split('.').reduce<unknown>((acc, k) => {
    if (acc && typeof acc === 'object' && k in (acc as object)) {
      return (acc as Record<string, unknown>)[k];
    }
    return undefined;
  }, dict);
  return typeof value === 'string' ? value : undefined;
}

/** 当前语言 → en 兜底 → key。缺键绝不能把 "tool.compress.title" 这种路径露给用户。 */
function resolve(lang: Lang, path: string): string {
  const dict = (DICTS as Record<string, unknown>)[lang];
  const hit = (dict ? lookup(dict, path) : undefined) ?? lookup(DICTS.en, path);
  if (hit === undefined && (import.meta as unknown as { env?: { DEV?: boolean } }).env?.DEV) {
    console.error(`[i18n] missing key: ${path}`);
  }
  return hit ?? path;
}

function interpolate(template: string, vars?: Vars): string {
  if (!vars) return template;
  return template.replace(/\{(\w+)\}/g, (m, name: string) =>
    name in vars ? String(vars[name]) : m
  );
}

interface LangContextValue {
  lang: Lang;
  setLang: (lang: Lang) => void;
  t: (path: string, vars?: Vars) => string;
}

const LangContext = createContext<LangContextValue>({
  lang: 'en',
  setLang: () => {},
  t: (p, v) => interpolate(resolve('en', p), v),
});

/** English is the default: the product targets an international audience.
 *
 * The system locale is deliberately ignored — the family convention is a fixed
 * English default, with the user's manual choice the only thing remembered.
 * This matches `globalRules.defaultLanguage: "en"` in family.json and the
 * "降中文优先级 / 面向海外" ruling. */
function detectLang(): Lang {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    /* 旧版本只存过 en/zh；这里放宽到家族标准集，脏值一律回落 en。 */
    if (saved && (LOCALES as readonly { code: string }[]).some((l) => l.code === saved)) {
      return saved as Lang;
    }
  } catch {
    /* localStorage unavailable — fall through */
  }
  return 'en';
}

export function LanguageProvider({ children }: { children: ReactNode }) {
  const [lang, setLangState] = useState<Lang>(detectLang);

  useEffect(() => {
    document.documentElement.lang = lang;
    try {
      localStorage.setItem(STORAGE_KEY, lang);
    } catch {
      /* ignore */
    }
    /* 切换语言必须重建原生菜单 —— 菜单项文案硬编码在 Rust 侧，
       只换前端字典的话，工具栏是日文、菜单还是英文。
       （L2_i18n 规则：切换语言必须重渲染全部界面，含原生菜单） */
    void invokeRebuildMenu(lang);
  }, [lang]);

  const setLang = useCallback((next: Lang) => setLangState(next), []);

  const t = useCallback(
    (path: string, vars?: Vars) => interpolate(resolve(lang, path), vars),
    [lang]
  );

  const value = useMemo(() => ({ lang, setLang, t }), [lang, setLang, t]);
  return <LangContext.Provider value={value}>{children}</LangContext.Provider>;
}

export function useI18n(): LangContextValue {
  return useContext(LangContext);
}

/** Shorthand: `const t = useT();` */
export function useT(): (path: string, vars?: Vars) => string {
  return useContext(LangContext).t;
}

/** 重建原生菜单（Rust 侧 build_menu）。浏览器/非 Tauri 环境静默跳过。
 *  lang 必须显式传参：这是模块级函数，拿不到组件作用域里的 lang。 */
async function invokeRebuildMenu(lang: Lang): Promise<void> {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('build_menu', { lang });
  } catch {
    /* dev 模式或非 Tauri 环境：忽略 */
  }
}

export { LOCALES };
export type { LocaleCode };