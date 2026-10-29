//! 数字签名：sidecar 编排、证书库、系统钥匙串、旧数据迁移。
//!
//! Go 引擎只做密码学（解析证书 / 增量签名 / 验签 / 生成外观图），
//! 授权、钥匙串、落盘与迁移都在这一层 —— 引擎不碰任何用户数据目录。
//!
//! 与引擎交互的三条纪律（来自引擎侧实现，改这里之前先读一遍）：
//!
//! 1. 一律 `<engine> <subcommand> ...`；需要私钥明文时用 `-key-stdin`，
//!    把 PEM 写进子进程 stdin 后**立刻关闭管道** —— 引擎阻塞在
//!    `io.ReadAll(os.Stdin)` 等 EOF，不关就永远等不到结果。
//! 2. `sign` 不传 `-key <path>`：两者同时给出时 `-key-stdin` 静默胜出，
//!    但同时传就等于把私钥路径也交给 `ps`，没有任何好处。
//!    （`inspect-cert` 是唯一的例外：它只认 `-key`，见 `engine_inspect`。）
//! 3. 引擎只读第一个 PEM 块，所以进钥匙串的一定是引擎 `-key-out` 归一化后的单块 PEM。
//!
//! 退出码不是「有没有出错」的同义词：`verify` 的 4（无签名/无效）与 5（不受信）
//! 都是**正常结论**，必须把 JSON 交给界面；1（usage）与 2（IO）不在契约内，
//! 一律当调用错误 —— 把它们当 verdict 会让界面显示一条根本不存在的签名。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use tauri::Manager as _;

// `de_signers` 里要以关联函数形式调 `Option::deserialize`，故引入 trait。
use serde::Deserialize as _;

#[cfg(target_os = "macos")]
pub const ENGINE_BIN: &str = "sign-engine-macos";
#[cfg(target_os = "windows")]
pub const ENGINE_BIN: &str = "sign-engine-windows.exe";
#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
pub const ENGINE_BIN: &str = "sign-engine-linux";

/// sidecar 超时。签名是本地操作，60s 足够；超时即视为引擎卡死。
pub const ENGINE_TIMEOUT: Duration = Duration::from_secs(60);

/// 私钥在系统安全存储里的 service 名。与 Sign 的 "Rocktier Sign" 分开：
/// 两个 App 是不同二进制，共用 service 会让迁移后的条目归属混乱。
const KEYRING_SERVICE: &str = "Rocktier PDF";

pub fn keyring_service() -> &'static str {
    KEYRING_SERVICE
}

pub fn keyring_user(name: &str) -> String {
    format!("sign-key:{name}")
}

/// 引擎 `inspect-cert` 输出的是 snake_case（`not_before` / `chain_len` …），
/// 前端与磁盘上的 `*.meta.json` 要 camelCase。所以：序列化统一 camelCase，
/// 反序列化用 `alias` 同时认两种写法 —— 少了 alias，来自引擎的字段会被
/// `#[serde(default)]` 静默吃成空串，界面显示「有效期 ——」而不是报错。
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct CertMeta {
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub cn: String,
    #[serde(default)]
    pub issuer: String,
    #[serde(default, alias = "not_before")]
    pub not_before: String,
    #[serde(default, alias = "not_after")]
    pub not_after: String,
    #[serde(default, alias = "key_type")]
    pub key_type: String,
    #[serde(default, alias = "chain_len")]
    pub chain_len: u32,
    #[serde(default)]
    pub expired: bool,
    /// 用户在证书页对已过期证书点过「仍要使用」后为 true。
    #[serde(default)]
    pub allow_expired: bool,
}

#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CertInfo {
    pub name: String,
    pub created: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<CertMeta>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct SignerInfo {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub valid_signature: bool,
    #[serde(default)]
    pub trusted_issuer: bool,
    #[serde(default)]
    pub ext_key_usage_valid: bool,
    #[serde(default)]
    pub revoked: bool,
    #[serde(default)]
    pub signature_time: String,
    #[serde(default)]
    pub timestamp_status: String,
    #[serde(default)]
    pub status: String,
}

