package main

// inspect-cert —— 外部证书导入的检查与归一化（阶段 69/D-52）。
//
// 输入五种形态：裸 PEM / 口令加密 PEM / PFX·P12 / DER / 含证书链的 PEM。
// 输出三样：归一化未加密私钥 PEM（key-out）、叶子+中间证书 PEM（cert-out）、
// 元数据 JSON（json-out：subject/cn/issuer/not_before/not_after/key_type/
// key_matches_cert/chain_len/expired）。
//
// 纪律（与 generate-key 一致）：
//   - key-out 以 0600 落盘 —— 调用方（Rust）读完立即送 keychain 并删除；
//   - 本命令不做任何网络与持久化，只在给定路径写文件。
//
// 退出码：0 成功；2 参数/解析失败；6 私钥与证书不匹配；7 缺口令（ encrypted 输入
// 而未给 -password 或 -password-stdin —— Rust 据此让前端弹出密码框）；
// 8 口令错误（给了口令但解不开 —— Rust 据此让前端显示「口令不正确」并保留密码框，
// 而不是把它误报成「需要口令」让用户无限重弹）。
//
// 口令加密 PKCS#8（PBES2）Go 标准库不认，这里按 RFC 8018 手解：
// PBKDF2（HMAC-SHA1/224/256/384/512）+ AES-128/192/256-CBC 或 3DES-CBC。
// 传统的 legacy 加密 PEM（DEK-Info 头）走 x509.DecryptPEMBlock。
// PFX·P12 走 sslmate/go-pkcs12（覆盖旧 3DES 与新 AES 两种加密）。

import (
	"crypto"
	"crypto/aes"
	"crypto/cipher"
	"crypto/des"
	"crypto/ecdsa"
	"crypto/ed25519"
	"crypto/rsa"
	"crypto/sha1"
	"crypto/sha256"
	"crypto/sha512"
	"crypto/x509"
	"crypto/x509/pkix"
	"encoding/asn1"
	"encoding/hex"
	"encoding/json"
	"encoding/pem"
	"errors"
	"fmt"
	"hash"
	"io"
	"os"
	"strings"
	"time"

	"golang.org/x/crypto/pbkdf2"
	pkcs12mod "software.sslmate.com/src/go-pkcs12"
)

// ── ASN.1 结构（RFC 8018 / RFC 5958）─────────────────────────────────

type encryptedPrivateKeyInfo struct {
	Algo          pkix.AlgorithmIdentifier
	EncryptedData []byte
}

type pbes2Params struct {
	KeyDerivationFunc pkix.AlgorithmIdentifier
	EncryptionScheme  pkix.AlgorithmIdentifier
}

type pbkdf2Params struct {
	Salt           []byte
	IterationCount int
	KeyLength      int                      `asn1:"optional"`
	PRF            pkix.AlgorithmIdentifier `asn1:"optional"`
}

var (
	oidPBES2   = asn1.ObjectIdentifier{1, 2, 840, 113549, 1, 5, 13}
	oidPBKDF2  = asn1.ObjectIdentifier{1, 2, 840, 113549, 1, 5, 12}
	oidHMAC1   = asn1.ObjectIdentifier{1, 2, 840, 113549, 2, 7}
	oidHMAC224 = asn1.ObjectIdentifier{1, 2, 840, 113549, 2, 8}
	oidHMAC256 = asn1.ObjectIdentifier{1, 2, 840, 113549, 2, 9}
	oidHMAC384 = asn1.ObjectIdentifier{1, 2, 840, 113549, 2, 10}
	oidHMAC512 = asn1.ObjectIdentifier{1, 2, 840, 113549, 2, 11}
	oidAES128  = asn1.ObjectIdentifier{2, 16, 840, 1, 101, 3, 4, 1, 2}
	oidAES192  = asn1.ObjectIdentifier{2, 16, 840, 1, 101, 3, 4, 1, 22}
	oidAES256  = asn1.ObjectIdentifier{2, 16, 840, 1, 101, 3, 4, 1, 42}
	oid3DES    = asn1.ObjectIdentifier{1, 2, 840, 113549, 3, 7}
)

// errPasswordRequired 由 inspect 路径统一翻译为退出码 7。
var errPasswordRequired = errors.New("PASSWORD_REQUIRED")

// errPasswordWrong 由 inspect 路径统一翻译为退出码 8（S2：口令给了但解不开，
// 必须与「没给口令」区分 —— 否则前端把口令打错也当成「需要口令」无限重弹）。
var errPasswordWrong = errors.New("PASSWORD_WRONG")

