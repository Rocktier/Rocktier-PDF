/**
 * 错误文案本地化 —— 把 Rust 侧返回的英文原文映射为当前语言的措辞。
 *
 * 为什么需要这一层：`commands.rs` 把错误**拍平为英文字符串**再返回（见该文件
 * 顶部注释「Errors are flattened to English strings and shown as-is」），前端
 * `usePdf.run()` 直接把它塞进 `setError`，StatusBar 再 `label = error ?? …` 原样
 * 显示。结果：中文用户全程看英文报错（PDF 23 处 / Journal 14 处 / Compressor 2 处）。
 *
 * 为什么不用错误码改 Rust 侧：那要动 40 处返回点，且改动面比这层大得多；这里
 * 做**纯前端映射**，未知文案回退到原文 —— 新增错误不需要改前端也能正常显示
 * （只是没有本地化），这符合规范「失败必须展示底层真实错误文本」。
 *
 * 匹配策略：先精确匹配整串，再匹配去掉标点/空白后的归一化串。两级足够覆盖
 * 「同一个意思在 Rust 里写了两遍」（实测 PDF 有 `No pages selected` 与
 * `no pages selected` 两种大小写）。
 */

/** Rust 错误原文 → i18n 键（`error.*`）。键须在 en.ts / zh.ts 两侧都存在。 */
const EXACT: Record<string, string> = {
  'This PDF has no pages.': 'error.noPages',
  'This PDF has no pages': 'error.noPages',
  'document has no pages': 'error.noPages',
  'no pages selected': 'error.noPagesSelected',
  'No pages selected': 'error.noPagesSelected',
  'Cannot delete every page': 'error.cannotDeleteAll',
  'Nothing to undo': 'error.nothingToUndo',
  'Nothing to redo': 'error.nothingToRedo',
  'Page index out of range': 'error.pageOutOfRange',
  'Page out of range': 'error.pageOutOfRange',
  'No destination path': 'error.noDestination',
  'Note text is empty': 'error.noteEmpty',
  'Password must not be empty': 'error.passwordEmpty',
  'This PDF is already encrypted': 'error.alreadyEncrypted',
  'Pick at least one image': 'error.pickOneImage',
  'Pick at least two PDFs to merge': 'error.pickTwoPdfs',
  'Selection is too small': 'error.selectionTooSmall',
  'Redaction region is too small': 'error.redactTooSmall',
  'No redaction regions were drawn': 'error.noRedactRegions',
  'missing %PDF- header': 'error.notAPdf',
  'missing %%EOF trailer': 'error.truncatedPdf',
  'ENCRYPTED_IN_PLACE_BLOCKED':
    'This document is password-protected; saving over it would remove the password, so it was refused.',
  // 数字签名引擎没随包安装（`sign.rs::engine_path`）。整串就是这一个码，走精确匹配。
  'SIGN_ENGINE_MISSING': 'error.signEngineMissing',
};

/** 归一化匹配：忽略大小写、标点与多余空白。 */
const NORMALIZED: Record<string, string> = {};
for (const [raw, key] of Object.entries(EXACT)) {
  NORMALIZED[normalize(raw)] = key;
}

function normalize(s: string): string {
  return s
    .toLowerCase()
    .replace(/[.,;:!?'"()]/g, '')
    .replace(/\s+/g, ' ')
    .trim();
}

/**
 * 返回本地化后的错误文案。
 *
 * @param raw    Rust 返回的原始错误串
 * @param lookup i18n 的取值函数（通常是 `t`）；找不到键时返回原串
 */
export function localizeError(raw: string, lookup: (key: string) => string): string {
  if (!raw) return raw;

  // 许可相关错误码走专用文案（前端有专门的处理路径，不该被当成普通错误显示）。
  if (raw === 'LICENSE_EXPIRED' || raw === 'LICENSE_WRONG_PRODUCT') return raw;

  /* 签名链路的错误码后面都跟着细节（`SIGNED_READ_ONLY: this file has…`），整串匹配
     不上 EXACT，所以按前缀匹配。放在 EXACT 查询之前：这些前缀与任何精确键都不冲突，
     顺序只影响可读性。
     刻意**不**映射 `CERT_EXPIRED_CONFIRM|{cert_id}|{not_after}` —— 那是给界面用的
     数据而不是文案：SignTabSign 要靠这段原始串解析出该确认哪张证书。在这里把它
     本地化掉，过期证书确认流就断了（未知串走下面的原样回退，正是我们要的结果）。 */
  if (raw.startsWith('SIGNED_READ_ONLY')) return lookup('error.signedReadOnly');
  if (raw.startsWith('SIGN_DIRTY_DOC')) return lookup('error.signDirty');
  if (raw.startsWith('SIGN_OUTPUT_IS_INPUT')) return lookup('error.signOutputIsInput');
  if (raw.startsWith('SIGN_PAGE_OUT_OF_RANGE')) return lookup('error.signPageOutOfRange');
  if (raw.startsWith('SIGN_KEY_MISSING')) return lookup('error.signKeyMissing');
  if (raw.startsWith('SIGN_ENGINE_TIMEOUT')) return lookup('error.signEngineTimeout');
  /* `SIGN_ENGINE_ERROR: exit N` 是引擎其余失败的统一出口（参数不合法、写不出文件等），
     不映射的话中文界面会整段显示英文原文。细节跟在冒号后面，按前缀匹配。 */
  if (raw.startsWith('SIGN_ENGINE_ERROR')) return lookup('error.signEngineError');

  const key = EXACT[raw] ?? NORMALIZED[normalize(raw)];
  if (!key) return raw; // 未知文案 → 原样显示底层真实错误（规范要求，不隐藏细节）

  const translated = lookup(key);
  // i18n 缺键时它会返回键名本身；那种情况下宁可显示原文。
  return translated && translated !== key ? translated : raw;
}