/// 引擎在「没有签名」时输出 `"signers": null`（Go 的 nil slice 就是这么序列化的）。
///
/// `#[serde(default)]` 只在字段**缺失**时生效，显式的 `null` 会直接报
/// `invalid type: null, expected a sequence` —— 于是打开一份没签名的 PDF
/// 反而报错。先按 `Option` 收，再折成空数组：前端拿到的永远是数组。
fn de_signers<'de, D>(deserializer: D) -> Result<Vec<SignerInfo>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // 用 `Option` 中转：`null` → None → 空列表。
    Ok(Option::<Vec<SignerInfo>>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct VerifyReport {
    #[serde(default)]
    pub verdict: String,
    #[serde(default)]
    pub exit_code: i32,
    #[serde(default)]
    pub summary: String,
    #[serde(default, deserialize_with = "de_signers")]
    pub signers: Vec<SignerInfo>,
}

/// `inspect-cert` 的失败分类。退出码与文案要一一对上，否则用户看到的是
/// 「引擎拒绝了输入」，却不知道该补密码还是换了张不匹配的证书。
pub enum InspectFail {
    PasswordRequired,
    PasswordWrong,
    KeyMismatch,
    Other(String),
}

pub fn inspect_fail_to_err(f: InspectFail) -> String {
    match f {
        InspectFail::PasswordRequired => "CERT_IMPORT_PASSWORD_REQUIRED".to_string(),
        InspectFail::PasswordWrong => "CERT_IMPORT_PASSWORD_WRONG".to_string(),
        InspectFail::KeyMismatch => {
            "CERT_KEY_MISMATCH: the private key does not match this certificate".to_string()
        }
        InspectFail::Other(e) => e,
    }
}

pub fn map_inspect_exit(code: Option<i32>) -> Result<(), InspectFail> {
    match code {
        Some(0) => Ok(()),
        Some(7) => Err(InspectFail::PasswordRequired),
        Some(8) => Err(InspectFail::PasswordWrong),
        Some(6) => Err(InspectFail::KeyMismatch),
        _ => Err(InspectFail::Other(
            "sign-engine rejected the input".to_string(),
        )),
    }
}

/// 前端把二进制文件按 latin1 逐字节转成字符串过 JSON；这里逐 char 还原。
pub fn latin1_to_bytes(s: &str) -> Vec<u8> {
    s.chars().map(|c| u32::from(c) as u8).collect()
}

/// 打包后 sidecar 在 `<resource>/resources/sign-engine/`；开发时回退到
/// `src-tauri/resources/sign-engine/`。与 pdfium-runtime 的定位逻辑保持一致。
pub fn resolve_engine(dirs: &[PathBuf], name: &str) -> Option<PathBuf> {
    for dir in dirs {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

pub fn engine_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(dir) = app.path().resource_dir() {
        dirs.push(dir.join("resources").join("sign-engine"));
        dirs.push(dir.join("sign-engine"));
    }
    dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/sign-engine"));
    resolve_engine(&dirs, ENGINE_BIN).ok_or_else(|| "SIGN_ENGINE_MISSING".to_string())
}

pub fn cert_store_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app_data_dir: {e}"))?;
    let dir = data_dir.join("certs");
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| format!("create certs dir: {e}"))?;
    }
    Ok(dir)
}

pub fn default_cert_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(cert_store_dir(app)?.join("default.json"))
}

pub fn sanitize_cert_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty()
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains("..")
        || trimmed.starts_with('.')
    {
        return Err("Invalid certificate name".to_string());
    }
    Ok(trimmed.to_string())
}

pub fn read_cert_meta(store: &Path, name: &str) -> Option<CertMeta> {
    let s = fs::read_to_string(store.join(format!("{name}.meta.json"))).ok()?;
    serde_json::from_str(&s).ok()
}

pub fn write_cert_meta(store: &Path, name: &str, meta: &CertMeta) -> Result<(), String> {
    let p = store.join(format!("{name}.meta.json"));
    let json = serde_json::to_string_pretty(meta).map_err(|e| e.to_string())?;
    fs::write(p, json).map_err(|e| e.to_string())
}

pub fn now_rfc3339() -> String {
    chrono::Local::now().to_rfc3339()
}

