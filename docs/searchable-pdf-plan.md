
## MSIX 在 CI 上踩过的两个坑（2026-09-19，各自花掉一轮 15 分钟）

### 坑一：输入文件被 `.gitignore` 目录级排除吞掉

根 `.gitignore` 的 `src-tauri/gen/`（Tauri 标准规则）是**目录级排除**，git 不会进入该目录，
于是 `gen/windows/` 里的打包输入**一个都没被跟踪**。本地看着齐全，CI 全新检出直接报
`bundle.config.json not found`。修法：

```gitignore
src-tauri/gen/*
!src-tauri/gen/windows/
```

**教训**：局部 `.gitignore` 的注释（"Keep bundle.config.json and templates in git"）**不能证明**
文件真在仓库里 —— 要 `git ls-files` 验证。

### 坑二：可执行文件名推导错位

报错 `Executable not found: ...\release\RocktierPDF.exe`。查工具官方 README，exe 名推导顺序是：

| 优先级 | 来源 |
|---|---|
| 1 | `bundle.config.json` 的 **`executableName`** |
| 2 | `tauri.conf.json` 的 **`mainBinaryName`** |
| 3 | `Cargo.toml` 第一个 `[[bin]]` 名（无则 `[package]` 名）|
| 4 | `productName`（最后兜底）|

它落到了第 4 级（`Rocktier PDF` → `RocktierPDF`），而实际产物是 `rocktier-pdf-editor.exe`（Cargo 包名）。

**第一轮修错了**：我把 `executableName` 与 `mainBinaryName` 都写成 `rocktier-pdf-editor`
（= 等于没改），重跑仍然找 `RocktierPDF.exe` —— 说明**锁定的 0.1.29 不认 `executableName`**
（我读的 README 是更新版本的行为）。

**正解是顺着它走**：Tauri 官方文档写明 *"By default, Tauri uses the output binary from cargo;
by setting this, we will rename that"* —— 即 `mainBinaryName` 会**重命名 cargo 产物**。
所以设成工具期望的 `RocktierPDF`，产物名与期望名一致。**不要去改 Cargo 的 bin 名**：那会牵连
`ci.yml` / `release.yml` / `README` / `ARCHITECTURE` 里的 `cargo test --bin rocktier-pdf-editor`。

**教训**：从别的仓库搬打包机制时，**配置字段也要一起搬** —— 我搬了 `gen/windows/`、脚本与依赖，
唯独漏了命名相关的字段，而它恰好是工具用来找产物的那把钥匙。
