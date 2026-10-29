package main

import (
	"crypto"
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"crypto/x509"
	"crypto/x509/pkix"
	"encoding/json"
	"encoding/pem"
	"errors"
	"fmt"
	"image"
	"image/color"
	"image/draw"
	"image/png"
	"io"
	"math/big"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/digitorus/pdfsign/sign"
	"github.com/digitorus/pdfsign/verify"
	"golang.org/x/image/font"
	"golang.org/x/image/font/basicfont"
	"golang.org/x/image/math/fixed"
)

func main() {
	if len(os.Args) < 2 {
		usage()
	}

	switch os.Args[1] {
	case "sign":
		cmdSign()
	case "verify":
		cmdVerify()
	case "generate-key":
		cmdGenerateKey()
	case "generate-sig-image":
		cmdGenerateSigImage()
	case "inspect-cert":
		cmdInspectCert()
	default:
		usage()
	}
}

func usage() {
	fmt.Println("Usage: sign-engine <command> [options]")
	fmt.Println("")
	fmt.Println("Commands:")
	fmt.Println("  sign           Sign a PDF file")
	fmt.Println("  verify         Verify a PDF signature")
	fmt.Println("  generate-key   Generate a self-signed certificate + private key")
	fmt.Println("  generate-sig-image  Generate a default signature appearance PNG")
	fmt.Println("  sign 额外支持 -rect llx,lly,urx,ury（自由摆放，优先于 -position）")
	fmt.Println("  inspect-cert    Inspect & normalize an imported certificate/key (PEM/DER/PFX)")
	fmt.Println("")
	fmt.Println("  inspect-cert -key <k> -key-out <k> -cert-out <c> -json-out <j> [-cert <c>] [-password <pw> | -password-stdin]")
	fmt.Println("    exit: 0 ok | 2 parse error | 6 key/cert mismatch | 7 password required | 8 password wrong")
	fmt.Println("")
	fmt.Println("  sign -input <i> -output <o> -cert <c> (-key <k> | -key-stdin) [-name <n>] [-rect <l,b,r,t>] [-page <n>] [-image <p>] [-position <corner>]")
	fmt.Println("  sign -name <name> -input <input.pdf> -output <output.pdf> -key <private.key> -cert <cert.crt> [-image <sig.png>] [-position <top-left|top-right|bottom-left|bottom-right>] [-page <n>]")
	fmt.Println("  verify <input.pdf>")
	fmt.Println("  generate-key -name <name> -key-out <private.key> -cert-out <cert.crt>")
	fmt.Println("  generate-sig-image -name <name> -output <sig.png>")
	os.Exit(1)
}

// readPEM reads a PEM-encoded file and returns blocks or raw content
func readPEM(path string) ([]byte, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("read %s: %w", path, err)
	}
	return data, nil
}

func loadPrivateKey(path string) (crypto.Signer, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	return loadPrivateKeyBytes(data, path)
}

func loadPrivateKeyBytes(data []byte, label string) (crypto.Signer, error) {
	block, _ := pem.Decode(data)
	if block == nil {
		// Try parsing as raw
		block = &pem.Block{Type: "PRIVATE KEY", Bytes: data}
	}
	if keyAny, err := x509.ParsePKCS8PrivateKey(block.Bytes); err == nil {
		if signer, ok := keyAny.(crypto.Signer); ok {
			return signer, nil
		}
	}
	if rsaKey, err := x509.ParsePKCS1PrivateKey(block.Bytes); err == nil {
		return rsaKey, nil
	}
	if ecKey, err := x509.ParseECPrivateKey(block.Bytes); err == nil {
		return ecKey, nil
	}
	return nil, fmt.Errorf("unable to parse private key from %s", label)
}

func loadCertificate(path string) (*x509.Certificate, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}

	block, _ := pem.Decode(data)
	if block == nil {
		return nil, fmt.Errorf("no PEM block found in %s", path)
	}

	return x509.ParseCertificate(block.Bytes)
}