/// 跑子进程，可选把 `stdin_bytes` 全部写入被调进程的标准输入，并施加超时。
///
/// 引擎输出是一行/一小段 JSON，远小于管道缓冲，不存在「不读 stdout 导致子进程阻塞」
/// 的死锁窗口。
///
/// 超时是硬超时：到点先 kill 再 wait，绝不把控制权交回给一个可能永不退出的子进程。
/// 写 stdin / 等待出错时同样先 kill 再 reap —— `Child` 没有 reap-on-drop，
/// 早期返回前不 wait 就会留下僵尸。
pub fn run_engine_with_stdin(
    mut cmd: Command,
    stdin_bytes: Option<&[u8]>,
    timeout: Duration,
) -> Result<Output, String> {
    cmd.stdin(if stdin_bytes.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    })
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| format!("engine exec: {e}"))?;
    if let Some(bytes) = stdin_bytes {
        // 写失败也照样离开这个块：si 在此 drop == 关闭 stdin，子进程不会因为
        // 还开着写端而卡住，随后的 reap 才能拿到退出状态。
        let write_err = if let Some(mut si) = child.stdin.take() {
            use std::io::Write as _;
            si.write_all(bytes).err()
        } else {
            None
        };
        if let Some(e) = write_err {
            // 典型是 EPIPE：引擎在读完 stdin 前就因参数错误退出了。
            // 真正的原因只在它的 stderr 里，收完再返回。
            return Err(reap_child_after_failure(
                child,
                &format!("engine stdin: {e}"),
            ));
        }
    }

    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            // 子进程已退出：退出码与管道内容由 wait_with_output 收口
            // （std 会把 try_wait 取到的状态缓存下来，不会二次 waitpid）。
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("SIGN_ENGINE_TIMEOUT".to_string());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(e) => {
                return Err(reap_child_after_failure(
                    child,
                    &format!("engine wait: {e}"),
                ))
            }
        }
    }
    child
        .wait_with_output()
        .map_err(|e| format!("engine output: {e}"))
}

/// 子进程提前失败时的统一收口：kill + reap，并把它的 stderr 附到错误文案上。
///
/// - `Child` 没有 reap-on-drop，任何早期返回都必须 wait，否则留下僵尸；
/// - 引擎因 usage / 参数错误立即退出时，控制台里唯一有用的是它的 stderr；
/// - 子进程已退出时 `kill()` 返回 `Err`，忽略即可，随后的
///   `wait_with_output()` 仍会回收它并读出管道里残留的 stderr。
fn reap_child_after_failure(mut child: std::process::Child, context: &str) -> String {
    let _ = child.kill();
    match child.wait_with_output() {
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let text = stderr.trim();
            if text.is_empty() {
                context.to_string()
            } else {
                format!("{context}: {text}")
            }
        }
        Err(_) => context.to_string(),
    }
}

/// 引擎退出码 → 是否「拿到了可用结论」。4（无效/无签名）与 5（不受信）都是
/// 正常结论，必须把 JSON 交给界面；3 及其它才是引擎错误。
pub fn verify_outcome(code: i32) -> Result<(), String> {
    match code {
        0 | 4 | 5 => Ok(()),
        3 => Err("SIGN_ENGINE_ERROR".to_string()),
        other => Err(format!("SIGN_ENGINE_ERROR: exit {other}")),
    }
}

fn run_engine_with_password(cmd: Command, password: Option<&str>) -> Result<Output, String> {
    let pw = password.filter(|p| !p.is_empty());
    run_engine_with_stdin(cmd, pw.map(str::as_bytes), ENGINE_TIMEOUT)
}

/// 导入证书：把用户选的文件交给引擎解析、归一化，再把引擎写出的
/// `meta.json` 读成 [`CertMeta`]。
///
/// 这里是唯一还传 `-key <path>` 的地方 —— `inspect-cert` 没有 `-key-stdin`，
/// 它要的就是一个文件路径。口令若存在则走 `-password-stdin`，不进命令行。
/// `tmp` 由调用方（命令层）创建为 0700 私有目录，本函数只往里写引擎产物。
pub fn engine_inspect(
    engine: &Path,
    key_in: &Path,
    cert_in: Option<&Path>,
    password: Option<&str>,
    tmp: &Path,
) -> Result<CertMeta, InspectFail> {
    let meta_json = tmp.join("meta.json");
    let key_out = tmp.join("out.key");
    let cert_out = tmp.join("out.crt");

    let mut cmd = Command::new(engine);
    cmd.arg("inspect-cert")
        .arg("-key")
        .arg(key_in)
        .arg("-key-out")
        .arg(&key_out)
        .arg("-cert-out")
        .arg(&cert_out)
        .arg("-json-out")
        .arg(&meta_json);
    if let Some(c) = cert_in {
        cmd.arg("-cert").arg(c);
    }
    if password.filter(|p| !p.is_empty()).is_some() {
        cmd.arg("-password-stdin");
    }
    let out = run_engine_with_password(cmd, password).map_err(InspectFail::Other)?;
    map_inspect_exit(out.status.code())?;

    let s = fs::read_to_string(&meta_json)
        .map_err(|e| InspectFail::Other(format!("read meta: {e}")))?;
    serde_json::from_str(&s).map_err(|e| InspectFail::Other(format!("parse meta: {e}")))
}

