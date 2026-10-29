#!/usr/bin/env bash
# 构建数字签名 sidecar（macOS）。产物不入库：CI 与本地都从这里生成。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="$ROOT/src-tauri/resources/sign-engine"
OUT="$OUT_DIR/sign-engine-macos"

if ! command -v go >/dev/null 2>&1; then
  echo "缺少 go，请先安装 Go 1.27+（brew install go）" >&2
  exit 1
fi

case "$(uname -m)" in
  arm64) GOARCH=arm64 ;;
  x86_64) GOARCH=amd64 ;;
  *) echo "不支持的架构: $(uname -m)" >&2; exit 1 ;;
esac

mkdir -p "$OUT_DIR"
rm -f "$OUT"
(
  cd "$ROOT/sign-engine"
  CGO_ENABLED=0 GOOS=darwin GOARCH="$GOARCH" \
    go build -trimpath -ldflags="-s -w" -o "$OUT" .
)

if [ ! -x "$OUT" ]; then
  echo "构建后找不到可执行文件: $OUT" >&2
  exit 1
fi
codesign --force --sign - --timestamp=none "$OUT" 2>/dev/null || true
echo "sign-engine sidecar 就绪: $OUT ($GOARCH)"