// errInvalidPadding：PBES2 解密后填充非法 —— 实践中几乎必为口令错误。
var errInvalidPadding = errors.New("invalid padding (wrong password?)")

// decryptPKCS8Encrypted 解「ENCRYPTED PRIVATE KEY」（PBES2 为主，兼容 PBES1 家族里
// 常见的 pbeWithMD5AndDES 等不做 —— 现实中 PKCS#8 加密几乎都是 PBES2；legacy PEM
// 加密走 DecryptPEMBlock，PFX 走 go-pkcs12，三条路都全了）。
func decryptPKCS8Encrypted(der, password []byte) ([]byte, error) {
	var epki encryptedPrivateKeyInfo
	if _, err := asn1.Unmarshal(der, &epki); err != nil {
		return nil, fmt.Errorf("parse EncryptedPrivateKeyInfo: %w", err)
	}
	if !epki.Algo.Algorithm.Equal(oidPBES2) {
		return nil, fmt.Errorf("unsupported encryption algorithm %s", epki.Algo.Algorithm)
	}
	var params pbes2Params
	if _, err := asn1.Unmarshal(epki.Algo.Parameters.FullBytes, &params); err != nil {
		return nil, fmt.Errorf("parse PBES2 params: %w", err)
	}
	if !params.KeyDerivationFunc.Algorithm.Equal(oidPBKDF2) {
		return nil, fmt.Errorf("unsupported KDF %s", params.KeyDerivationFunc.Algorithm)
	}
	var kp pbkdf2Params
	if _, err := asn1.Unmarshal(params.KeyDerivationFunc.Parameters.FullBytes, &kp); err != nil {
		return nil, fmt.Errorf("parse PBKDF2 params: %w", err)
	}

	prf := func() hash.Hash { return sha1.New() } // RFC 8018 默认 hmacWithSHA1
	switch {
	case kp.PRF.Algorithm.Equal(oidHMAC1), len(kp.PRF.Algorithm) == 0:
		prf = func() hash.Hash { return sha1.New() }
	case kp.PRF.Algorithm.Equal(oidHMAC224):
		prf = func() hash.Hash { return sha256.New224() }
	case kp.PRF.Algorithm.Equal(oidHMAC256):
		prf = func() hash.Hash { return sha256.New() }
	case kp.PRF.Algorithm.Equal(oidHMAC384):
		prf = func() hash.Hash { return sha512.New384() }
	case kp.PRF.Algorithm.Equal(oidHMAC512):
		prf = func() hash.Hash { return sha512.New() }
	default:
		return nil, fmt.Errorf("unsupported PRF %s", kp.PRF.Algorithm)
	}

	keyLen := kp.KeyLength
	iv := []byte(nil)
	var block cipherBlock
	switch {
	case params.EncryptionScheme.Algorithm.Equal(oidAES128):
		keyLen = orDefault(keyLen, 16)
		block = mustAES()
	case params.EncryptionScheme.Algorithm.Equal(oidAES192):
		keyLen = orDefault(keyLen, 24)
		block = mustAES()
	case params.EncryptionScheme.Algorithm.Equal(oidAES256):
		keyLen = orDefault(keyLen, 32)
		block = mustAES()
	case params.EncryptionScheme.Algorithm.Equal(oid3DES):
		keyLen = orDefault(keyLen, 24)
		block = &desTriple{}
	default:
		return nil, fmt.Errorf("unsupported encryption scheme %s", params.EncryptionScheme.Algorithm)
	}

	// iv 在 encryptionScheme 的 parameters（OCTET STRING）里。
	if _, err := asn1.Unmarshal(params.EncryptionScheme.Parameters.FullBytes, &iv); err != nil {
		return nil, fmt.Errorf("parse IV: %w", err)
	}

	derived := pbkdf2.Key(password, kp.Salt, kp.IterationCount, keyLen, prf)
	plain, err := block.decrypt(derived, iv, epki.EncryptedData)
	if err != nil {
		return nil, err
	}
	return plain, nil
}

func orDefault(v, d int) int {
	if v == 0 {
		return d
	}
	return v
}

// cipherBlock 把两种块密码的「解密 + 去填充」收成一个接口。
type cipherBlock interface {
	decrypt(key, iv, data []byte) ([]byte, error)
}

func mustAES() cipherBlock { return aesBlock{} }

type aesBlock struct{}