/// 增量签名。私钥 PEM 只经 stdin 传入，不落盘、不进日志。
#[allow(clippy::too_many_arguments)]
pub fn sign_with_stdin_key(
    engine: &Path,
    input: &str,
    output: &str,
    key_pem: &str,
    cert: &Path,
    name: &str,
    rect: &str,
    page: u32,
    image: Option<&Path>,
) -> Result<(), String> {
    let mut cmd = Command::new(engine);
    cmd.arg("sign")
        .arg("-input")
        .arg(input)
        .arg("-output")
        .arg(output)
        .arg("-key-stdin")
        .arg("-cert")
        .arg(cert)
        .arg("-name")
        .arg(name)
        .arg("-rect")
        .arg(rect)
        .arg("-page")
        .arg(page.to_string());
    if let Some(img) = image {
        cmd.arg("-image").arg(img);
    }
    let out = run_engine_with_stdin(cmd, Some(key_pem.as_bytes()), ENGINE_TIMEOUT)?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// 验签。引擎把报告写在 stdout，退出码只是分类（0/4/5 都有可用的 JSON）。
pub fn verify_document(engine: &Path, input: &str) -> Result<VerifyReport, String> {
    let mut cmd = Command::new(engine);
    cmd.arg("verify").arg(input);
    let out = run_engine_with_stdin(cmd, None, ENGINE_TIMEOUT)?;
    let code = out.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();

    if verify_outcome(code).is_err() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("SIGN_ENGINE_ERROR: {}", stderr.trim()));
    }
    serde_json::from_str(&stdout).map_err(|e| format!("parse verify output: {e}"))
}

/* ── 签名探测（打开时同步调用，必须廉价）──────────────────────────── */

/// 廉价预筛：没有 `/ByteRange` 字节序列的 PDF 一定没有签名，
/// 避免对每个文件都做一次完整解析。
fn bytes_look_signed(bytes: &[u8]) -> bool {
    bytes.windows(10).any(|w| w == b"/ByteRange")
}

/// 只认**带 `/ByteRange` 的字典**：签名的定义就是「一段被字节范围覆盖的签名值」。
/// 光有 `/Type /Sig` 或字符串里出现 "ByteRange" 都不算 —— 前者可能是废弃对象，
/// 后者是在正文里提到这个词。
fn object_is_signature(obj: &lopdf::Object) -> bool {
    match obj {
        lopdf::Object::Dictionary(d) => d.has(b"ByteRange"),
        lopdf::Object::Stream(s) => s.dict.has(b"ByteRange"),
        _ => false,
    }
}

/// 文档是否带密码学签名。只做存在性判断（打开时同步调用），
/// 具体签名条目由验证页按需 `verify` 获取 —— 不阻塞首屏。
pub fn has_signature(path: &str) -> Result<bool, String> {
    let Ok(bytes) = fs::read(path) else {
        return Ok(false);
    };
    if !bytes_look_signed(&bytes) {
        return Ok(false);
    }
    let doc = lopdf::Document::load_mem(&bytes).map_err(|e| e.to_string())?;
    Ok(doc.objects.values().any(object_is_signature))
}

/* ── 从 Rocktier Sign 迁移（一次性）────────────────────────────────── */

#[derive(serde::Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct MigrationReport {
    pub receipt_copied: bool,
    pub certs_copied: usize,
    pub certs_skipped: usize,
    pub keys_copied: usize,
    pub keys_failed: usize,
}

/// 从 Sign 的 app data 迁移回执、证书与私钥。
///
/// 纪律：只读旧目录、只复制；同名不覆盖；私钥迁移失败不算错误（ACL 拒绝在老用户
/// 机器上很常见），证书条目照留，界面提示重新导入。
pub fn migrate_dirs(
    old: &Path,
    new: &Path,
    copy_key: &dyn Fn(&str) -> bool,
) -> Result<MigrationReport, String> {
    let mut report = MigrationReport::default();

    let old_receipt = old.join("receipt.txt");
    let new_receipt = new.join("receipt.txt");
    if old_receipt.is_file() && !new_receipt.exists() {
        fs::create_dir_all(new).map_err(|e| e.to_string())?;
        fs::copy(&old_receipt, &new_receipt).map_err(|e| e.to_string())?;
        report.receipt_copied = true;
    }

    let old_certs = old.join("certs");
    let new_certs = new.join("certs");
    if old_certs.is_dir() {
        fs::create_dir_all(&new_certs).map_err(|e| e.to_string())?;
        let Ok(entries) = fs::read_dir(&old_certs) else {
            return Ok(report);
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "crt" && ext != "json" {
                continue;
            }
            let dest = new_certs.join(name);
            if dest.exists() {
                report.certs_skipped += 1;
                continue;
            }
            if fs::copy(&path, &dest).is_ok() {
                report.certs_copied += 1;
            } else {
                report.certs_skipped += 1;
            }
        }
        // 私钥：逐张 .crt 名字去旧 service 读一次，写进新 service。
        // 注意读的是**新**目录：复制失败的证书根本不该去要私钥，而且迁移前
        // 新目录可能是空的（第一次安装）—— 读旧目录会把跳过的也算进来。
        if let Ok(entries) = fs::read_dir(&new_certs) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("crt") {
                    continue;
                }
                let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                    continue;
                };
                if copy_key(stem) {
                    report.keys_copied += 1;
                } else {
                    report.keys_failed += 1;
                }
            }
        }
    }
    Ok(report)
}