// loadCertChain 读入全部 CERTIFICATE 块（叶子在前，保持文件顺序）。
// 外部导入的证书常带中间件；inspect-cert 的归一化 cert-out 也是这个布局。
// 单张自签名证书返回 (cert, nil, nil)，签名路径行为与旧版完全一致。
func loadCertChain(path string) (*x509.Certificate, []*x509.Certificate, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, nil, err
	}
	var certs []*x509.Certificate
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
		c, err := x509.ParseCertificate(block.Bytes)
		if err != nil {
			return nil, nil, fmt.Errorf("parse certificate in %s: %w", path, err)
		}
		certs = append(certs, c)
	}
	if len(certs) == 0 {
		return nil, nil, fmt.Errorf("no PEM certificate found in %s", path)
	}
	return certs[0], certs[1:], nil
}

func cmdSign() {
	var name, input, output, keyPath, certPath, imagePath, position, rectStr string
	var page int
	var keyStdin bool

	// Parse flags
	args := os.Args[2:]
	for i := 0; i < len(args); i++ {
		switch args[i] {
		case "-name":
			name = args[i+1]
			i++
		case "-input":
			input = args[i+1]
			i++
		case "-output":
			output = args[i+1]
			i++
		case "-key":
			keyPath = args[i+1]
			i++
		case "-key-stdin":
			// 私钥 PEM 从 stdin 读（与 -key 二选一）；调用方读完管道即关闭，
			// 明文私钥不落命令行、不长驻文件。
			keyStdin = true
		case "-cert":
			certPath = args[i+1]
			i++
		case "-image":
			imagePath = args[i+1]
			i++
		case "-position":
			position = args[i+1]
			i++
		case "-rect":
			rectStr = args[i+1]
			i++
		case "-page":
			fmt.Sscanf(args[i+1], "%d", &page)
			i++
		}
	}

	if input == "" || output == "" || certPath == "" || (keyPath == "" && !keyStdin) {
		fmt.Fprintln(os.Stderr, "Error: -input, -output, -cert and either -key or -key-stdin are required")
		usage()
	}

	var signer crypto.Signer
	var loadErr error
	if keyStdin {
		// 私钥经 stdin 传入：调用方（Rust）从系统钥匙串取出后写入管道，
		// 进程结束即消失，磁盘上不留明文私钥（对应设计 §3.4）。
		var data []byte
		data, loadErr = io.ReadAll(os.Stdin)
		if loadErr == nil {
			signer, loadErr = loadPrivateKeyBytes(data, "<stdin>")
		}
	} else {
		signer, loadErr = loadPrivateKey(keyPath)
	}
	if loadErr != nil {
		fmt.Fprintf(os.Stderr, "Error loading private key: %v\n", loadErr)
		os.Exit(2)
	}

	cert, extraCerts, err := loadCertChain(certPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error loading certificate: %v\n", err)
		os.Exit(2)
	}

	// Default: bottom-right on the last page if visual sig requested but no position given
	if position == "" {
		position = "bottom-right"
	}

	signData := sign.SignData{
		Signature: sign.SignDataSignature{
			Info: sign.SignDataSignatureInfo{
				Name: name,
			},
			CertType:   sign.ApprovalSignature,
			DocMDPPerm: sign.AllowFillingExistingFormFieldsAndSignaturesPerms,
		},
		Signer:          signer,
		DigestAlgorithm: crypto.SHA256,
		Certificate:     cert,
	}

	// 有中间证书时走 pdfsign 原生的 CertificateChains（[0][0]=叶子由 Certificate
	// 字段单独承担，[1:] 作为附加链嵌入）。单张自签名证书不设该字段 —— 行为与旧版逐字一致。
	if len(extraCerts) > 0 {
		signData.CertificateChains = [][]*x509.Certificate{append([]*x509.Certificate{cert}, extraCerts...)}
	}

	// 签名落点（阶段 72/D-53 v1）：
	//   -rect "llx,lly,urx,ury"  自由摆放（前端在真实页面预览上拖出来的绝对坐标，
	//                            PDF 点、左下原点），优先于 -position；
	//   -page <n>                落页（此前被解析后丢弃，等于永远第 1 页 —— 本版修复）；
	//   -position <四角>         兼容旧路径。
	// 无 image 时 Appearance.Visible=false：不可见签名仍占据该页的签名域。
	if rectStr != "" {
		llx, lly, urx, ury, err := parseRect(rectStr)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error parsing -rect: %v\n", err)
			os.Exit(2)
		}
		if page < 1 {
			page = 1
		}
		signData.Appearance = sign.Appearance{
			Visible:     imagePath != "",
			Page:        uint32(page),
			LowerLeftX:  llx,
			LowerLeftY:  lly,
			UpperRightX: urx,
			UpperRightY: ury,
		}
		if imagePath != "" {
			imageData, rerr := os.ReadFile(imagePath)
			if rerr != nil {
				fmt.Fprintf(os.Stderr, "Error reading image: %v\n", rerr)
				os.Exit(2)
			}
			signData.Appearance.Image = imageData
		}
	} else if imagePath != "" {
		imageData, err := os.ReadFile(imagePath)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error reading image: %v\n", err)
			os.Exit(2)
		}
		llx, lly, urx, ury := positionToCoords(position, page, 0, 220.0, 72.0)
		signData.Appearance = sign.Appearance{
			Visible:     true,
			Page:        uint32(page),
			LowerLeftX:  llx,
			LowerLeftY:  lly,
			UpperRightX: urx,
			UpperRightY: ury,
			Image:       imageData,
		}
	}

	err = sign.SignFile(input, output, signData)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error signing: %v\n", err)
		os.Exit(3)
	}

	fmt.Printf("OK: signed %s -> %s\n", input, output)
}

