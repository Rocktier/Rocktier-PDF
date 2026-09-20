//! 直链渠道的试用与授权 —— 准则 §12.3 的应用侧实现。
//!
//! 分两半，互相独立：
//!
//! 1. **试用**：首次启动落一个起始时间戳，之后按天算。纯本地、不联网。
//!    ⚠️ 删掉那个文件即可重置 —— 这是**已知且接受**的设计（§12.3 原文"容忍删除重置"）。
//!    它的目的是让愿意付费的人走完付费，不是拦住铁了心白用的人。
//!
//! 2. **回执**：用户在 `/license` 拿到激活码后，粘贴进应用 → 应用向
//!    `rocktier.com/api/activate` 换取一张**服务端签名**的回执 → 之后**离线验签**，
//!    永不联网。
//!
//! **为什么是签名而不是对称密钥**：代码未合并前的计划是"应用用与服务器相同的 secret
//! 本地校验激活码"。那样 secret 必须打进二进制，等于把它交给每一位用户 ——
//! 谁都能反编译取出并自制无限激活码。这里改为客户端只持有**公钥**（泄露无害），
//! 而激活码本身只在服务端校验（见 `rocktier.com/api/activate`）。
//!
//! **注意**：`ENFORCE` 为 `false` 时只记账、不拦截。在激活链路（服务端端点 + 密钥对 +
//! 应用侧引导 UI）全部就绪之前必须保持 `false`，否则会发出一个"7 天后无法解锁、
//! 且没有任何办法解锁"的包。

use std::path::{Path, PathBuf};

/// 试用天数。与 `terms.html` 对公众承诺的 7 天一致（改动须同步条款）。
pub const TRIAL_DAYS: i64 = 7;

/// 是否真的拦截写操作。
///
/// **在激活链路完成前必须保持 `false`。** 翻成 `true` 的那一刻起，试用到期且未激活的
/// 用户将无法保存/导出，因此服务端 `/api/activate`、密钥对与引导 UI 缺一不可。
pub const ENFORCE: bool = false;

/// 服务端签名公钥（Ed25519，base64）。与 `rocktier.com` 环境变量 `LICENSE_PUBLIC_KEY`
/// 同源；可在 `tools/license-keygen.mjs` 生成密钥对时得到。
///
/// 留空表示**尚未配置**：此时任何回执都无法通过校验，授权一律判为无效（安全侧默认）。
pub const PUBLIC_KEY_B64: &str = "";

/// 试用状态的落盘位置（相对于应用数据目录）。文件名故意平淡，不写成 "trial"。
const STATE_FILE: &str = "state.bin";

/// 判定的结果。`days_left` 只用于界面提示，不参与是否放行的判断。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// 试用中，剩余天数（0 表示最后一天还没过完）。
    Trialing { days_left: i64 },
    /// 试用已过期，且没有有效回执。
    Expired,
    /// 已激活；`product` 来自回执（`SQ` 单品 / `FL` 全家桶等）。
    Licensed { product: String },
}

impl Status {
    /// 是否允许写文件（保存、导出、压缩产出、拆分、加密等）。
    ///
    /// 只读操作（打开、翻页、渲染、搜索、读文本、列表单字段）一律不受影响 ——
    /// 用户到期后仍然**可以看**，只是不能**产出新文件**。这是 2026-09-20 用户定调。
    pub fn allows_write(&self, enforce: bool) -> bool {
        if !enforce {
            return true;
        }
        !matches!(self, Status::Expired)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Status::Trialing { .. } => "trial",
            Status::Expired => "expired",
            Status::Licensed { .. } => "licensed",
        }
    }
}

/// 服务端签发的回执内容。字段名与 `rocktier.com/api/activate` 的输出保持一致。
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct Receipt {
    /// 产品码：`SQ` / `WP` / `MD` / `CV` / `FL`。
    pub product: String,
    /// 交易号（`txn_…` / `che_…`），用于人工核对。
    pub txn: String,
    /// 签发时间（Unix 秒）。
    pub issued_at: i64,
}

/// 计算自 `started_at` 起已过的整天数。抽出来是为了能在测试里喂固定时间。
fn elapsed_days(started_at: i64, now: i64) -> i64 {
    // 时间倒退（用户改了系统时钟）时视为 0 天，而不是负数。
    ((now - started_at).max(0)) / 86_400
}