/// 这一轮迁移算不算「跑完了」。抽成纯函数是为了让下面那条断言能在没有
/// `AppHandle` 的情况下测到 —— 判据一旦散进 `migrate_from_sign` 里就只能靠真机了。
///
/// 只有整件事做完才许写标记：私钥欠账（keys_failed > 0）与硬错误同样算没跑完。
pub fn migration_complete(report: &MigrationReport) -> bool {
    report.keys_failed == 0
}

/// 迁移入口：只跑一次，结果写 `<pdf app_data>/migrated-from-sign.json`。
///
/// 标记文件是**唯一**的「跑过没跑过」判据，只有**整件事做完**才写：
/// - `migrate_dirs` 出错 → 不写标记，下次启动整体重试；
/// - **私钥没迁完（keys_failed > 0）→ 同样不写标记**（2026-10-28 最终审查修的洞）。
///   此前只把硬错误当未完成，私钥复制失败照样写标记 —— 于是用户在 macOS
///   钥匙串授权框上点一次「拒绝」，私钥就**永久不再迁移**：证书还在列表里
///   （证书复制在私钥之前），界面什么也不说，直到用户主动去签名才撞上
///   `SIGN_KEY_MISSING`。重跑是安全的：目录复制 copy-only + 同名跳过，
///   已迁好的不会被动；macOS 对「拒绝」会记住选择，后续访问直接失败而不再弹框，
///   所以重试不会变成每次启动骚扰。
/// - 标记本身写失败也照样算跑过（本次已经复制完毕），否则每次启动都会重跑一遍
///   复制与钥匙串读写。
pub fn migrate_from_sign(app: &tauri::AppHandle) {
    let Ok(new_dir) = app.path().app_data_dir() else {
        return;
    };
    let marker = new_dir.join("migrated-from-sign.json");
    if marker.exists() {
        return;
    }
    let Some(parent) = new_dir.parent() else {
        return;
    };
    let old_dir = parent.join("Rocktier.RocktierSign");

    let copy_key = |name: &str| -> bool {
        let Ok(old_entry) = keyring::Entry::new("Rocktier Sign", &keyring_user(name)) else {
            return false;
        };
        let Ok(pem) = old_entry.get_password() else {
            return false;
        };
        let Ok(new_entry) = keyring::Entry::new(KEYRING_SERVICE, &keyring_user(name)) else {
            return false;
        };
        new_entry.set_password(&pem).is_ok()
    };

    // 出错就不写标记：下次启动重试（copy-only、幂等，见迁移测试）。
    let report = match migrate_dirs(&old_dir, &new_dir, &copy_key) {
        Ok(report) => report,
        Err(_) => return,
    };
    // 私钥没迁完同样不算跑过 —— 见本函数文档注释第二条。证书已经复制过去，
    // 重跑时会被 `dest.exists()` 跳过，只有欠着的那几张私钥会再试一次。
    if !migration_complete(&report) {
        return;
    }
    let payload = serde_json::json!({
        "migratedAt": now_rfc3339(),
        "source": old_dir.to_string_lossy(),
        "report": report,
    });
    let _ = fs::write(&marker, payload.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_rejects_escapes() {
        assert!(sanitize_cert_name("work").is_ok());
        assert!(sanitize_cert_name("  work  ").is_ok());
        assert!(sanitize_cert_name("").is_err());
        assert!(sanitize_cert_name("../evil").is_err());
        assert!(sanitize_cert_name("a/b").is_err());
        assert!(sanitize_cert_name("a\\b").is_err());
        assert!(sanitize_cert_name(".hidden").is_err());
    }

    #[test]
    fn inspect_exit_codes_map_to_distinct_failures() {
        assert!(matches!(map_inspect_exit(Some(0)), Ok(())));
        assert!(matches!(
            map_inspect_exit(Some(7)),
            Err(InspectFail::PasswordRequired)
        ));
        assert!(matches!(
            map_inspect_exit(Some(8)),
            Err(InspectFail::PasswordWrong)
        ));
        assert!(matches!(
            map_inspect_exit(Some(6)),
            Err(InspectFail::KeyMismatch)
        ));
        assert!(matches!(map_inspect_exit(Some(2)), Err(InspectFail::Other(_))));
    }

    #[test]
    fn keyring_names_are_stable() {
        assert_eq!(keyring_user("work"), "sign-key:work");
        assert_eq!(keyring_service(), "Rocktier PDF");
    }

    #[test]
    fn latin1_roundtrip_preserves_bytes() {
        let raw: Vec<u8> = vec![0x00, 0x7f, 0x80, 0xff];
        let s: String = raw.iter().map(|b| char::from(*b)).collect();
        assert_eq!(latin1_to_bytes(&s), raw);
    }

    #[test]
    fn verify_report_deserializes_engine_json() {
        let raw = r#"{
      "verdict": "UNTRUSTED",
      "exit_code": 5,
      "summary": "1 signature(s) present and cryptographically valid, but NOT from a trusted issuer",
      "signers": [{
        "name": "Work",
        "valid_signature": true,
        "trusted_issuer": false,
        "ext_key_usage_valid": true,
        "revoked": false,
        "signature_time": "2026-10-28T09:00:00Z",
        "timestamp_status": "none",
        "status": "untrusted"
      }]
    }"#;
        let report: VerifyReport = serde_json::from_str(raw).unwrap();
        assert_eq!(report.verdict, "UNTRUSTED");
        assert_eq!(report.exit_code, 5);
        assert_eq!(report.signers.len(), 1);
        assert!(report.signers[0].valid_signature);
        assert!(!report.signers[0].trusted_issuer);
        assert_eq!(report.signers[0].signature_time, "2026-10-28T09:00:00Z");

        // 序列化给前端时必须是 camelCase。
        let out = serde_json::to_string(&report).unwrap();
        assert!(out.contains("\"validSignature\":true"));
        assert!(out.contains("\"signatureTime\""));
    }

    #[test]
    fn verify_report_tolerates_null_signers() {
        // Go 的 nil slice 序列化成 null：没签名的 PDF 走的就是这条路径。
        let raw = r#"{
      "verdict": "NO_SIGNATURES",
      "exit_code": 4,
      "summary": "This PDF contains no digital signatures.",
      "signers": null
    }"#;
        let report: VerifyReport = serde_json::from_str(raw).unwrap();
        assert_eq!(report.verdict, "NO_SIGNATURES");
        assert!(report.signers.is_empty());
    }

    #[test]
    fn verify_classifies_by_exit_code() {
        assert!(verify_outcome(0).is_ok());
        assert!(verify_outcome(4).is_ok());
        assert!(verify_outcome(5).is_ok());
        assert!(verify_outcome(3).is_err());
        assert!(verify_outcome(2).is_err());
        // 1 = usage：参数写错不是「无签名」这种结论，不能当 verdict 交给界面，
        // 否则界面会把一次调用失败显示成一份 legit 的验签报告。
        assert!(verify_outcome(1).is_err());
    }

    /// 引擎写的是 snake_case（`inspect.go` 的 certMeta），磁盘/前端要 camelCase。
    /// 这条链断掉时 serde 的 `#[serde(default)]` 会静默吞掉字段（界面显示
    /// 「有效期 ——」而不是报错），所以必须用测试锁住两个方向。
    #[test]
    fn cert_meta_reads_engine_snake_case_and_round_trips_camel() {
        let raw = r#"{
      "subject": "CN=Work, O=Rocktier",
      "cn": "Work",
      "issuer": "CN=Work",
      "not_before": "2026-10-01T00:00:00Z",
      "not_after": "2027-10-01T00:00:00Z",
      "key_type": "RSA-2048",
      "key_matches_cert": true,
      "chain_len": 2,
      "expired": false
    }"#;
        let meta: CertMeta = serde_json::from_str(raw).unwrap();
        assert_eq!(meta.cn, "Work");
        assert_eq!(meta.not_before, "2026-10-01T00:00:00Z");
        assert_eq!(meta.not_after, "2027-10-01T00:00:00Z");
        assert_eq!(meta.key_type, "RSA-2048");
        assert_eq!(meta.chain_len, 2);
        assert!(!meta.expired);
        assert!(!meta.allow_expired);

        // 落盘（write_cert_meta）与给前端都必须是 camelCase。
        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("\"notBefore\":\"2026-10-01T00:00:00Z\""));
        assert!(json.contains("\"notAfter\":\"2027-10-01T00:00:00Z\""));
        assert!(json.contains("\"keyType\":\"RSA-2048\""));
        assert!(json.contains("\"chainLen\":2"));

        // camelCase 也要能读回来：第二次启动读的就是自己写的那份。
        let back: CertMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(back.not_before, meta.not_before);
        assert_eq!(back.not_after, meta.not_after);
        assert_eq!(back.key_type, meta.key_type);
        assert_eq!(back.chain_len, meta.chain_len);
    }

    /// 用 `sh -c` 冒充引擎：它不读 stdin 就带着 stderr 退出，写端必然拿到
    /// EPIPE。回归点是「早期返回也要 kill + reap，并把 stderr 带进错误」。
    #[cfg(unix)]
    #[test]
    fn early_exit_surfaces_stderr_without_hanging() {
        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg("echo 'sign-engine: bad usage: unknown flag' >&2; exit 2");

        // 1 MiB 远大于管道缓冲：子进程从不读，写入只可能失败（EPIPE）。
        let payload: Vec<u8> = vec![b'x'; 1024 * 1024];
        let started = Instant::now();
        let err = run_engine_with_stdin(cmd, Some(payload.as_slice()), Duration::from_secs(5))
            .unwrap_err();
        let elapsed = started.elapsed();

        assert!(
            err.starts_with("engine stdin:"),
            "expected the stdin write to fail, got: {err}"
        );
        assert!(
            err.contains("sign-engine: bad usage: unknown flag"),
            "child stderr missing from error: {err}"
        );
        // 写失败后必须立刻收口，而不是等子进程/永远等下去。
        assert!(
            elapsed < Duration::from_secs(10),
            "run_engine_with_stdin hung for {elapsed:?}"
        );
    }

    /// 超时路径：子进程活到远超过 timeout，函数必须按时返回
    /// `SIGN_ENGINE_TIMEOUT`，并且把子进程真正回收掉（不是留成僵尸）。
    #[cfg(unix)]
    #[test]
    fn timeout_kills_and_reaps_child() {
        let dir = std::env::temp_dir().join(format!(
            "rocktier-sign-engine-timeout-{}",
            std::process::id()
        ));
        // 上一次运行残留的 PID 文件会让断言读到过期的 PID（假通过）。
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let pid_file = dir.join("child.pid");

        let mut cmd = Command::new("sh");
        // `exec` 让记录的 PID 就是最终那个 sleep，避免 sh 再 fork 出逃逸的孤儿。
        cmd.arg("-c").arg(format!(
            "echo $$ > '{}'; exec sleep 30",
            pid_file.display()
        ));

        let started = Instant::now();
        // 500ms 而不是生产用的 ENGINE_TIMEOUT：测试必须快；相比 sh 的
        // fork/exec（几毫秒）仍留了两个数量级的余量，不会假红。
        let err = run_engine_with_stdin(cmd, None, Duration::from_millis(500)).unwrap_err();
        let elapsed = started.elapsed();

        assert!(
            err.contains("SIGN_ENGINE_TIMEOUT"),
            "unexpected error: {err}"
        );
        assert!(
            elapsed < Duration::from_secs(10),
            "timeout not honoured, blocked for {elapsed:?}"
        );

        let pid = read_child_pid(&pid_file);
        // `kill -0` 只探测存在性、不发信号。子进程若还在跑或是僵尸（只 kill
        // 没 wait），探测都会成功；返回非 0 说明 PID 已从进程表消失 == 被回收。
        let status = Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .status()
            .unwrap();
        assert!(
            !status.success(),
            "child {pid} still exists after SIGN_ENGINE_TIMEOUT (unreaped)"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    fn read_child_pid(path: &Path) -> u32 {
        // 函数返回前子进程已跑满 timeout（500ms），PID 文件基本一定就绪；
        // 留 1s 重试余量给高负载 CI。
        for _ in 0..50 {
            if let Some(pid) = fs::read_to_string(path)
                .ok()
                .and_then(|s| s.trim().parse().ok())
            {
                return pid;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("child never wrote its pid to {}", path.display());
    }

    /// 预筛的作用是「省钱」而不是「判真」：没有 `/ByteRange` 的文件绝不能被
    /// 当成签名文件，否则每打开一份普通 PDF 都要做一次完整 lopdf 解析。
    #[test]
    fn byte_range_prefilter_is_cheap_and_correct() {
        assert!(!bytes_look_signed(
            b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog >>\nendobj\n"
        ));
        assert!(bytes_look_signed(b"%PDF-1.7\n<< /ByteRange [0 10 20 30] >>\n"));
    }

    /// 只有带 `/ByteRange` 的字典（或流字典）才算签名。表单域字典、正文里
    /// 出现 "ByteRange" 这几个字的字符串都不算 —— 误判会让一份普通 PDF
    /// 以只读方式打开，用户连保存都做不到。
    #[test]
    fn only_byte_range_dictionaries_count_as_signatures() {
        use lopdf::{dictionary, Object};
        let sig = Object::Dictionary(dictionary! {
            "Type" => "Sig",
            "Filter" => "Adobe.PPKLite",
            "ByteRange" => Object::Array(vec![Object::Integer(0), Object::Integer(100)]),
        });
        let form = Object::Dictionary(dictionary! { "Type" => "Annot", "Subtype" => "Widget" });
        let text = Object::String(b"ByteRange".to_vec(), lopdf::StringFormat::Literal);
        assert!(object_is_signature(&sig));
        assert!(!object_is_signature(&form));
        assert!(!object_is_signature(&text));
    }

    /// 迁移的两条硬要求：只复制不覆盖（新目录已有同名证书时旧的不许盖掉）、
    /// 可重跑（第二次什么都不复制）。私钥回调按 .crt 名字逐张走。
    #[test]
    fn migration_copies_receipt_and_certs_without_overwriting() {
        let tmp = std::env::temp_dir().join(format!("rt-mig-{}", std::process::id()));
        let old = tmp.join("Rocktier.RocktierSign");
        let new = tmp.join("Rocktier.RocktierPDF");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(old.join("certs")).unwrap();
        fs::create_dir_all(new.join("certs")).unwrap();
        fs::write(old.join("receipt.txt"), "OLD").unwrap();
        fs::write(old.join("certs/work.crt"), "OLD-CERT").unwrap();
        fs::write(new.join("certs/work.crt"), "NEW-CERT").unwrap();

        // 私钥回调在这里只记账：迁移测试不碰真钥匙串。
        let copy_key = |_name: &str| true;
        let report = migrate_dirs(&old, &new, &copy_key).unwrap();
        assert!(report.receipt_copied);
        assert_eq!(report.certs_copied, 0);
        assert_eq!(report.certs_skipped, 1);
        assert_eq!(
            fs::read_to_string(new.join("certs/work.crt")).unwrap(),
            "NEW-CERT"
        );
        assert_eq!(fs::read_to_string(new.join("receipt.txt")).unwrap(), "OLD");

        // 幂等：第二次跑不再复制任何东西。
        let again = migrate_dirs(&old, &new, &copy_key).unwrap();
        assert!(!again.receipt_copied);
        assert_eq!(again.certs_copied, 0);

        let _ = fs::remove_dir_all(&tmp);
    }

    /// 私钥欠账不许写标记（2026-10-28 最终审查修的洞，见 `migration_complete`）。
    ///
    /// 修之前 `migrate_from_sign` 只看硬错误：私钥复制失败照样写标记，于是用户
    /// 在钥匙串授权框上点一次「拒绝」，那几张私钥就**永久不再迁移** —— 证书还在
    /// 列表里、界面一声不吭，直到用户自己去签名才撞上 `SIGN_KEY_MISSING`。
    /// 这条断言在修复前是红的（那时没有这个判定，keys_failed 不影响返回值）。
    #[test]
    fn migration_is_incomplete_while_any_key_failed() {
        let mut ok = MigrationReport::default();
        ok.keys_copied = 2;
        assert!(migration_complete(&ok), "私钥全迁完就该算跑完");

        let mut partial = MigrationReport::default();
        partial.keys_copied = 1;
        partial.keys_failed = 1;
        assert!(
            !migration_complete(&partial),
            "还有私钥没迁完就不许写标记 —— 否则失败永不重试"
        );

        let mut none = MigrationReport::default();
        none.keys_failed = 3;
        assert!(!migration_complete(&none), "一张都没迁成更不能算跑完");

        // 没有证书、也就没有私钥要迁的空迁移，应当算跑完（不该每次启动都重跑）。
        assert!(migration_complete(&MigrationReport::default()));
    }
}
