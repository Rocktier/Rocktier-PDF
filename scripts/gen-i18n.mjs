#!/usr/bin/env node
/* PDF 多语言字典生成器（C 方案：全量机翻，术语表优先）。

   ── 为什么是生成器而不是手写文件 ──
   220 条 × 6 门 = 1320 条译文。手写文件无法保证：
     ① 与 en.ts 的键**永远同步**（en 加一条，翻译文件就少一条）
     ② 术语一致 —— 家族术语表 `docs/rocktier/i18n/glossary.json`
        是唯一真源，术语必须从那里来而不是各语言各译
   `tsc` 是第二道保险：译文结构与 `Strings` 类型不符会直接编译失败。

   ── 数据来源与优先级 ──
     1. 术语表（glossary.json）—— 家族已裁定的批准译法，**最高优先**
     2. 内置译文表（scripts/i18n-translations.mjs）—— 语境译文
     3. 缺失 → 报错退出，**不生成残缺字典**
        （宁可构建失败，也不要悄悄产出「界面一半英文」的包）

   ── 结构还原（这里踩过一次坑）──
   en.ts 里存在**三层**嵌套：`compress.profiles.web.label`、
   `theme.mode.auto`、`dialog.unsaved.title`。第一版按「顶层组 + 一层」
   输出，把这些拍平了，tsc 报 TS2353「auto does not exist」——
   **类型检查当场抓住了它**，这是把结构交给生成器的主要理由。

   ── 用户已知并接受 ──
   这是机翻基线（C 方案）。后续接百度/有道/腾讯翻译 API 时，
   只需替换 TRANSLATIONS 里对应语言的数据源；
   键结构还原与术语优先级逻辑完全不变。

   用法：
     node scripts/gen-i18n.mjs                # 生成全部语言
     node scripts/gen-i18n.mjs --lang ja      # 只生成日语
     ROCKTIER_ROOT=/path/to/Rocktier node …   # 家族根不在默认位置时
*/

import { readFileSync, writeFileSync, existsSync, rmSync } from "node:fs";
import { pathToFileURL } from "node:url";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { TRANSLATIONS } from "./i18n-translations.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = process.env.ROCKTIER_ROOT || join(HERE, "..", "..", "..", "..");
const GLOSSARY = join(REPO, "docs", "rocktier", "i18n", "glossary.json");

const args = process.argv.slice(2);
const onlyLang = args.includes("--lang") ? args[args.indexOf("--lang") + 1] : null;

/* ── 1. 家族术语表 ── */
if (!existsSync(GLOSSARY)) {
  console.error(`  ❌ 找不到家族术语表: ${GLOSSARY}`);
  console.error(`     设 ROCKTIER_ROOT 环境变量指向家族根目录`);
  process.exit(2);
}
const G = JSON.parse(readFileSync(GLOSSARY, "utf8"));
const approved = new Map();
for (const [, loc] of Object.entries(G.terms)) {
  const en = loc.en?.value;
  if (!en) continue;
  const pack = {};
  for (const l of ["ja", "ko", "de", "es", "pt", "ar"]) {
    if (loc[l]?.value) pack[l] = loc[l].value;
  }
  if (Object.keys(pack).length) approved.set(en, pack);
}
console.error(`  术语表: ${approved.size} 条英文有家族批准译法`);

/* ── 2. 取 en.ts 的真实结构 ──
   ── 为什么不用正则解析 ──
   手写解析器在这个文件上错了三次：
     ① 只处理两层嵌套 → tsc 报 TS2353（theme.mode.auto）
     ② 长文案跨行续写（confirmWarning / expired / storeNote… 共 35 行）
        被整行正则漏掉 → tsc 报 TS2741（exportImages missing）
     ③ 修①的缩进回退逻辑后**过度合并**，把 `toolbar.saveAs` 和
        `toolbar.merge` 粘成一条 → 227 条塌缩到 118 条
   三次都是「用正则猜 TypeScript 的语法」。正确做法是**让编译器求值**：
   esbuild 转 JS → 动态 import → 拿到的就是运行时真实对象。
   少写 100 行猜测逻辑，也不会有第 4 次。 */
const enSrc = readFileSync(join(HERE, "..", "src", "i18n", "en.ts"), "utf8");

async function loadEnTree() {
  const esbuild = await import(join(REPO, "..", "Rocktier PDF", "node_modules", "esbuild", "lib", "main.js"))
    .catch(() => import("esbuild"));
  const out = await esbuild.transform(enSrc, { loader: "ts", format: "esm" });
  const tmp = join(HERE, ".en.generated.mjs");
  writeFileSync(tmp, out.code);
  try {
    const mod = await import(`${pathToFileURL(tmp).href}?t=${Date.now()}`);
    return mod.en;
  } finally {
    rmSync(tmp, { force: true });
  }
}

