# Rocktier PDF Squeeze - 打包 qpdf 为自包含 sidecar (Windows)
#
# 与旧版 provision-gs-windows.ps1 的差异：不再依赖 choco 装 Ghostscript，而是直接
# 下载 qpdf 官方发布的 MSVC 构建（自带 qpdf.exe 与全部 DLL），并按官方提供的
# .sha256 校验哈希后再展开。这样构建结果不随第三方包管理器的最新版本漂移，
# 也避免"随包分发一个包管理器装的、来路不明版本的解释器"。
#
# 用法: powershell -ExecutionPolicy Bypass -File scripts/provision-qpdf-windows.ps1 [-OutDir <dir>]
param(
    [string]$OutDir = (Join-Path $PSScriptRoot "..\src-tauri\resources\qpdf"),
    [string]$Version = "12.4.1"
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$asset = "qpdf-$Version-msvc64.zip"
$base = "https://github.com/qpdf/qpdf/releases/download/v$Version"
$zipUrl = "$base/$asset"
$shaUrl = "$base/qpdf-$Version.sha256"

Write-Host "=============================================="
Write-Host "  Provisioning qpdf (Windows)"
Write-Host "  version: $Version"
Write-Host "  out:     $OutDir"
Write-Host "=============================================="

$tmp = Join-Path $env:TEMP "qpdf-provision-$Version"
if (Test-Path $tmp) { Remove-Item -Recurse -Force $tmp }
New-Item -ItemType Directory -Force -Path $tmp | Out-Null

$zipPath = Join-Path $tmp $asset
$shaPath = Join-Path $tmp "qpdf-$Version.sha256"

Write-Host "  下载 $asset ..."
Invoke-WebRequest -Uri $zipUrl -OutFile $zipPath -UseBasicParsing
Invoke-WebRequest -Uri $shaUrl -OutFile $shaPath -UseBasicParsing

# 校验哈希。官方 .sha256 每行形如 "<hash>  <filename>"，只挑我们下载的那个文件。
$expected = $null
foreach ($line in Get-Content $shaPath) {
    $parts = $line -split '\s+'
    if ($parts.Count -ge 2 -and $parts[-1].TrimStart('*') -eq $asset) {
        $expected = $parts[0].ToLower()
        break
    }
}
if (-not $expected) {
    throw "官方校验文件中找不到 $asset 的哈希，拒绝继续（供应链保护）。"
}
$actual = (Get-FileHash -Algorithm SHA256 -Path $zipPath).Hash.ToLower()
if ($actual -ne $expected) {
    throw "qpdf 下载包哈希不匹配：期望 $expected，实际 $actual"
}
Write-Host "  哈希校验通过: $actual"

Expand-Archive -Path $zipPath -DestinationPath (Join-Path $tmp "x") -Force

# 官方 zip 里 qpdf.exe 与全部 DLL 同处 bin/ —— 整目录复制即可自包含。
$bin = Get-ChildItem -Path (Join-Path $tmp "x") -Recurse -Filter "qpdf.exe" |
    Select-Object -First 1
if (-not $bin) { throw "解压后找不到 qpdf.exe" }

if (Test-Path $OutDir) { Remove-Item -Recurse -Force $OutDir }
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
Copy-Item -Path (Join-Path $bin.Directory.FullName "*") -Destination $OutDir -Recurse -Force

# 自检：能跑起来并报出版本，且版本不低于 11（低于 11 缺 --progress 等选项）。
$exe = Join-Path $OutDir "qpdf.exe"
if (-not (Test-Path $exe)) { throw "复制后找不到 $exe" }
$reported = (& $exe --version | Select-Object -First 1)
Write-Host "  自检: $reported"
if ($reported -notmatch 'qpdf version (\d+)\.') { throw "qpdf --version 输出异常: $reported" }
if ([int]$Matches[1] -lt 11) { throw "qpdf 版本过旧（需 >= 11）: $reported" }

$dllCount = (Get-ChildItem -Path $OutDir -Filter "*.dll" | Measure-Object).Count
Remove-Item -Recurse -Force $tmp
Write-Host ""
Write-Host "qpdf sidecar 就绪: $OutDir"
Write-Host "   可执行文件: qpdf.exe | 动态库: $dllCount | 哈希与版本校验: 通过"
