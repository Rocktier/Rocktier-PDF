import type { Strings } from './en';

export const zh: Strings = {
  appName: 'PDF Editor',
  tagline: '编辑 PDF。快、私密、无需联网。',

  toolbar: {
    open: '打开',
    save: '保存',
    saveAs: '另存为',
    merge: '合并',
    split: '拆分',
    extract: '提取',
    rotateLeft: '左转 90°',
    rotateRight: '右转 90°',
    delete: '删除',
    close: '关闭',
    theme: '主题',
    language: '语言',
  },

  empty: {
    title: '拖入 PDF 开始',
    subtitle: '一切都在你的设备上完成。不上传、无账号、不联网。',
    browse: '选择 PDF',
    hint: '或按 Ctrl/Cmd + O',
  },

  rail: {
    pages: '页面',
    page: '页',
  },

  status: {
    ready: '就绪',
    working: '处理中',
    error: '出错',
    unsaved: '未保存的修改',
    saved: '已保存',
    pages: '页',
  },

  merge: {
    title: '合并 PDF',
    add: '添加文件',
    empty: '还没有添加文件。',
    hint: '按显示顺序追加页面。拖拽排序将在 v0.2 提供。',
    output: '另存为',
    cancel: '取消',
    run: '合并',
    done: '已合并为 {name}',
    needTwo: '至少选择两个 PDF 才能合并',
    needOutput: '请选择合并后的保存位置',
    failed: '合并失败',
  },

  split: {
    title: '拆分 PDF',
    mode: '方式',
    everyPage: '每页一个文件',
    everyN: '每 N 页',
    ranges: '自定义范围',
    nPages: '每个文件页数',
    rangesPlaceholder: '例如 1-3, 5, 8-',
    output: '输出文件夹',
    choose: '选择…',
    cancel: '取消',
    run: '拆分',
    done: '已生成 {count} 个文件',
    needOutput: '请选择输出文件夹',
    failed: '没有生成任何文件，请检查页面范围',
  },

  zoom: {
    in: '放大',
    out: '缩小',
    fit: '适应宽度',
    actual: '实际大小',
  },

  toast: {
    saved: '已保存到 {name}',
    deleted: '已删除 {count} 页',
    rotated: '已旋转 {count} 页',
    moved: '页面已移动',
    opened: '已打开 {name}',
    revealed: '已在文件管理器中显示',
  },

  error: {
    openFailed: '无法打开该 PDF。',
    saveFailed: '无法保存 PDF。',
    renderFailed: '无法渲染此页。',
    noDoc: '当前没有打开的文档。',
    pdfiumMissing: '未找到 Pdfium 引擎，请运行 `npm run fetch:pdfium`。',
    passwordProtected: 'v0.1 不支持加密的 PDF。',
    invalidRange: '页面范围无效。',
  },

  privacy: {
    badge: '离线',
    tooltip: '本应用不发起任何网络请求。',
  },
};
