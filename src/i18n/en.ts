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
    security: 'Password protection',
    compress: 'Compress',
    fillForm: 'Fill form',
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
    whereToFind:
      'Your code was shown on the page right after payment, and is in the purchase email too.',
    privacyNote:
      'Activating sends the code to rocktier.com once and stores the signed reply locally. Nothing else is sent.',
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
  },

  error: {
    openFailed: 'Could not open that PDF.',
    saveFailed: 'Could not save the PDF.',
    renderFailed: 'Could not render this page.',
    noDoc: 'No document is open.',
    pdfiumMissing: 'Pdfium engine not found. Run `npm run fetch:pdfium`.',
    passwordProtected: 'Encrypted PDFs are not supported in v0.1.',
    invalidRange: 'That page range is not valid.',
    // ── 以下由 services/errorText.ts 映射 Rust 侧的英文错误原文 ──
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
};

export type Strings = typeof en;
