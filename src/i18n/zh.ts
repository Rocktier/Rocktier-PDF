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
    copyText: '复制文本',
    exportImages: '导出图片',
    imagesToPdf: '图片转 PDF',
    sign: '签名',
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

  find: {
    placeholder: '在文档中查找',
    prev: '上一个',
    next: '下一个',
    close: '关闭',
    none: '无匹配',
  },

  markup: {
    highlight: '高亮',
    underline: '下划线',
    strikeout: '删除线',
    note: '便签',
    hint: '在要标注的文字上拖拽',
    done: '已添加标注',
  },

  form: {
    title: '填写表单',
    empty: '该 PDF 没有可填写的表单字段。',
    loading: '正在读取字段…',
    apply: '应用',
    cancel: '取消',
    done: '表单已更新',
    page: '第 {n} 页',
  },

  security: {
    title: '密码保护',
    mode: '操作',
    set: '设置密码',
    remove: '移除密码',
    password: '密码',
    ownerPassword: '所有者密码（可选）',
    ownerHint: '留空则与上面的密码相同。',
    cancel: '取消',
    run: '保存副本',
    done: '已保存 {name}',
    needPassword: '请输入密码',
    needDoc: '请先将该 PDF 保存到磁盘',
  },

  password: {
    title: '需要密码',
    hint: '该 PDF 已加密。',
    label: '密码',
    open: '打开',
    cancel: '取消',
    wrong: '密码错误，请重试。',
  },

  sign: {
    pick: '选择签名图片',
    hint: '在页面上点击放置签名',
    done: '已放置签名',
  },

  note: {
    title: '添加便签',
    label: '便签内容',
    placeholder: '输入便签内容…',
    cancel: '取消',
    run: '添加',
    done: '已添加便签',
  },

  stamp: {
    title: '页码与水印',
    kind: '类型',
    pageNumbers: '页码',
    watermark: '水印',
    text: '水印文字',
    textPlaceholder: '例如 机密',
    fontSize: '字号',
    margin: '底部边距',
    opacity: '不透明度',
    cancel: '取消',
    run: '应用',
    done: '已应用到每一页',
    needText: '请输入水印文字',
  },

  toast: {
    saved: '已保存到 {name}',
    deleted: '已删除 {count} 页',
    rotated: '已旋转 {count} 页',
    moved: '页面已移动',
    opened: '已打开 {name}',
    revealed: '已在文件管理器中显示',
    copied: '已复制到剪贴板',
    noText: '所选页面没有文本',
    exported: '已导出 {count} 张图片',
    imagesPdf: '已生成 {name}',
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
