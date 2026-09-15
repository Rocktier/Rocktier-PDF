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
    sign: 'Sign',
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

  sign: {
    pick: 'Choose a signature image',
    hint: 'Click on the page to place your signature',
    done: 'Signature placed',
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

  toast: {
    saved: 'Saved to {name}',
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
  },

  privacy: {
    badge: 'Offline',
    tooltip: 'This app makes zero network requests.',
  },
};

export type Strings = typeof en;
