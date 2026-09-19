# Rocktier PDF Squeeze - Windows 构建交接文档

## 任务概述

在 **Windows 电脑** 上搭建完整的构建环境，完成 **Rocktier PDF Squeeze** 的 MSI 与 MSIX 打包，并将产物上传到 GitHub Release。

## 硬性约束

所有环境和依赖 **必须安装在 F 盘**，统一放在 `F:\deps\` 目录下，**禁止装到 C 盘**。具体路径如下：

| 工具                            | 安装路径                                   |
| ----------------------------- | -------------------------------------- |
| **Git**                       | `F:\deps\Git`                          |
| **Node.js**                   | `F:\deps\nodejs`                       |
| **Rust**                      | `F:\deps\Rust`（安装时让 rustup-init 选择此路径） |
| **Cargo**                     | `F:\deps\.cargo`（安装时选择）                |
| **Ghostscript**               | `F:\deps\gs`                           |
| **NSIS**                      | `F:\deps\nsis`                         |
| **Visual Studio Build Tools** | `F:\deps\vsbuildtools`（安装时选择自定义路径）     |
| **项目代码**                      | `F:\deps\Rocktier-PDF-Squeeze`（克隆到此目录） |

> 安装时每一步都手动选择路径，直接装到 `F:\deps\` 下。如果某工具已经在 C 盘默认位置装好了，也可以直接剪切移动过去，然后更新系统 PATH 环境变量指向新路径即可。

***

## 项目信息

- **GitHub 仓库**: `https://github.com/Rocktier/Rocktier-PDF-Squeeze`

- **当前版本**: v1.0.1

- **技术栈**: Tauri v2 (Rust + React TypeScript)

- **构建产物**:

  - MSI 安装包: `src-tauri/target/release/bundle/msi/Rocktier PDF Squeeze_{version}_x64_zh-CN.msi`

  - MSIX 包: `src-tauri/target/msix/`（文件名由 productName 推导，形如 `Rocktier.PDF.Squeeze_{version}.0_x64.msix`，另有 `.msixbundle`）

  - NSIS 安装包(备选): `src-tauri/target/release/bundle/nsis/`

***

## 构建前准备

### 1. 必备工具安装

| 工具                            | 版本要求  | 安装说明                                                                                                  |
| ----------------------------- | ----- | ----------------------------------------------------------------------------------------------------- |
| **Git**                       | 最新    | `winget install Git.Git` 或从国内镜像下载: <https://registry.npmmirror.com/binary.html?path=git-for-windows/> |
| **Node.js**                   | >= 20 | `winget install OpenJS.NodeJS.LTS` 或国内镜像: <https://npmmirror.com/mirrors/node/>                       |
| **Rust**                      | 最新稳定版 | 下载 rustup-init.exe: <https://mirrors.ustc.edu.cn/rust-static/rustup/> (设置国内镜像后运行，详见下方 Rust 镜像配置)      |
| **Visual Studio Build Tools** | 2022  | 以下 C++ 工作负载必须安装                                                                                       |
| **Ghostscript**               | 最新    | 国内镜像: <https://mirrors.ustc.edu.cn/ghostscript/>                                                      |
| **NSIS**                      | 3.x   | 国内镜像: <https://mirrors.ustc.edu.cn/nsis/>                                                             |

### 2. Visual Studio Build Tools 安装细节

安装时需勾选以下工作负载：

- **使用 C++ 的桌面开发** (Desktop development with C++)

  - MSVC v143 - VS 2022 C++ x64/x86 build tools

  - Windows 10/11 SDK

  - C++ CMake tools for Windows

### 3. 国内镜像配置（重要）

安装完工具后，务必先配置以下镜像，否则下载依赖会非常慢：

#### npm 镜像

```powershell
npm config set registry https://registry.npmmirror.com/
```

#### Rust 镜像

创建 `F:\deps\.cargo\config.toml`，写入以下内容：

