import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import type { ReactNode } from 'react';
import { en } from './en';
import { zh } from './zh';

export type Lang = 'en' | 'zh';

const DICTS = { en, zh } as const;
// 命名空间前缀（原值为 PDF Editor 旧名残留）
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

/** 当前语言 → en 兜底 → key。缺键绝不能把 "tool.compress.title" 这种路径直接吐给用户。 */
function resolve(lang: Lang, path: string): string {
  const hit = lookup(DICTS[lang], path) ?? lookup(DICTS.en, path);
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
 * English default, with the user's manual choice the only thing remembered. */
function detectLang(): Lang {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === 'en' || saved === 'zh') return saved;
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