// parseRect 解析 "llx,lly,urx,ury"（PDF 点、左下原点），并做基本秩序校验。
// 页边界由调用方（前端按真实页面尺寸换算）保证；这里只挡明显错误。
func parseRect(s string) (llx, lly, urx, ury float64, err error) {
	var parts [4]float64
	for i := 0; i < 4; i++ {
		if _, err := fmt.Sscanf(s, "%f", &parts[i]); err != nil {
			return 0, 0, 0, 0, fmt.Errorf("component %d: %w", i+1, err)
		}
		if i < 3 {
			s = s[strings.IndexByte(s, ',')+1:]
		}
	}
	llx, lly, urx, ury = parts[0], parts[1], parts[2], parts[3]
	if llx >= urx || lly >= ury {
		return 0, 0, 0, 0, errors.New("rect must satisfy llx < urx and lly < ury")
	}
	if llx < 0 || lly < 0 {
		return 0, 0, 0, 0, errors.New("rect coordinates must be non-negative")
	}
	return llx, lly, urx, ury, nil
}

// positionToCoords converts a named position into PDF coordinates.
// PDF origin is bottom-left. A4-ish page assumed (595×842 pt).
// Signature rect defaults to 220×72 pt.
func positionToCoords(pos string, page, customPage int, w, h float64) (llx, lly, urx, ury float64) {
	if w == 0 {
		w = 220
	}
	if h == 0 {
		h = 72
	}

	margin := 40.0
	pageWidth := 595.0
	pageHeight := 842.0

	switch pos {
	case "top-left":
		llx = margin
		lly = pageHeight - margin - h
		urx = margin + w
		ury = pageHeight - margin
	case "top-right":
		llx = pageWidth - margin - w
		lly = pageHeight - margin - h
		urx = pageWidth - margin
		ury = pageHeight - margin
	case "bottom-left":
		llx = margin
		lly = margin
		urx = margin + w
		ury = margin + h
	case "bottom-right":
		llx = pageWidth - margin - w
		lly = margin
		urx = pageWidth - margin
		ury = margin + h
	default:
		// Default to bottom-right
		llx = pageWidth - margin - w
		lly = margin
		urx = pageWidth - margin
		ury = margin + h
	}
	return
}

// cmdGenerateSigImage creates a default signature appearance PNG.
func cmdGenerateSigImage() {
	var name, output string
	args := os.Args[2:]
	for i := 0; i < len(args); i++ {
		switch args[i] {
		case "-name":
			name = args[i+1]
			i++
		case "-output":
			output = args[i+1]
			i++
		}
	}

	if name == "" || output == "" {
		fmt.Fprintln(os.Stderr, "Error: -name, -output are required")
		usage()
	}

	now := time.Now().Format("2006-01-02 15:04")
	if err := generateSigImage(name, now, output); err != nil {
		fmt.Fprintf(os.Stderr, "Error generating sig image: %v\n", err)
		os.Exit(2)
	}
	fmt.Printf("OK: generated %s\n", output)
}