```toml
[source.crates-io]
replace-with = "mirror"

[source.mirror]
registry = "https://mirrors.ustc.edu.cn/crates.io-index/"
```

#### Rustup 镜像 & 环境变量

运行 rustup-init.exe 前，先设置环境变量指向 F 盘：

```powershell
# Rustup 镜像
$env:RUSTUP_DIST_SERVER = "https://mirrors.ustc.edu.cn/rust-static"
$env:RUSTUP_UPDATE_ROOT = "https://mirrors.ustc.edu.cn/rust-static/rustup"

# 指定 Cargo 和 Rust 安装路径到 F 盘
$env:CARGO_HOME = "F:\deps\.cargo"
$env:RUSTUP_HOME = "F:\deps\Rust"
```

> 建议将以上环境变量添加到系统用户变量，这样后续 `rustup update` 和命令行都能自动识别。

### 4. 硬盘空间与内存要求

| 项目                            | 预计占用                     |
| ----------------------------- | ------------------------ |
| **Git**                       | \~200 MB                 |
| **Node.js**                   | \~300 MB                 |
| **Rust + Cargo**              | \~2 GB (含缓存)             |
| **Visual Studio Build Tools** | \~5-8 GB                 |
| **Ghostscript**               | \~200 MB                 |
| **NSIS**                      | \~50 MB                  |
| **项目依赖 + 构建缓存**               | \~2-3 GB                 |
| **总计**                        | **约 10-15 GB**           |
| **推荐内存**                      | **16 GB+** (8 GB 也可行但较慢) |

***

## 详细构建步骤

### Step 1: 克隆仓库

```powershell
# 先创建并进入 F 盘工作目录
mkdir -p F:\deps
cd F:\deps

# 克隆项目
git clone https://github.com/Rocktier/Rocktier-PDF-Squeeze.git
cd Rocktier-PDF-Squeeze
```

### Step 2: 配置 npm 镜像 & 安装前端依赖

```powershell
# 配置 npm 国内镜像（重要，否则下载慢或失败）
npm config set registry https://registry.npmmirror.com/

# 将 npm 全局路径和缓存也指向 F 盘（避免占用 C 盘）
npm config set prefix "F:\deps\npm-global"
npm config set cache "F:\deps\npm-cache"

# 安装依赖
npm install
```

### Step 3: 配置 Ghostscript 环境变量

Ghostscript 安装后，确保 `gswin64c.exe` 在 `PATH` 中：

```powershell
# 检查是否在 PATH 中
gswin64c --version

# 如果不在，手动添加（路径根据实际安装版本调整）
$env:Path += ";F:\deps\gs\gs10.04.0\bin"
```

### Step 4: 构建 MSI 包

```powershell
npm run tauri:build
```

> 产物位置: `src-tauri/target/release/bundle/msi/` 和 `src-tauri/target/release/bundle/nsis/`

### Step 5: 构建 MSIX 包

```powershell
npm run tauri:windows:build
```

> 产物位置: `src-tauri/target/msix/`
>
> ⚠️ **注意路径**：`tauri-windows-bundle` 的输出**不在** `src-tauri/gen/windows/output/`。
> `src-tauri/gen/windows/` 只放**输入**（`bundle.config.json`、`AppxManifest.xml.template`、`Assets/`），
> 产物落在 `src-tauri/target/msix/`（同时生成 `.msix` 与 `.msixbundle`）。
> 写错这个路径不会报错，但 glob 匹配不到文件，**MSIX 会静默地不被上传/发布**
> （`softprops/action-gh-release` 对匹配不到的模式只跳过并给个警告）。已实测踩过。

### Step 6: 上传到 GitHub Release

```powershell
# 先拉取最新标签
git fetch --tags

# 上传 MSI 包
gh release upload v1.0.1 src-tauri/target/release/bundle/msi/*.msi --clobber

# 上传 MSIX 包
gh release upload v1.0.1 src-tauri/target/msix/*.msix* --clobber

# 上传 NSIS 包（可选）
gh release upload v1.0.1 src-tauri/target/release/bundle/nsis/*.exe --clobber
```