/// 依据起始时间戳与是否有有效回执判定状态。
pub fn status_from(started_at: Option<i64>, receipt: Option<&Receipt>, now: i64) -> Status {
    if let Some(r) = receipt {
        return Status::Licensed { product: r.product.clone() };
    }
    match started_at {
        Some(start) => {
            let used = elapsed_days(start, now);
            if used < TRIAL_DAYS {
                Status::Trialing { days_left: (TRIAL_DAYS - used).max(0) }
            } else {
                Status::Expired
            }
        }
        // 没有起始戳也不该走到这里（`ensure_started` 会先写一个），保险起见按过期处理，
        // 避免"文件被删"反而变成无限试用。
        None => Status::Expired,
    }
}

/// 读取试用起始时间戳。文件不存在或内容损坏都返回 `None`（不 panic —— 这个文件
/// 可能被用户删改，坏掉不该让应用起不来）。
pub fn read_started_at(dir: &Path) -> Option<i64> {
    let raw = std::fs::read_to_string(dir.join(STATE_FILE)).ok()?;
    raw.trim().parse::<i64>().ok()
}

/// 确保存在起始时间戳，没有就写入当前时间并返回它。
pub fn ensure_started(dir: &Path, now: i64) -> Option<i64> {
    if let Some(existing) = read_started_at(dir) {
        return Some(existing);
    }
    std::fs::create_dir_all(dir).ok()?;
    std::fs::write(dir.join(STATE_FILE), now.to_string()).ok()?;
    Some(now)
}

/// 校验回执签名。`signed` 是服务端返回的 `"<base64(receipt_json)>.<base64(signature)>"`。
///
/// 公钥未配置、格式不对、签名不匹配 —— 一律返回 `Err`，绝不放行。
pub fn verify_receipt(signed: &str, public_key_b64: &str) -> Result<Receipt, String> {
    use base64::Engine as _;
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    if public_key_b64.trim().is_empty() {
        return Err("no public key configured".to_string());
    }

    let (payload_b64, sig_b64) = signed
        .split_once('.')
        .ok_or_else(|| "malformed receipt".to_string())?;

    let engine = base64::engine::general_purpose::STANDARD;
    let payload = engine.decode(payload_b64).map_err(|e| format!("bad payload: {e}"))?;
    let sig_bytes = engine.decode(sig_b64).map_err(|e| format!("bad signature: {e}"))?;

    let key_bytes = engine
        .decode(public_key_b64)
        .map_err(|e| format!("bad public key: {e}"))?;
    let key_array: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| "public key must be 32 bytes".to_string())?;
    let key = VerifyingKey::from_bytes(&key_array).map_err(|e| format!("invalid key: {e}"))?;

    let sig = Signature::from_slice(&sig_bytes).map_err(|e| format!("invalid signature: {e}"))?;
    key.verify(&payload, &sig).map_err(|_| "signature does not match".to_string())?;

    serde_json::from_slice::<Receipt>(&payload).map_err(|e| format!("bad receipt body: {e}"))
}

/// 保存回执（连签名一起存，验签时不需要重新联网）。返回落盘路径。
pub fn save_receipt(dir: &Path, signed: &str) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path = dir.join("receipt.txt");
    std::fs::write(&path, signed).map_err(|e| e.to_string())?;
    Ok(path)
}