func (aesBlock) decrypt(key, iv, data []byte) ([]byte, error) {
	b, err := aes.NewCipher(key)
	if err != nil {
		return nil, err
	}
	return cbcDecryptStrip(b, iv, data)
}

type desTriple struct{}

func (desTriple) decrypt(key, iv, data []byte) ([]byte, error) {
	b, err := des.NewTripleDESCipher(key)
	if err != nil {
		return nil, err
	}
	return cbcDecryptStrip(b, iv, data)
}

func cbcDecryptStrip(b cipher.Block, iv, data []byte) ([]byte, error) {
	if len(data) == 0 || len(data)%b.BlockSize() != 0 {
		return nil, errors.New("ciphertext not block-aligned")
	}
	out := make([]byte, len(data))
	cipher.NewCBCDecrypter(b, iv).CryptBlocks(out, data)
	return pkcs7Strip(out, b.BlockSize())
}

// pkcs7Strip 去除 PKCS#7 填充；填充满块（整块 padding）也按标准剥掉。
// 填充非法时返回 errInvalidPadding 哨兵（由调用方判「口令错误」）。
func pkcs7Strip(data []byte, blockSize int) ([]byte, error) {
	if len(data) == 0 {
		return nil, errors.New("empty plaintext")
	}
	pad := int(data[len(data)-1])
	if pad <= 0 || pad > blockSize || pad > len(data) {
		return nil, errInvalidPadding
	}
	for _, c := range data[len(data)-pad:] {
		if int(c) != pad {
			return nil, errInvalidPadding
		}
	}
	return data[:len(data)-pad], nil
}

// ── 输入解析 ─────────────────────────────────────────────────────────

type parsedKey struct {
	signer crypto.Signer
	// der 原始（未加密）DER，用于归一化输出
	kind string // "EC" / "RSA" / "ED25519" / "PKCS8"
}

type parsedCerts struct {
	leaf  *x509.Certificate   // 与私钥配对的那张
	chain []*x509.Certificate // 其余（中间件等），保持输入顺序
}

// parseKeyFile 把任意支持形态的私钥文件解成统一 signer。
//   - PEM：PKCS8 / 加密 PKCS8（需口令）/ PKCS1（含 legacy 加密）/ SEC1 EC（含 legacy 加密）
//   - 二进制：PFX·P12（口令）或裸 DER（PKCS8/PKCS1/EC）
func parseKeyFile(path, password string) (*parsedKey, []*x509.Certificate, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, nil, fmt.Errorf("read %s: %w", path, err)
	}
	if len(data) > 5 && string(data[:5]) == "-----" { // PEM
		return parseKeyPEM(data, password)
	}
	// 二进制：先当 PFX 试（magic 无法可靠嗅探，靠解析），再试裸 DER。
	if priv, certs, err := decodePFX(data, password); err == nil {
		pk := &parsedKey{signer: priv}
		return pk, certs, nil
	} else if errors.Is(err, pkcs12mod.ErrIncorrectPassword) {
		// PFX 结构成立（MAC 校验失败说明它就是 PFX）只是口令问题：
		// 没给口令 → 「需要口令」（前端弹框）；给过口令 → 「口令错误」。
		// 都不能报「无法识别格式」—— 那会让用户以为文件坏了（S2）。
		if password == "" {
			return nil, nil, errPasswordRequired
		}
		return nil, nil, errPasswordWrong
	}
	pk, err := parseKeyDER(data)
	if err != nil {
		return nil, nil, fmt.Errorf("unrecognized key format in %s", path)
	}
	return pk, nil, nil
}

func decodePFX(data []byte, password string) (crypto.Signer, []*x509.Certificate, error) {
	privAny, cert, cas, err := pkcs12mod.DecodeChain(data, password)
	if err != nil {
		// PFX 无口令时口令应为空串而非 nil —— 再给一次机会。
		if password != "" {
			return nil, nil, err
		}
		privAny, cert, cas, err = pkcs12mod.DecodeChain(data, "\x00")
		if err != nil {
			return nil, nil, err
		}
	}
	priv, ok := privAny.(crypto.Signer)
	if !ok {
		return nil, nil, errors.New("PFX private key is not a signer")
	}
	all := []*x509.Certificate{}
	if cert != nil {
		all = append(all, cert)
	}
	all = append(all, cas...)
	return priv, all, nil
}