> 需要先安装 GitHub CLI: `winget install GitHub.cli` 并运行 `gh auth login` 登录

***

## 构建配置说明

### 环境变量

构建过程会自动读取以下环境变量（如果设置了的话）：

- `GHOSTSCRIPT_PATH` - 自定义 Ghostscript 路径（可选，不设置则自动从 PATH 查找）

### MSIX 配置

MSIX 配置文件位于 `src-tauri/gen/windows/bundle.config.json`：

```json
{
  "publisher": "CN=Rocktier",
  "publisherDisplayName": "Rocktier",
  "capabilities": {
    "general": ["internetClient"]
  },
  "extensions": {
    "shareTarget": false,
    "fileAssociations": [".pdf"],
    "protocolHandlers": []
  },
  "signing": {
    "pfx": null,
    "pfxPassword": null
  }
}
```

> **注意**: MSIX 签名需要有效的代码签名证书。如果不上架 Microsoft Store，可以跳过签名步骤，但 MSIX 安装时会提示"未签名"警告。

***

## GitHub Actions 自动构建参考

项目已配置 GitHub Actions 自动构建流程（`.github/workflows/build.yml`），每次推送 tag 时自动触发：

1. 在 Ubuntu 上构建 macOS DMG
2. 在 Windows 上构建 MSI
3. 在 Windows 上构建 MSIX
4. 所有产物自动上传到 GitHub Release

如果本地构建遇到问题，可以对比 GitHub Actions 的构建日志排查。

***

## 注意事项

1. **安装路径**: 所有工具必须装到 `F:\deps\` 下，禁止放 C 盘，装错就卸载重装
2. **国内镜像加速**: 配置好 npm 镜像（registry.npmmirror.com）、Cargo 镜像（USTC）、Rustup 镜像后再开始构建，否则下载依赖极慢甚至失败
3. **PATH 路径**: 确保 `gswin64c` 和 `rustc` 在 PATH 中可用（需把 `F:\deps\gs\gs10.04.0\bin` 和 `F:\deps\.cargo\bin` 加入系统 PATH）
4. **Node.js 版本**: 建议使用 LTS 版本（20.x 或 22.x）
5. **Rust 工具链**: 安装后确保 `rustup show` 显示已安装 stable 工具链
6. **长路径支持**: 如果构建时遇到路径过长错误，以管理员身份运行：

   ```powershell
   New-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Control\FileSystem" -Name "LongPathsEnabled" -Value 1 -PropertyType DWORD -Force
   ```
7. **杀毒软件**: 构建过程中可能会被 Windows Defender 或其他杀毒软件拦截，建议临时添加排除目录
8. **代理设置**: 如果所在网络需要代理，设置：

   ```powershell
   $env:HTTP_PROXY = "http://proxy:port"
   $env:HTTPS_PROXY = "http://proxy:port"
   ```

***

## 遇到问题时的排查方向

| 问题               | 排查方向                                              |
| ---------------- | ------------------------------------------------- |
| `npm install` 失败 | 检查网络/代理，尝试 `npm cache clean --force` 后重试          |
| Rust 编译失败        | 运行 `rustup update`，检查 MSVC 工具链是否安装完整              |
| Ghostscript 找不到  | 检查 `gswin64c --version` 是否正常，确认 PATH 包含 gs bin 目录 |
| MSIX 生成失败        | 检查 `bundle.config.json` 配置，确认安装了适当的 Windows SDK   |
| 权限错误             | 以管理员身份运行 PowerShell 或终端                           |
| 构建超时             | 首次构建需要下载大量 crate 依赖，确保网络通畅                        |

***

## 联系方式

Windows 构建过程中遇到任何问题，可对比 GitHub Actions 的构建日志或联系项目维护者。