// generateSigImage draws a clean digital signature appearance.
func generateSigImage(name, dateStr, outputPath string) error {
	const width, height = 220, 72

	// Dark background with rounded-corner feel via border
	img := image.NewRGBA(image.Rect(0, 0, width, height))

	// Transparent background
	draw.Draw(img, img.Bounds(), &image.Uniform{color.RGBA{0, 0, 0, 0}}, image.Point{}, draw.Src)

	// Fill with subtle dark background
	for y := 0; y < height; y++ {
		for x := 0; x < width; x++ {
			img.Set(x, y, color.RGBA{255, 255, 255, 255})
		}
	}

	// Border
	borderColor := color.RGBA{0, 0, 0, 255}
	for x := 0; x < width; x++ {
		img.Set(x, 0, borderColor)
		img.Set(x, height-1, borderColor)
	}
	for y := 0; y < height; y++ {
		img.Set(0, y, borderColor)
		img.Set(width-1, y, borderColor)
	}

	// Inner light border (2px)
	innerColor := color.RGBA{230, 230, 230, 255}
	for x := 2; x < width-2; x++ {
		img.Set(x, 2, innerColor)
		img.Set(x, height-3, innerColor)
	}
	for y := 2; y < height-2; y++ {
		img.Set(2, y, innerColor)
		img.Set(width-3, y, innerColor)
	}

	// Add text: name (bold) + date (smaller)
	face := basicfont.Face7x13

	// Name text (centered vertically, slightly above center)
	nameY := height/2 - 8
	addLabel(img, face, 10, nameY, name, color.RGBA{0, 0, 0, 255})

	// Date text
	dateY := height/2 + 10
	addLabel(img, face, 10, dateY, "Digitally signed: "+dateStr, color.RGBA{60, 60, 60, 255})

	// Right-side: red dot (brand mark)
	redDot := color.RGBA{255, 74, 61, 255}
	for dy := -6; dy <= 6; dy++ {
		for dx := -6; dx <= 6; dx++ {
			if dx*dx+dy*dy <= 36 {
				img.Set(width-22+dx, height/2+dy, redDot)
			}
		}
	}

	f, err := os.Create(outputPath)
	if err != nil {
		return err
	}
	defer f.Close()

	return png.Encode(f, img)
}

// addLabel draws text on the image at the given coordinates.
func addLabel(img *image.RGBA, face font.Face, x, y int, text string, col color.Color) {
	point := fixed.Point26_6{
		X: fixed.I(x),
		Y: fixed.I(y),
	}
	d := &font.Drawer{
		Dst:  img,
		Src:  image.NewUniform(col),
		Face: face,
		Dot:  point,
	}
	d.DrawString(text)
}

func cmdVerify() {
	if len(os.Args) < 3 {
		fmt.Fprintln(os.Stderr, "Error: verify requires a PDF file path")
		usage()
	}

	input := os.Args[2]
	file, err := os.Open(input)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error opening file: %v\n", err)
		os.Exit(2)
	}
	defer file.Close()

	options := verify.DefaultVerifyOptions()
	// Keep the chain check ON. A self-signed certificate is NOT auto-trusted by
	// the verifier (the old code's comment claimed otherwise) — TrustedIssuer is
	// false for self-signed, which the UI surfaces as "valid but untrusted".
	options.AllowUntrustedRoots = false

	result, err := verify.VerifyFileWithOptions(file, options)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error verifying: %v\n", err)
		os.Exit(3)
	}

	// Classify every signer and derive an overall verdict + exit code so the
	// host can colour the result. A tampered file must never look identical to
	// a valid one (P0-14).
	verdict := "VALID"
	exitCode := 0
	var signers []map[string]interface{}
	if len(result.Signers) == 0 {
		verdict = "NO_SIGNATURES"
		exitCode = 4
	}
	for _, s := range result.Signers {
		// ExtKeyUsageValid lives on the nested Certificate, not on Signer.
		ekuValid := true
		if len(s.Certificates) > 0 {
			ekuValid = s.Certificates[0].ExtKeyUsageValid
		}
		status := "valid"
		if !s.ValidSignature {
			status = "invalid"
			if exitCode < 4 {
				exitCode = 4
			}
			verdict = "INVALID"
		} else if !s.TrustedIssuer || !ekuValid || s.RevokedCertificate {
			status = "untrusted"
			if exitCode < 5 {
				exitCode = 5
			}
			if verdict == "VALID" {
				verdict = "UNTRUSTED"
			}
		}
		signerInfo := map[string]interface{}{
			"name":                s.Name,
			"valid_signature":     s.ValidSignature,
			"trusted_issuer":      s.TrustedIssuer,
			"ext_key_usage_valid": ekuValid,
			"revoked":             s.RevokedCertificate,
			"signature_time":      s.SignatureTime,
			"timestamp_status":    s.TimestampStatus,
			"status":              status,
		}
		signers = append(signers, signerInfo)
	}

	var summary string
	switch verdict {
	case "VALID":
		summary = fmt.Sprintf("All %d signature(s) are valid and trusted.", len(result.Signers))
	case "UNTRUSTED":
		summary = fmt.Sprintf("%d signature(s) present and cryptographically valid, but NOT from a trusted issuer (self-signed or untrusted chain).", len(result.Signers))
	case "INVALID":
		summary = fmt.Sprintf("%d signature(s) present; at least one is INVALID — the document may have been altered since signing.", len(result.Signers))
	case "NO_SIGNATURES":
		summary = "This PDF contains no digital signatures."
	}

	out := map[string]interface{}{
		"verdict":   verdict,
		"exit_code": exitCode,
		"summary":   summary,
		"signers":   signers,
	}
	jsonData, _ := json.MarshalIndent(out, "", "  ")
	fmt.Println(string(jsonData))
	os.Exit(exitCode)
}