func parseKeyPEM(data []byte, password string) (*parsedKey, []*x509.Certificate, error) {
	var (
		certs    []*x509.Certificate
		pk       *parsedKey
		needPw   bool
		wrongPw  bool
		lastErr  error
	)
	rest := data
	for {
		var block *pem.Block
		block, rest = pem.Decode(rest)
		if block == nil {
			break
		}
		switch block.Type {
		case "CERTIFICATE":
			if c, err := x509.ParseCertificate(block.Bytes); err == nil {
				certs = append(certs, c)
			}
		case "PRIVATE KEY":
			if k, err := parseKeyDER(block.Bytes); err == nil {
				pk = k
			} else {
				lastErr = err
			}
		case "RSA PRIVATE KEY":
			if isLegacyEncrypted(block) {
				der, err := x509.DecryptPEMBlock(block, []byte(password))
				if err != nil {
					// 口令错（且用户确实给过口令）→ exit 8；否则按「需要口令」。
					if password != "" && errors.Is(err, x509.IncorrectPasswordError) {
						needPw, wrongPw = true, true
					} else {
						needPw = true
					}
					lastErr = err
					break
				}
				if k, err := rsaFromDER(der); err == nil {
					pk = k
				}
				break
			}
			if k, err := rsaFromDER(block.Bytes); err == nil {
				pk = k
			}
		case "EC PRIVATE KEY":
			if isLegacyEncrypted(block) {
				der, err := x509.DecryptPEMBlock(block, []byte(password))
				if err != nil {
					// 口令错（且用户确实给过口令）→ exit 8；否则按「需要口令」。
					if password != "" && errors.Is(err, x509.IncorrectPasswordError) {
						needPw, wrongPw = true, true
					} else {
						needPw = true
					}
					lastErr = err
					break
				}
				if k, err := ecFromDER(der); err == nil {
					pk = k
				}
				break
			}
			if k, err := ecFromDER(block.Bytes); err == nil {
				pk = k
			}
		case "ENCRYPTED PRIVATE KEY":
			if password == "" {
				needPw = true
				break
			}
			der, err := decryptPKCS8Encrypted(block.Bytes, []byte(password))
			if err != nil {
				// 填充非法 = 口令错（exit 8）；算法/结构不支持 = 格式错（exit 2）。
				if errors.Is(err, errInvalidPadding) {
					needPw, wrongPw = true, true
				} else {
					lastErr = err
				}
				break
			}
			if k, err := parseKeyDER(der); err == nil {
				pk = k
			} else {
				lastErr = err
			}
		}
	}
	if pk == nil {
		if needPw {
			if wrongPw {
				return nil, nil, errPasswordWrong
			}
			return nil, nil, errPasswordRequired
		}
		if lastErr != nil {
			return nil, nil, fmt.Errorf("parse key PEM: %w", lastErr)
		}
		return nil, nil, errors.New("no private key found in PEM input")
	}
	return pk, certs, nil
}

func isLegacyEncrypted(block *pem.Block) bool {
	t, ok := block.Headers["Proc-Type"]
	return ok && len(t) >= 1 && t[0] == '4' && block.Headers["DEK-Info"] != ""
}

func parseKeyDER(der []byte) (*parsedKey, error) {
	if k, err := x509.ParsePKCS8PrivateKey(der); err == nil {
		s, ok := k.(crypto.Signer)
		if !ok {
			return nil, errors.New("PKCS8 key is not a signer")
		}
		return &parsedKey{signer: s, kind: "PKCS8"}, nil
	}
	if k, err := x509.ParsePKCS1PrivateKey(der); err == nil {
		return &parsedKey{signer: k, kind: "RSA"}, nil
	}
	if k, err := x509.ParseECPrivateKey(der); err == nil {
		return &parsedKey{signer: k, kind: "EC"}, nil
	}
	return nil, errors.New("unrecognized private key DER")
}

func rsaFromDER(der []byte) (*parsedKey, error) {
	k, err := x509.ParsePKCS1PrivateKey(der)
	if err != nil {
		return nil, err
	}
	return &parsedKey{signer: k, kind: "RSA"}, nil
}

func ecFromDER(der []byte) (*parsedKey, error) {
	k, err := x509.ParseECPrivateKey(der)
	if err != nil {
		return nil, err
	}
	return &parsedKey{signer: k, kind: "EC"}, nil
}

