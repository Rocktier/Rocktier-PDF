#!/usr/bin/env bash
# Rocktier PDF Squeeze - 打包 qpdf 为自包含 sidecar (macOS)
#
# 与旧版 provision-gs-macos.sh 同构：把 Homebrew 安装的 qpdf 及其全部 dylib 依赖
# 集中到 src-tauri/resources/qpdf/，用 @loader_path 重写引用，使其脱离 Homebrew
# 也能运行，最后 ad-hoc 签名。
#
# 与 gs 的差异：qpdf 不读字体/ICC 数据目录、也不往自身目录写缓存，所以没有
# Resource/fonts/iccprofiles 那一整套复制；单文件 + 若干 dylib 即可。
#
# 用法: bash scripts/provision-qpdf-macos.sh [OUT_DIR]
set -euo pipefail

OUT_DIR="${1:-$(cd "$(dirname "$0")/.." && pwd)/src-tauri/resources/qpdf}"
# 12.x 才支持我们要用的全部选项；低于 11 连 --progress 都没有，直接拒绝。
MIN_MAJOR=11

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "缺少 $1，请先安装: brew install qpdf" >&2
    exit 1
  fi
}
require_cmd qpdf
require_cmd otool
require_cmd install_name_tool

QPDF_REAL="$(readlink -f "$(command -v qpdf)")"
VER="$(qpdf --version | head -1 | awk '{print $3}')"
MAJOR="${VER%%.*}"
if [[ "${MAJOR:-0}" -lt "$MIN_MAJOR" ]]; then
  echo "qpdf $VER 过旧，需要 >= $MIN_MAJOR（--optimize-images / --recompress-flate 等）" >&2
  exit 1
fi

echo "=============================================="
echo "  Provisioning qpdf (macOS)"
echo "  qpdf:    $QPDF_REAL"
echo "  version: $VER"
echo "  out:     $OUT_DIR"
echo "=============================================="

rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"

# ── 1. 复制 qpdf 可执行文件 ────────────────────────
cp "$QPDF_REAL" "$OUT_DIR/qpdf"
chmod +x "$OUT_DIR/qpdf"
chmod u+w "$OUT_DIR/qpdf"

# ── 2. 递归归集 dylib 并重写为 @loader_path ────────
ad_hoc_sign() {
  codesign --force --sign - --timestamp=none "$1" 2>/dev/null || true
}

resolve_dep() {
  local raw="$1"
  case "$raw" in
    /opt/homebrew/*|/usr/local/*) echo "$raw" ;;
    @rpath/*|@loader_path/*)
      local name="${raw##*/}"
      for dir in /opt/homebrew/lib /opt/homebrew/opt/*/lib /usr/local/lib /usr/local/opt/*/lib "$OUT_DIR"; do
        if [[ -f "$dir/$name" ]]; then echo "$dir/$name"; return; fi
      done
      ;;
  esac
}

bundle_deps() {
  local file="$1"
  chmod u+w "$file"
  local raw_deps
  raw_deps=$(otool -L "$file" 2>/dev/null | tail -n +2 | awk '{print $1}')
  for raw in $raw_deps; do
    case "$raw" in @loader_path/*) continue ;; esac
    local resolved
    resolved=$(resolve_dep "$raw")
    [[ -z "$resolved" ]] && continue
    local name
    name=$(basename "$resolved")
    install_name_tool -change "$raw" "@loader_path/$name" "$file" 2>/dev/null || true
    if [[ ! -f "$OUT_DIR/$name" ]]; then
      cp -p "$resolved" "$OUT_DIR/$name"
      chmod u+w "$OUT_DIR/$name"
      xattr -c "$OUT_DIR/$name" 2>/dev/null || true
      install_name_tool -id "@loader_path/$name" "$OUT_DIR/$name" 2>/dev/null || true
      bundle_deps "$OUT_DIR/$name"
      ad_hoc_sign "$OUT_DIR/$name"
    fi
  done
}

bundle_deps "$OUT_DIR/qpdf"
ad_hoc_sign "$OUT_DIR/qpdf"

# ── 3. 自检：不再引用 Homebrew 绝对路径 ────────────
# 自包含与否是可判定的，别把它留给"用户机器上跑一下看看"。
if otool -L "$OUT_DIR/qpdf" | tail -n +2 | grep -qE '/opt/homebrew|/usr/local'; then
  echo "打包后仍存在 Homebrew 绝对路径引用，sidecar 不是自包含的：" >&2
  otool -L "$OUT_DIR/qpdf" | tail -n +2 | grep -E '/opt/homebrew|/usr/local' >&2
  exit 1
fi

dylib_count=$(find "$OUT_DIR" -maxdepth 1 -name '*.dylib' | wc -l | tr -d ' ')
echo ""
echo "qpdf sidecar 就绪: $OUT_DIR"
echo "   可执行文件: qpdf | 动态库: ${dylib_count} | 自包含校验: 通过"
