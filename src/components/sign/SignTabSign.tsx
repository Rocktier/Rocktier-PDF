import { useEffect, useMemo, useRef, useState } from 'react';
import { useT } from '../../i18n';
import { localizeError } from '../../services/errorText';
import { allowExpiredCert, pickSignatureImage, signPdf } from '../../services/sign';
import type { CertInfo, DocumentInfo } from '../../types';
import type { SignPlacement } from '../SignDialog';

/** 默认签名外观尺寸（PDF 点）。与引擎默认值一致，避免签完位置漂移。 */
const SIG_W = 160;
const SIG_H = 50;

interface Props {
  doc: DocumentInfo;
  placement: SignPlacement | null;
  /** 证书列表唯一来源是父组件 SignDialog（tab 常挂载下自取只会跑一次，会看到过期列表）。 */
  certs: CertInfo[];
  defaultCert: string | null;
  busy: boolean;
  setBusy: (busy: boolean) => void;
  onRequestPlacement: () => void;
  onSaveFirst: () => Promise<boolean>;
  onSigned: (info: DocumentInfo) => void;
  onError: (message: string | null) => void;
}

export function SignTabSign({
  doc,
  placement,
  certs,
  defaultCert,
  busy,
  setBusy,
  onRequestPlacement,
  onSaveFirst,
  onSigned,
  onError,
}: Props) {
  const t = useT();
  const [certId, setCertId] = useState<string>('');
  const [signerName, setSignerName] = useState('');
  const [output, setOutput] = useState(() => defaultOutput(doc.path));
  const [imagePath, setImagePath] = useState<string | null>(null);
  // 用户一旦手动改过签署人姓名，切换证书就不再覆盖它。
  const nameEditedRef = useRef(false);

  // 父组件列表刷新后保持选中项自洽：已删除/尚未选中时回落到默认证书，默认不在列表则取第一个。
  useEffect(() => {
    setCertId((prev) =>
      prev && certs.some((c) => c.name === prev)
        ? prev
        : certs.find((c) => c.name === defaultCert)?.name ?? certs[0]?.name ?? ''
    );
  }, [certs, defaultCert]);

  useEffect(() => {
    if (!nameEditedRef.current) setSignerName(certId);
  }, [certId]);

  const rect = useMemo(() => {
    if (!placement) return null;
    const info = doc.pages[placement.page];
    if (!info) return null;
    /* 点击点是签名框左下角；越界时向内收，保证整个框落在页面里。
       尺寸也要跟着收：页面比签名框还窄（宽 < 160pt 或高 < 50pt）时，
       `info.width - SIG_W` 是负数，`Math.max(0, …)` 会把 x 夹到 0，
       而右上角仍是 x + SIG_W —— 框子照样溢出页面。引擎只校验
       llx<urx 与非负（`sign-engine/main.go:324-329`），不替我们裁，
       结果就是一个画到页外的签名域。等比收进来才真正落在页内。 */
    const w = Math.min(SIG_W, info.width);
    const h = Math.min(SIG_H, info.height);
    const x = Math.min(Math.max(placement.x, 0), Math.max(0, info.width - w));
    const y = Math.min(Math.max(placement.y, 0), Math.max(0, info.height - h));
    return `${x.toFixed(2)},${y.toFixed(2)},${(x + w).toFixed(2)},${(y + h).toFixed(2)}`;
  }, [doc.pages, placement]);

  /** 签名调用本体 —— 过期确认后的重试也走这里，避免重复参数拼装。 */
  const callSign = (placement: SignPlacement, rect: string): Promise<DocumentInfo> =>
    signPdf({
      certId,
      output,
      rect,
      page: placement.page + 1,
      name: signerName.trim() || certId,
      imagePath: imagePath ?? undefined,
    });

  const submit = async () => {
    onError(null);
    if (doc.dirty) {
      onError(t('dsign.sign.dirty'));
      return;
    }
    if (!rect || !placement) {
      onError(t('dsign.sign.needPlacement'));
      return;
    }
    if (!certId) {
      onError(t('dsign.sign.needCert'));
      return;
    }
    setBusy(true);
    try {
      let info: DocumentInfo;
      try {
        info = await callSign(placement, rect);
      } catch (e) {
        const msg = localizeError(String(e), t);
        // 过期证书：Rust 返回 CERT_EXPIRED_CONFIRM|{cert_id}|{not_after}，
        // 前端确认后放行并重试一次（errorText 不映射它，Task 11 才接）。
        if (!msg.startsWith('CERT_EXPIRED_CONFIRM|')) throw e;
        const [, expiredCertId, notAfter] = msg.split('|');
        if (!expiredCertId) throw e;
        const confirmed = window.confirm(
          t('dsign.sign.expiredConfirm', { date: notAfter ?? '' })
        );
        if (!confirmed) {
          /* 用户点了「取消」：错误框里绝不能回填 `msg` —— 按设计它就是不翻译的原始
             线路串 `CERT_EXPIRED_CONFIRM|{证书}|{日期}`（映射掉会断掉上面那段 startsWith
             匹配，见 services/errorText.ts）。复用过期的问句（en/zh 都已有 {date} 变量），
             不新增文案、不加映射表条目。重试路径原样不动。 */
          onError(t('dsign.sign.expiredConfirm', { date: notAfter ?? '' }));
          return;
        }
        await allowExpiredCert(expiredCertId);
        info = await callSign(placement, rect);
      }
      onSigned(info);
    } catch (e) {
      onError(localizeError(String(e), t));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div>
      <div className="field">
        <label>{t('dsign.sign.cert')}</label>
        <select className="input" value={certId} onChange={(e) => setCertId(e.target.value)}>
          {certs.map((c) => (
            <option key={c.name} value={c.name}>
              {c.name}
              {c.meta?.expired ? ` · ${t('dsign.cert.expired')}` : ''}
            </option>
          ))}
        </select>
      </div>

      <div className="field">
        <label>{t('dsign.sign.signer')}</label>
        <input
          className="input"
          value={signerName}
          onChange={(e) => {
            nameEditedRef.current = true;
            setSignerName(e.target.value);
          }}
        />
      </div>

      <div className="field">
        <label>{t('dsign.sign.page')}</label>
        <div className="row">
          <button className="btn" disabled={busy} onClick={onRequestPlacement}>
            {placement ? t('dsign.sign.replacePlacement') : t('dsign.sign.place')}
          </button>
          <span className="muted">
            {placement
              ? t('dsign.sign.placed', { page: placement.page + 1 })
              : t('dsign.sign.placementHint')}
          </span>
        </div>
      </div>

      <div className="field">
        <label>{t('dsign.sign.output')}</label>
        <input className="input" value={output} onChange={(e) => setOutput(e.target.value)} />
      </div>

      <div className="field">
        <label>{t('dsign.sign.appearance')}</label>
        <div className="row">
          <button
            className="btn"
            onClick={() =>
              void pickSignatureImage().then((p) => {
                if (p) setImagePath(p);
              })
            }
          >
            {t('dsign.sign.pickImage')}
          </button>
          <span className="muted">{imagePath ?? t('dsign.sign.noImage')}</span>
        </div>
      </div>

      {doc.dirty ? (
        <div className="error">
          {t('dsign.sign.dirty')}{' '}
          <button
            className="btn btn-sm"
            onClick={() =>
              void onSaveFirst().then((ok) => {
                if (ok) onError(null);
              })
            }
          >
            {t('dsign.sign.saveFirst')}
          </button>
        </div>
      ) : null}

      <p className="muted">{t('dsign.sign.note')}</p>

      <div className="row">
        <button
          className="btn btn-primary"
          disabled={busy || doc.dirty}
          onClick={() => void submit()}
        >
          {t('dsign.sign.submit')}
        </button>
      </div>
    </div>
  );
}

function defaultOutput(path: string): string {
  if (!path) return 'signed.pdf';
  const slash = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'));
  const dir = path.slice(0, slash + 1);
  const base = path.slice(slash + 1).replace(/\.pdf$/i, '');
  return `${dir}${base}-signed.pdf`;
}