// parseCertFile：PEM 链（全部 CERTIFICATE 块）/ 单张 DER / PFX（口令）。
func parseCertFile(path, password string) ([]*x509.Certificate, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("read %s: %w", path, err)
	}
	if len(data) > 5 && string(data[:5]) == "-----" {
		var out []*x509.Certificate
		rest := data
		for {
			var block *pem.Block
			block, rest = pem.Decode(rest)
			if block == nil {
				break
			}
			if block.Type != "CERTIFICATE" {
				continue
			}
			if c, err := x509.ParseCertificate(block.Bytes); err == nil {
				out = append(out, c)
			}
		}
		if len(out) == 0 {
			return nil, errors.New("no certificate found in " + path)
		}
		return out, nil
	}
	if _, certs, err := decodePFX(data, password); err == nil && len(certs) > 0 {
		return certs, nil
	}
	if c, err := x509.ParseCertificate(data); err == nil { // 裸 DER
		return []*x509.Certificate{c}, nil
	}
	return nil, fmt.Errorf("unrecognized certificate format in %s", path)
}

// ── 归一化输出 ───────────────────────────────────────────────────────

func marshalKeyPEM(pk *parsedKey) ([]byte, error) {
	// 与 generate-key 的落盘约定对齐：EC 用 SEC1，其余用 PKCS8。
	switch k := pk.signer.(type) {
	case *ecdsa.PrivateKey:
		der, err := x509.MarshalECPrivateKey(k)
		if err != nil {
			return nil, err
		}
		return pem.EncodeToMemory(&pem.Block{Type: "EC PRIVATE KEY", Bytes: der}), nil
	default:
		der, err := x509.MarshalPKCS8PrivateKey(pk.signer)
		if err != nil {
			return nil, err
		}
		return pem.EncodeToMemory(&pem.Block{Type: "PRIVATE KEY", Bytes: der}), nil
	}
}

func marshalChainPEM(leaf *x509.Certificate, chain []*x509.Certificate) []byte {
	out := pem.EncodeToMemory(&pem.Block{Type: "CERTIFICATE", Bytes: leaf.Raw})
	for _, c := range chain {
		out = append(out, pem.EncodeToMemory(&pem.Block{Type: "CERTIFICATE", Bytes: c.Raw})...)
	}
	return out
}

// ── 元数据 ───────────────────────────────────────────────────────────

type certMeta struct {
	Subject        string `json:"subject"`
	CN             string `json:"cn"`
	Issuer         string `json:"issuer"`
	NotBefore      string `json:"not_before"`
	NotAfter       string `json:"not_after"`
	KeyType        string `json:"key_type"`
	KeyMatchesCert bool   `json:"key_matches_cert"`
	ChainLen       int    `json:"chain_len"`
	Expired        bool   `json:"expired"`
}

func keyTypeName(pub interface{}) string {
	switch pub.(type) {
	case *rsa.PublicKey:
		return "RSA"
	case *ecdsa.PublicKey:
		return "EC"
	case ed25519.PublicKey:
		return "ED25519"
	default:
		return "UNKNOWN"
	}
}

func pkEqual(a, b interface{}) bool {
	ab, err1 := x509.MarshalPKIXPublicKey(a)
	bb, err2 := x509.MarshalPKIXPublicKey(b)
	if err1 != nil || err2 != nil {
		return false
	}
	return hex.EncodeToString(ab) == hex.EncodeToString(bb)
}

func cnOf(name pkix.Name) string {
	if name.CommonName != "" {
		return name.CommonName
	}
	if len(name.Organization) > 0 {
		return name.Organization[0]
	}
	return name.String()
}

// ── 命令入口 ─────────────────────────────────────────────────────────

