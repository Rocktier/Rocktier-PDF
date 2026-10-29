export const en = {
  appName: 'PDF Editor',
  tagline: 'Edit PDFs. Fast, private, no internet needed.',

  toolbar: {
    open: 'Open',
    save: 'Save',
    saveAs: 'Save as',
    merge: 'Merge',
    split: 'Split',
    extract: 'Extract',
    rotateLeft: 'Rotate left',
    rotateRight: 'Rotate right',
    delete: 'Delete',
    close: 'Close',
    theme: 'Theme',
    language: 'Language',
    copyText: 'Copy text',
    exportImages: 'To images',
    imagesToPdf: 'Images to PDF',
    sign: 'Stamp',
    digitalSign: 'Digital signature',
    security: 'Password protection',
    compress: 'Compress',
    fillForm: 'Fill form',
    markdown: 'To Markdown',
    stamp: 'Page numbers',
    extractPages: 'Extract pages',
    undo: 'Undo',
    redo: 'Redo',
  },

  // 主题按钮三态文案（家族 §6.5）。title = 「Theme · Follow system」
  theme: {
    mode: {
      auto: 'Follow system',
      light: 'Light',
      dark: 'Dark',
    },
  },

  empty: {
    title: 'Drop a PDF to begin',
    subtitle: 'Everything happens on your device. No uploads, no accounts, no internet.',
    browse: 'Choose a PDF',
    hint: 'or press Ctrl/Cmd + O',
    recent: 'Recent',
    clear: 'Clear',
  },

  rail: {
    pages: 'Pages',
    page: 'Page',
  },

  status: {
    ready: 'Ready',
    working: 'Working',
    error: 'Error',
    unsaved: 'Unsaved changes',
    saved: 'Saved',
    pages: 'pages',
  },

  merge: {
    title: 'Merge PDFs',
    add: 'Add files',
    empty: 'No files added yet.',
    hint: 'Pages are appended in the order shown. Drag is coming in v0.2.',
    output: 'Save as',
    cancel: 'Cancel',
    run: 'Merge',
    done: 'Merged into {name}',
    needTwo: 'Pick at least two PDFs to merge',
    needOutput: 'Choose where to save the merged file',
    failed: 'Merge failed',
  },

  split: {
    title: 'Split PDF',
    mode: 'Mode',
    everyPage: 'Every page → its own file',
    everyN: 'Every N pages',
    ranges: 'Custom ranges',
    nPages: 'Pages per file',
    rangesPlaceholder: 'e.g. 1-3, 5, 8-',
    output: 'Output folder',
    choose: 'Choose…',
    cancel: 'Cancel',
    run: 'Split',
    done: 'Created {count} files',
    needOutput: 'Choose an output folder',
    failed: 'Nothing was written — check the page range',
  },

  zoom: {
    in: 'Zoom in',
    out: 'Zoom out',
    fit: 'Fit width',
    actual: 'Actual size',
  },

  find: {
    placeholder: 'Find in document',
    prev: 'Previous match',
    next: 'Next match',
    close: 'Close',
    none: 'No matches',
  },

  markup: {
    highlight: 'Highlight',
    underline: 'Underline',
    strikeout: 'Strikeout',
    note: 'Note',
    hint: 'Drag across the text you want to mark up',
    done: 'Markup added',
  },

  redact: {
    toolbar: 'Redact',
    hint: 'Drag rectangles over the text to remove, then apply',
    barCount: '{count} region(s) marked',
    apply: 'Apply redaction',
    cancel: 'Cancel',
    confirmTitle: 'Apply redaction?',
    confirmBody:
      '{count} region(s) will be redacted. The text underneath is permanently deleted from the document — not covered up, deleted.',
    confirmWarningTitle: 'This cannot be undone',
    confirmWarning:
      'The underlying text is permanently removed. After saving, no PDF reader can select, copy, or recover it — not even this app. Undo only works before you save.',
    confirmRun: 'Delete text permanently',
    done: 'Redacted {count} text object(s)',
    crossed: '{count} object(s) extended past a region and were removed entirely.',
    residual: 'Warning: text may still remain in {count} region(s). Verify before sharing.',
  },

  form: {
    title: 'Fill form',
    empty: 'This PDF has no fillable form fields.',
    loading: 'Reading fields…',
    apply: 'Apply',
    cancel: 'Cancel',
    done: 'Form updated',
    page: 'Page {n}',
  },

  security: {
    title: 'Password protection',
    mode: 'Action',
    set: 'Set a password',
    remove: 'Remove password',
    password: 'Password',
    ownerPassword: 'Owner password (optional)',
    ownerHint: 'Leave blank to reuse the password above.',
    cancel: 'Cancel',
    run: 'Save copy',
    done: 'Saved {name}',
    needPassword: 'Enter a password',
    needDoc: 'Save this PDF to disk first',
  },

  password: {
    title: 'Password required',
    hint: 'This PDF is encrypted.',
    label: 'Password',
    open: 'Open',
    cancel: 'Cancel',
    wrong: 'Wrong password — try again.',
  },

  sign: {
    pick: 'Choose a stamp image',
    hint: 'Click on the page to place the stamp',
    done: 'Stamp placed',
    disclaimer: 'This is an image stamp only — not a cryptographic digital signature and carries no legal validity.',
    tooltip: 'Visual stamp only — no cryptographic digital signature.',
  },

  note: {
    title: 'Add a note',
    label: 'Note',
    placeholder: 'Type a note…',
    cancel: 'Cancel',
    run: 'Add',
    done: 'Note added',
  },

  stamp: {
    title: 'Page numbers & watermark',
    kind: 'Type',
    pageNumbers: 'Page numbers',
    watermark: 'Watermark',
    text: 'Watermark text',
    textPlaceholder: 'e.g. CONFIDENTIAL',
    fontSize: 'Font size',
    margin: 'Bottom margin',
    opacity: 'Opacity',
    cancel: 'Cancel',
    run: 'Apply',
    done: 'Applied to every page',
    needText: 'Enter some watermark text',
  },

  license: {
    title: 'License',
    loading: 'Checking…',
    trialLeft: 'Free trial — {days} day(s) left.',
    trialChip: 'Trial · {days}d',
    expiredChip: 'Not activated',
    expired:
      'Your trial has ended. Viewing and searching still work; saving and exporting need a license.',
    revokedNotice:
      'This licence has been revoked — usually after a refund or a payment dispute. Your licence status has been reset. Activate again below to continue, or write to hello@rocktier.com if you believe this is a mistake.',
    licensed: 'Licensed. Thank you.',
    licensedFamily: 'Licensed — family bundle. Every Rocktier app is unlocked.',
    licensedNote: 'This copy is activated. No further checks, and no network access.',
    storeNote:
      'This copy came from the Microsoft Store, so the Store handles the license for it.',
    notConfigured:
      'This build cannot activate a code yet — it carries no verification key. Please write to hello@rocktier.com.',
    codeLabel: 'Activation code',
    codePlaceholder: 'RKT-…',
    activate: 'Activate',
    activating: 'Activating…',
    buy: 'Buy — $9.99',
    close: 'Close',
    invalid: 'That code was not accepted. Check it for a typo — the code is not case-sensitive.',
    wrongProduct:
      'That code belongs to a different Rocktier app. Each app has its own code — or the family bundle, which unlocks all of them.',
    refunded:
      'That code was refunded, so it no longer unlocks anything. If this is a mistake, write to hello@rocktier.com with your order number.',
    offline:
      'Could not reach rocktier.com. Activating needs one connection; after that the app stays offline.',
    deviceLimit:
      'This code has reached its device limit. Deactivate a device at rocktier.com/license, then try again.',
    whereToFind:
      'Your code was shown on the page right after payment, and is in the purchase email too.',
    privacyNote:
      'Activating sends the code to rocktier.com once and stores the signed reply locally. Nothing else is sent.',
  },
  update: {
    check: 'Check for updates',
    checking: 'Checking…',
    upToDate: "You're up to date",
    available: 'Version {v} is available',
    failed: "Couldn't check for updates",
    lastChecked: 'Last checked {date}',
    never: 'Never',
    download: 'Get {v}',
    privacyNote: 'Only checks when you click — the app never checks on its own',
    close: 'Close',
    openReleases: 'Open releases',
    openReleasesHint: 'You can also open the releases page directly.',
  },
  markdown: {
    title: 'Export as Markdown',
    converting: 'Reading text and inferring structure…',
    close: 'Close',
    save: 'Save as .md',
    saving: 'Saving…',
    saveFailed: 'Could not write the Markdown file.',
    emptyDocument:
      'No text could be extracted from this document. It is most likely a scan — OCR is the next step for these.',
    preview: 'Markdown preview',
    stats:
      '{pages} pages · {headings} headings · {paragraphs} paragraphs · {lists} list items · body text about {bodySize}pt',
    tag: { scanned: 'Scan', tables: 'Table', error: 'Error' },
    emptyDocumentWarning:
      'No selectable text anywhere in this document. This is almost certainly a scan, so nothing could be converted. Open it in Rocktier OCR first, then export from there.',
    scannedPages:
      'No selectable text on page(s) {pages}. Scanned pages need OCR; they were skipped.',
    tablesKept:
      'Detected {regions} table-like area(s). Version 1 keeps them as plain text — column alignment is not reconstructed.',
    columnSplitTables:
      'Detected {regions} table-like area(s) and re-ordered the page into columns — table rows may have been split. Please review this page manually.',
  },

  compress: {
    title: 'Compress PDF',
    mode: 'Size / quality',
    profiles: {
      web: { label: 'Smallest (web)', desc: 'Lowest quality. For sending by mail or embedding in a web page.' },
      balanced: { label: 'Balanced', desc: 'Good quality at a much smaller size. The default.' },
      archive: { label: 'Highest quality (archive)', desc: 'Keeps more detail. Use when the document may be printed.' },
    },
    output: 'Save as',
    choose: 'Choose output…',
    noDestination: 'No destination chosen yet.',
    needOutput: 'Choose where to save the compressed copy first.',
    cancel: 'Cancel',
    run: 'Compress',
    busy: 'Compressing…',
    stages: {
      optimize: 'Optimising the document structure…',
      images: 'Re-encoding images…',
      write: 'Writing the new file…',
    },
    failed: 'Compression failed. The original file was not modified.',
    note: 'Writes a new file. Your original is never modified, and the text layer stays exactly as it was.',
  },

  toast: {
    saved: 'Saved to {name}',
    compressed: 'Compressed to {name}',
    deleted: 'Deleted {count} page(s)',
    rotated: 'Rotated {count} page(s)',
    moved: 'Page moved',
    opened: 'Opened {name}',
    revealed: 'Revealed in file manager',
    copied: 'Copied to clipboard',
    noText: 'No text on the selected pages',
    exported: 'Exported {count} image(s)',
    imagesPdf: 'Created {name}',
    // 破折号而不是冒号：见 dsignReady 上面对 family-check 取键正则的说明。
    signed: 'Signed — {name}',
  },

  error: {
    /* 由 services/errorText.ts 把 Rust 侧的英文错误原文映射到这里。
       2026-10-15 删掉了 7 个自 0.x 起就没有任何调用点的键（openFailed /
       saveFailed / renderFailed / noDoc / pdfiumMissing / passwordProtected /
       invalidRange）：自引入 errorText.ts 之后错误统一走「Rust 英文原文 →
       localizeError」，它们从未接进那条链路，留着只会让后来者以为有映射。 */
    noPages: 'This PDF has no pages.',
    noPagesSelected: 'Select at least one page first.',
    cannotDeleteAll: 'A PDF must keep at least one page.',
    nothingToUndo: 'Nothing to undo.',
    nothingToRedo: 'Nothing to redo.',
    pageOutOfRange: 'That page is out of range.',
    noDestination: 'No destination chosen.',
    noteEmpty: 'The note text is empty.',
    passwordEmpty: 'The password must not be empty.',
    alreadyEncrypted: 'This PDF is already encrypted.',
    pickOneImage: 'Pick at least one image.',
    pickTwoPdfs: 'Pick at least two PDFs to merge.',
    selectionTooSmall: 'That selection is too small.',
    redactTooSmall: 'That redaction area is too small.',
    noRedactRegions: 'Draw at least one area to redact.',
    notAPdf: 'That file is not a PDF.',
    truncatedPdf: 'That PDF looks truncated or damaged.',
    /* 数字签名链路（services/errorText.ts 的 SIGN_* 前缀分支与 SIGN_ENGINE_MISSING）。
       CERT_EXPIRED_CONFIRM 不在这里：那是 `码|证书|日期` 三段数据，界面自己解析，
       本地化掉反而会断掉过期证书的确认流。 */
    signEngineMissing: 'The signing engine is missing from this build.',
    signedReadOnly: 'This file is signed and read-only. Save an editable copy first.',
    signDirty: 'Save the document before signing.',
    signOutputIsInput: 'Signing never overwrites the original file. Choose a new output path.',
    signPageOutOfRange: 'That page does not exist.',
    signKeyMissing: 'The private key for this certificate is missing. Re-import the certificate.',
    signEngineTimeout: 'The signing engine did not respond in time. The original file is unchanged.',
    signEngineError: 'The signing engine reported an error. The file is unchanged.',
  },

  privacy: {
    badge: 'Offline',
    tooltip: 'This app makes zero network requests.',
  },

  viewer: {
    label: 'Document pages',
  },

  dialog: {
    unsaved: {
      title: 'Unsaved changes',
      body: '"{name}" has changes that are not saved yet. Save them before continuing?',
      save: 'Save',
      discard: "Don't save",
      cancel: 'Cancel',
    },
  },

  dsign: {
    title: 'Digital signature',
    close: 'Close',
    tab: { certs: 'Certificates', sign: 'Sign', verify: 'Verify' },
    engineMissing: 'The signing engine is missing from this build. Reinstall Rocktier PDF.',
    cert: {
      heading: 'Signing certificates',
      intro:
        'Certificates stay on this device. Private keys live in secure system storage: the Keychain on macOS, Credential Manager on Windows.',
      namePlaceholder: 'Certificate name',
      generate: 'Create self-signed',
      importKey: 'Private key (.pem / .p12 / .pfx)',
      importCert: 'Certificate (optional)',
      password: 'Password',
      preview: 'Check files',
      import: 'Import',
      setDefault: 'Use by default',
      default: 'Default',
      expired: 'Expired',
      allowExpired: 'Use anyway',
      delete: 'Delete',
      deleteConfirm: 'Delete certificate "{name}"? Signatures already made with it stay valid, but you will not be able to sign with it again.',
      empty: 'No certificates yet. Create a self-signed one, or import your own.',
      issuer: 'Issuer',
      validUntil: 'Valid until',
      chain: 'Chain',
      error: {
        passwordRequired: 'This private key is password-protected. Enter the password.',
        passwordWrong: 'Wrong password. Try again.',
        keyMismatch: 'This private key does not match the certificate.',
      },
    },
    sign: {
      cert: 'Certificate',
      signer: 'Signer name',
      page: 'Signature position',
      place: 'Choose position on page',
      replacePlacement: 'Choose another position',
      placementHint: 'Click the page to place the signature.',
      placed: 'Position set: page {page}',
      output: 'Output file',
      appearance: 'Appearance image (optional)',
      pickImage: 'Choose image…',
      noImage: 'None — the signature stays invisible but valid',
      submit: 'Sign',
      dirty: 'Save your changes first — a signature must match the saved file byte for byte.',
      saveFirst: 'Save now',
      note: 'Signing creates a new file and opens it read-only.',
      needPlacement: 'Choose a position on the page first.',
      needCert: 'Select a certificate first.',
      expiredConfirm: 'This certificate expired on {date}. Sign anyway?',
    },
    verify: {
      run: 'Check signatures',
      running: 'Checking…',
      signer: 'Signer',
      time: 'Signed at',
      statusLabel: 'Status',
      trusted: 'Trusted issuer',
      yes: 'Yes',
      no: 'No',
      validNote: 'Every signature is valid and the document is unchanged since signing.',
      untrustedNote:
        'The signature is cryptographically valid, but the certificate is not issued by a trusted authority (self-signed or private CA).',
      invalidNote:
        'At least one signature failed verification. This usually means the document changed after it was signed.',
      noneNote: 'This file contains no digital signatures.',
      status: { valid: 'Valid', invalid: 'Invalid', untrusted: 'Valid, untrusted' },
    },
  },

  /* 已签名文档的状态栏徽标与出路。文案属于「只读的结论」，规则本身在后端
     （ensure_editable / SIGNED_READ_ONLY），界面只显示、不重述。
     英文值里刻意用破折号而不是「冒号紧跟花括号占位符」：family-check 对 PDF 用
     整文件正则取键（注释也扫），那种写法会被读成一个只在 en 出现的键，
     en↔zh 对账直接 FAIL。 */
  dsignReady: {
    signed: 'Signed · read-only',
    saveCopy: 'Save editable copy',
    copyDone: 'Saved an editable copy without the signature — {name}',
  },
};

export type Strings = typeof en;