/// 读取并校验已保存的回执；没有或无效都返回 `None`。
pub fn read_valid_receipt(dir: &Path, public_key_b64: &str) -> Option<Receipt> {
    let signed = std::fs::read_to_string(dir.join("receipt.txt")).ok()?;
    verify_receipt(signed.trim(), public_key_b64).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: i64 = 86_400;
    const T0: i64 = 1_700_000_000;

    #[test]
    fn first_launch_starts_the_clock_and_second_launch_keeps_it() {
        let dir = std::env::temp_dir().join("rt-license-test-start");
        let _ = std::fs::remove_dir_all(&dir);

        let first = ensure_started(&dir, T0).unwrap();
        assert_eq!(first, T0, "首次启动应写入当前时间");
        let second = ensure_started(&dir, T0 + 3 * DAY).unwrap();
        assert_eq!(second, T0, "再次启动不得重置起始时间（否则试用永远不过期）");
    }

    #[test]
    fn trial_counts_down_and_then_expires() {
        assert_eq!(status_from(Some(T0), None, T0), Status::Trialing { days_left: 7 });
        assert_eq!(status_from(Some(T0), None, T0 + 6 * DAY), Status::Trialing { days_left: 1 });
        assert_eq!(status_from(Some(T0), None, T0 + 7 * DAY), Status::Expired);
        assert_eq!(status_from(Some(T0), None, T0 + 99 * DAY), Status::Expired);
    }

    #[test]
    fn a_backwards_clock_does_not_extend_or_break_the_trial() {
        // 用户把系统时间往回调：已用天数按 0 算，不能变成负数（那会得到一个超过 7 天的试用）。
        assert_eq!(status_from(Some(T0), None, T0 - 30 * DAY), Status::Trialing { days_left: 7 });
    }

    #[test]
    fn missing_state_is_not_an_endless_trial() {
        // 起始戳缺失（文件被删坏）时应按过期处理 —— 否则删文件就成了无限试用。
        assert_eq!(status_from(None, None, T0), Status::Expired);
    }

    #[test]
    fn a_valid_receipt_outranks_the_trial_state() {
        let r = Receipt { product: "SQ".into(), txn: "txn_abc".into(), issued_at: T0 };
        assert_eq!(
            status_from(Some(T0), Some(&r), T0 + 99 * DAY),
            Status::Licensed { product: "SQ".into() }
        );
    }

    #[test]
    fn write_is_blocked_only_when_enforcing_and_expired() {
        let expired = Status::Expired;
        let trialing = Status::Trialing { days_left: 3 };
        let licensed = Status::Licensed { product: "SQ".into() };

        assert!(expired.allows_write(false), "未启用拦截时一律放行（当前线上状态）");
        assert!(!expired.allows_write(true), "启用拦截后过期用户不得写文件");
        assert!(trialing.allows_write(true), "试用期内可以写");
        assert!(licensed.allows_write(true), "已激活可以写");
    }

    /// 用固定的测试密钥做一次完整的签/验往返，并确认被篡改的回执会被拒。
    #[test]
    fn receipt_signature_roundtrip_and_tamper_rejection() {
        use base64::Engine as _;
        use ed25519_dalek::{Signer, SigningKey};

        let key = SigningKey::from_bytes(&[7u8; 32]);
        let pub_b64 = base64::engine::general_purpose::STANDARD
            .encode(key.verifying_key().to_bytes());

        let receipt = Receipt { product: "FL".into(), txn: "che_xyz".into(), issued_at: T0 };
        let payload = serde_json::to_vec(&receipt).unwrap();
        let sig = key.sign(&payload);
        let engine = base64::engine::general_purpose::STANDARD;
        let signed = format!("{}.{}", engine.encode(&payload), engine.encode(sig.to_bytes()));

        assert_eq!(verify_receipt(&signed, &pub_b64).unwrap(), receipt);

        // 逐字节篡改：改动正文里任意一个字节，签名必须对不上。
        let mut flipped = payload.clone();
        flipped[0] ^= 0x01;
        let forged = format!("{}.{}", engine.encode(&flipped), engine.encode(sig.to_bytes()));
        assert!(verify_receipt(&forged, &pub_b64).is_err(), "正文被改动后必须拒绝");

        // 语义篡改：把全家桶（FL）伪造成单品（CV），签名同样必须对不上。
        let mut swapped = receipt.clone();
        swapped.product = "CV".into();
        let swapped_payload = serde_json::to_vec(&swapped).unwrap();
        let forged = format!("{}.{}", engine.encode(&swapped_payload), engine.encode(sig.to_bytes()));
        assert!(verify_receipt(&forged, &pub_b64).is_err(), "签名与内容不匹配时必须拒绝");

        // 没配公钥的时候任何回执都不能通过。
        assert!(verify_receipt(&signed, "").is_err());
        assert!(verify_receipt("garbage", &pub_b64).is_err());
    }

    #[test]
    fn saving_and_reading_back_a_receipt() {
        use base64::Engine as _;
        use ed25519_dalek::{Signer, SigningKey};

        let dir = std::env::temp_dir().join("rt-license-test-receipt");
        let _ = std::fs::remove_dir_all(&dir);

        let key = SigningKey::from_bytes(&[3u8; 32]);
        let pub_b64 = base64::engine::general_purpose::STANDARD
            .encode(key.verifying_key().to_bytes());
        let receipt = Receipt { product: "SQ".into(), txn: "txn_1".into(), issued_at: T0 };
        let payload = serde_json::to_vec(&receipt).unwrap();
        let engine = base64::engine::general_purpose::STANDARD;
        let signed = format!("{}.{}", engine.encode(&payload), engine.encode(key.sign(&payload).to_bytes()));

        save_receipt(&dir, &signed).unwrap();
        assert_eq!(read_valid_receipt(&dir, &pub_b64).unwrap(), receipt);
        // 公钥不对时读不回来（回执被换产品/换密钥签的都会落到这里）。
        let other = SigningKey::from_bytes(&[9u8; 32]);
        let other_b64 = engine.encode(other.verifying_key().to_bytes());
        assert!(read_valid_receipt(&dir, &other_b64).is_none());
    }
}