func cmdInspectCert() {
	var keyPath, certPath, password, keyOut, certOut, jsonOut string
	var passwordStdin bool
	args := os.Args[2:]
	for i := 0; i < len(args); i++ {
		switch args[i] {
		case "-key":
			keyPath = args[i+1]
			i++
		case "-cert":
			certPath = args[i+1]
			i++
		case "-password":
			password = args[i+1]
			i++
		case "-password-stdin":
			passwordStdin = true
		case "-key-out":
			keyOut = args[i+1]
			i++
		case "-cert-out":
			certOut = args[i+1]
			i++
		case "-json-out":
			jsonOut = args[i+1]
			i++
		}
	}
	if passwordStdin {
		// 口令走 stdin（S1）：命令行参数在 ps 里可见，窗口期内可被同机进程窥视。
		raw, err := io.ReadAll(os.Stdin)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error reading password from stdin: %v\n", err)
			os.Exit(2)
		}
		password = strings.TrimRight(string(raw), "\r\n")
	}
	if keyPath == "" || keyOut == "" || certOut == "" || jsonOut == "" {
		fmt.Fprintln(os.Stderr, "Error: -key, -key-out, -cert-out, -json-out are required")
		usage()
	}

	pk, certsFromKey, err := parseKeyFile(keyPath, password)
	if err != nil {
		if errors.Is(err, errPasswordRequired) {
			writeInspectJSON(jsonOut, certMeta{}, err) // 带标记输出，见 writeInspectJSON
			os.Exit(7)
		}
		if errors.Is(err, errPasswordWrong) {
			writeInspectJSON(jsonOut, certMeta{}, err)
			os.Exit(8)
		}
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(2)
	}

	certs := certsFromKey
	if certPath != "" {
		more, err := parseCertFile(certPath, password)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error: %v\n", err)
			os.Exit(2)
		}
		certs = append(certs, more...)
	}
	if len(certs) == 0 {
		fmt.Fprintln(os.Stderr, "Error: no certificate found (key-only input; provide -cert)")
		os.Exit(2)
	}

	// 找与私钥配对的叶子证书。
	leafIdx := -1
	for i, c := range certs {
		if pkEqual(pk.signer.Public(), c.PublicKey) {
			leafIdx = i
			break
		}
	}
	matches := leafIdx >= 0
	if !matches {
		// 不匹配也要把元数据写出去（UI 要展示「密钥与证书不配对」），但退出码 6。
		meta := certMeta{
			Subject:        certs[0].Subject.String(),
			CN:             cnOf(certs[0].Subject),
			Issuer:         certs[0].Issuer.String(),
			NotBefore:      certs[0].NotBefore.UTC().Format(time.RFC3339),
			NotAfter:       certs[0].NotAfter.UTC().Format(time.RFC3339),
			KeyType:        keyTypeName(pk.signer.Public()),
			KeyMatchesCert: false,
			ChainLen:       len(certs),
			Expired:        time.Now().After(certs[0].NotAfter),
		}
		writeInspectJSON(jsonOut, meta, nil)
		os.Exit(6)
	}

	leaf := certs[leafIdx]
	chain := append([]*x509.Certificate{}, certs[:leafIdx]...)
	chain = append(chain, certs[leafIdx+1:]...)

	// 归一化写盘：私钥 0600（generate-key 同款纪律）。
	keyPEM, err := marshalKeyPEM(pk)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: marshal key: %v\n", err)
		os.Exit(2)
	}
	if err := os.WriteFile(keyOut, keyPEM, 0600); err != nil {
		fmt.Fprintf(os.Stderr, "Error: write key-out: %v\n", err)
		os.Exit(2)
	}
	if err := os.WriteFile(certOut, marshalChainPEM(leaf, chain), 0600); err != nil {
		fmt.Fprintf(os.Stderr, "Error: write cert-out: %v\n", err)
		os.Exit(2)
	}

	now := time.Now()
	meta := certMeta{
		Subject:        leaf.Subject.String(),
		CN:             cnOf(leaf.Subject),
		Issuer:         leaf.Issuer.String(),
		NotBefore:      leaf.NotBefore.UTC().Format(time.RFC3339),
		NotAfter:       leaf.NotAfter.UTC().Format(time.RFC3339),
		KeyType:        keyTypeName(pk.signer.Public()),
		KeyMatchesCert: true,
		ChainLen:       len(certs),
		Expired:        now.After(leaf.NotAfter),
	}
	writeInspectJSON(jsonOut, meta, nil)

	fmt.Printf("OK: inspected certificate for %q (chain=%d, expired=%v)\n", meta.CN, meta.ChainLen, meta.Expired)
}

// writeInspectJSON：成功/不匹配/口令问题三种出口共用的 JSON 落盘。
// 缺口令时写入 {"password_required":true}、口令错误时 {"password_wrong":true}
// （其余字段为零值）—— Rust/前端据此区分「弹密码框」与「提示口令不正确」。
func writeInspectJSON(path string, meta certMeta, err error) {
	if err != nil {
		if errors.Is(err, errPasswordRequired) {
			data, _ := json.MarshalIndent(map[string]interface{}{"password_required": true}, "", "  ")
			_ = os.WriteFile(path, data, 0600)
			return
		}
		if errors.Is(err, errPasswordWrong) {
			data, _ := json.MarshalIndent(map[string]interface{}{"password_wrong": true}, "", "  ")
			_ = os.WriteFile(path, data, 0600)
			return
		}
	}
	data, _ := json.MarshalIndent(meta, "", "  ")
	_ = os.WriteFile(path, data, 0600)
}
