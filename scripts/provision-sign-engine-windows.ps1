# Build the digital-signature sidecar for Windows (x64).
$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot
$OutDir = Join-Path $Root "src-tauri\resources\sign-engine"
$Out = Join-Path $OutDir "sign-engine-windows.exe"

if (-not (Get-Command go -ErrorAction SilentlyContinue)) {
  throw "go not found; install Go 1.27+ first"
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
if (Test-Path $Out) { Remove-Item $Out -Force }

Push-Location (Join-Path $Root "sign-engine")
try {
  $env:CGO_ENABLED = "0"
  $env:GOOS = "windows"
  $env:GOARCH = "amd64"
  go build -trimpath -ldflags "-s -w" -o $Out .
  if ($LASTEXITCODE -ne 0) { throw "go build failed with $LASTEXITCODE" }
} finally {
  Pop-Location
}

if (-not (Test-Path $Out)) { throw "sign-engine build produced no output" }
Write-Host "sign-engine sidecar ready: $Out"
