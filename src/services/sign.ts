import { invoke } from '@tauri-apps/api/core';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import type { CertInfo, CertMeta, DocumentInfo, VerifyReport } from '../types';

/** 数字签名的全部 IPC 入口。组件不直接 import `@tauri-apps/api`。 */

export function signCapabilities(): Promise<{ engineAvailable: boolean }> {
  return invoke('sign_capabilities');
}

export function listCerts(): Promise<CertInfo[]> {
  return invoke('list_certs');
}

export function getDefaultCert(): Promise<string | null> {
  return invoke('get_default_cert');
}

export function setDefaultCert(name: string): Promise<void> {
  return invoke('set_default_cert', { name });
}

export function allowExpiredCert(name: string): Promise<void> {
  return invoke('allow_expired_cert', { name });
}

export function deleteCert(name: string): Promise<void> {
  return invoke('delete_cert', { name });
}

export function generateCert(name: string): Promise<CertInfo> {
  return invoke('generate_key', { name });
}

export function inspectCertFiles(
  keyData: string,
  certData?: string,
  password?: string
): Promise<CertMeta> {
  return invoke('inspect_cert_files', {
    keyData,
    certData: certData ?? null,
    password: password ?? null,
  });
}

export function importCert(
  name: string,
  keyData: string,
  certData?: string,
  password?: string
): Promise<CertMeta> {
  return invoke('import_cert', {
    name,
    keyData,
    certData: certData ?? null,
    password: password ?? null,
  });
}

export interface SignRequest {
  certId: string;
  output: string;
  /** "llx,lly,urx,ury"，PDF 点、左下原点、未旋转坐标系。 */
  rect: string;
  /** 1-based。 */
  page: number;
  name: string;
  imagePath?: string;
}

export function signPdf(req: SignRequest): Promise<DocumentInfo> {
  return invoke('sign_pdf', {
    certId: req.certId,
    output: req.output,
    rect: req.rect,
    page: req.page,
    name: req.name,
    imagePath: req.imagePath ?? null,
  });
}

/** 验签：`NO_SIGNATURES` / `UNTRUSTED` / `INVALID` 都算正常返回，不是异常。 */
export function verifyPdf(input: string): Promise<VerifyReport> {
  return invoke('verify_pdf', { input });
}

/** 二进制文件逐字节转 latin1 字符串（Rust 侧无损还原），用于证书导入。 */
export function readAsLatin1(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const r = new FileReader();
    r.onload = () => {
      const bytes = new Uint8Array(r.result as ArrayBuffer);
      let s = '';
      for (let i = 0; i < bytes.length; i++) s += String.fromCharCode(bytes[i]);
      resolve(s);
    };
    r.onerror = () => reject(r.error);
    r.readAsArrayBuffer(file);
  });
}

export async function pickSignatureImage(): Promise<string | null> {
  const selected = await openDialog({ filters: [{ name: 'Image', extensions: ['png', 'jpg', 'jpeg'] }] });
  return typeof selected === 'string' ? selected : null;
}