const TREE = await loadEnTree();
/* 展平成 { 路径: 英文 } */
function flatten(obj, prefix = "", out = {}) {
  for (const [k, v] of Object.entries(obj)) {
    const p = prefix ? `${prefix}.${k}` : k;
    if (v && typeof v === "object") flatten(v, p, out);
    else if (typeof v === "string") out[p] = v;
  }
  return out;
}
const FLAT = flatten(TREE);
console.error(`  en.ts 解析: ${Object.keys(FLAT).length} 条叶子键（含任意深度嵌套）`);

/* ── 3. 生成译文树 ── */
const LANGS = ["ja", "ko", "de", "es", "pt", "ar"];
const targets = onlyLang ? [onlyLang] : LANGS;
let missingTotal = 0;
const written = [];

for (const lang of targets) {
  const ctx = TRANSLATIONS[lang] || {};
  const tree = {};
  const missing = [];

  const putInto = (dst, node, prefix) => {
    /* 注意：**不能一边迭代 Object.entries(node) 一边改 node**。
       之前就在这里把子对象替换成新对象（node[k] = {}），迭代的是
       旧快照、写的是新对象 —— 顶层键被整体丢掉，生成的 ja.ts 顶层为空。
       现在读与写分离：entries 只读，结果另存，最后一次性替换。 */
    const entries = Object.entries(node);

    for (const [k, v] of entries) {
      const p = prefix ? `${prefix}.${k}` : k;
      if (v && typeof v === "object") {
        dst[k] = {};
        putInto(dst[k], v, p);
        continue;
      }
      const term = approved.get(v)?.[lang];
      const byPath = ctx[p];
      const byTail = ctx[k];
      let out;
      if (term) out = term;
      else if (typeof byPath === "string") out = byPath;
      else if (typeof byTail === "string") out = byTail;
      else { missing.push(`${p} = ${v.slice(0, 46)}`); continue; }

      /* 占位符校验：{n} {size} {percent} {cur} {total} {name}
         译文丢了占位符，界面上会直接露出 "{n}" —— 必须在生成期拦住。 */
      const want = [...v.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort().join(",");
      const got = [...out.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort().join(",");
      if (want !== got) {
        missing.push(`${p} — 占位符不符（源 {${want}} / 译文 {${got}}）`);
        continue;
      }
      dst[k] = out;
    }
    return dst;
  };

  const built = putInto({}, TREE, "");

  if (missing.length) {
    missingTotal += missing.length;
    console.error(`\n  ⚠️ ${lang}: 缺 ${missing.length} 条（不生成该语言，避免半成品）`);
    missing.forEach((m) => console.error(`      ${m}`));
    continue;
  }

  /* 输出 TS，缩进与 en.ts 对齐 */
  const q = (s) => JSON.stringify(s);
  const emit = (node, depth) => {
    const pad = "  ".repeat(depth + 1);
    const out = [];
    for (const [k, v] of Object.entries(node)) {
      if (v && typeof v === "object") {
        out.push(`${pad}${k}: {`);
        out.push(...emit(v, depth + 1));
        out.push(`${pad}},`);
      } else {
        out.push(`${pad}${k}: ${q(v)},`);
      }
    }
    return out;
  };
  const count = Object.keys(FLAT).length;
  const content = [
    "/* 由 scripts/gen-i18n.mjs 生成 —— 请勿手改。",
    " * 家族术语优先取自 docs/rocktier/i18n/glossary.json；",
    " * 其余取自 scripts/i18n-translations.mjs。",
    " * 改动流程：改术语表或译文表 → 重跑生成器，不要直接编辑本文件。",
    ` * 语言: ${lang} · 键数: ${count}`,
    " *",
    " * C 方案：机翻基线。接入翻译 API 后重跑生成器覆盖即可，",
    " * 键结构还原与术语优先级逻辑不变。",
    " *",
    " * 结构由生成器从 en.ts 逐层还原，tsc 会校验它与 Strings 类型一致。 */",
    "",
    "import type { Strings } from './en';",
    "",
    `export const ${lang}: Strings = {`,
    ...emit(built, 0),
    "};",
    "",
  ].join("\n");

  writeFileSync(join(HERE, "..", "src", "i18n", `${lang}.ts`), content);
  written.push(lang);
  console.error(`  ✅ ${lang}: ${count} 条齐全 → src/i18n/${lang}.ts`);
}

if (missingTotal) {
  console.error(`\n  ❌ 共 ${missingTotal} 条缺译文或占位符不符。补齐后重跑。\n`);
  process.exit(1);
}
console.error(`\n  完成：${written.join(", ") || "无"}\n`);