func cmdGenerateKey() {
	var name, keyOut, certOut string
	args := os.Args[2:]
	for i := 0; i < len(args); i++ {
		switch args[i] {
		case "-name":
			name = args[i+1]
			i++
		case "-key-out":
			keyOut = args[i+1]
			i++
		case "-cert-out":
			certOut = args[i+1]
			i++
		}
	}

	if name == "" || keyOut == "" || certOut == "" {
		fmt.Fprintln(os.Stderr, "Error: -name, -key-out, -cert-out are required")
		usage()
	}

	// Generate P-256 EC key
	privateKey, err := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error generating key: %v\n", err)
		os.Exit(2)
	}

	// Random 128-bit serial: a constant serial (big.NewInt(1)) means every
	// user's certificate shares the same identity number — no PKI semantics.
	serialLimit := new(big.Int).Lsh(big.NewInt(1), 128)
	serial, err := rand.Int(rand.Reader, serialLimit)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error generating serial number: %v\n", err)
		os.Exit(2)
	}

	template := x509.Certificate{
		SerialNumber: serial,
		Subject: pkix.Name{
			CommonName:   name,
			Organization: []string{"Rocktier Sign (Self-Signed)"},
		},
		NotBefore:             time.Now(),
		NotAfter:              time.Now().Add(10 * 365 * 24 * time.Hour), // 10 years
		KeyUsage:              x509.KeyUsageDigitalSignature,
		ExtKeyUsage:           []x509.ExtKeyUsage{x509.ExtKeyUsageAny},
		BasicConstraintsValid: true,
	}

	certDER, err := x509.CreateCertificate(rand.Reader, &template, &template, &privateKey.PublicKey, privateKey)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error creating certificate: %v\n", err)
		os.Exit(2)
	}

	// Write private key. 0600 — a "signatures unforgeable" tool must not
	// leave the signing key group/world-readable (os.Create → 0644).
	keyFile, err := os.OpenFile(keyOut, os.O_WRONLY|os.O_CREATE|os.O_TRUNC, 0600)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error creating key file: %v\n", err)
		os.Exit(2)
	}
	defer keyFile.Close()

	keyDER, _ := x509.MarshalECPrivateKey(privateKey)
	pem.Encode(keyFile, &pem.Block{Type: "EC PRIVATE KEY", Bytes: keyDER})
	keyFile.Close()

	// Write certificate
	certFile, err := os.Create(certOut)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error creating cert file: %v\n", err)
		os.Exit(2)
	}
	defer certFile.Close()

	pem.Encode(certFile, &pem.Block{Type: "CERTIFICATE", Bytes: certDER})
	certFile.Close()

	fmt.Printf("OK: generated self-signed certificate for %q\n", name)
	keyAbs, _ := filepath.Abs(keyOut)
	certAbs, _ := filepath.Abs(certOut)
	fmt.Printf("  Private key: %s\n", keyAbs)
	fmt.Printf("  Certificate: %s\n", certAbs)
}
