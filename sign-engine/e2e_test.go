package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

// 一个最小但结构完整的单页 PDF：目录 / 页树 / 页 / 内容流。
// 加一行注释当「篡改探针」—— 翻转它不会破坏解析，只会改变签名覆盖的字节。
func buildMinimalPDF() []byte {
	var buf bytes.Buffer
	buf.WriteString("%PDF-1.7\n")
	offsets := make([]int, 5)
	objs := []string{
		"<< /Type /Catalog /Pages 2 0 R >>",
		"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
		"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R /Resources << >> >>",
		"<< /Length 0 >>\nstream\n\nendstream",
	}
	for i, body := range objs {
		offsets[i+1] = buf.Len()
		fmt.Fprintf(&buf, "%d 0 obj\n%s\nendobj\n", i+1, body)
	}
	xref := buf.Len()
	buf.WriteString("xref\n0 5\n")
	buf.WriteString("0000000000 65535 f \n")
	for i := 1; i <= 4; i++ {
		fmt.Fprintf(&buf, "%010d 00000 n \n", offsets[i])
	}
	fmt.Fprintf(&buf, "trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n", xref)
	return buf.Bytes()
}

func buildBinary(t *testing.T) string {
	t.Helper()
	bin := filepath.Join(t.TempDir(), "sign-engine-test")
	if err := exec.Command("go", "build", "-o", bin, ".").Run(); err != nil {
		t.Fatalf("go build: %v", err)
	}
	return bin
}

func run(t *testing.T, bin string, stdin []byte, args ...string) (string, int) {
	t.Helper()
	cmd := exec.Command(bin, args...)
	if stdin != nil {
		cmd.Stdin = bytes.NewReader(stdin)
	}
	var stdout, stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr
	err := cmd.Run()
	code := 0
	if ee, ok := err.(*exec.ExitError); ok {
		code = ee.ExitCode()
	} else if err != nil {
		t.Fatalf("run %v: %v", args, err)
	}
	if code != 0 && stdout.Len() == 0 {
		t.Logf("stderr: %s", stderr.String())
	}
	return stdout.String(), code
}

type verifyJSON struct {
	Verdict string `json:"verdict"`
	Signers []struct {
		ValidSignature bool   `json:"valid_signature"`
		TrustedIssuer  bool   `json:"trusted_issuer"`
		Status         string `json:"status"`
	} `json:"signers"`
}

func TestSignVerifyTamperDoubleSign(t *testing.T) {
	bin := buildBinary(t)
	dir := t.TempDir()
	keyPath := filepath.Join(dir, "k.pem")
	certPath := filepath.Join(dir, "c.pem")

	if _, code := run(t, bin, nil, "generate-key", "-name", "E2E", "-key-out", keyPath, "-cert-out", certPath); code != 0 {
		t.Fatalf("generate-key exit %d", code)
	}
	keyPEM, err := os.ReadFile(keyPath)
	if err != nil {
		t.Fatal(err)
	}

	original := buildMinimalPDF()
	in := filepath.Join(dir, "in.pdf")
	if err := os.WriteFile(in, original, 0o644); err != nil {
		t.Fatal(err)
	}
	signed := filepath.Join(dir, "signed.pdf")
	if _, code := run(t, bin, keyPEM, "sign", "-input", in, "-output", signed,
		"-cert", certPath, "-key-stdin", "-name", "E2E", "-rect", "10,10,170,60", "-page", "1"); code != 0 {
		t.Fatalf("sign exit %d", code)
	}

	after, _ := os.ReadFile(in)
	if !bytes.Equal(original, after) {
		t.Fatal("engine modified the input file; signing must be append-only")
	}

	out, code := run(t, bin, nil, "verify", signed)
	if code != 5 {
		t.Fatalf("self-signed verify exit = %d, want 5 (untrusted)", code)
	}
	var v verifyJSON
	if err := json.Unmarshal([]byte(out), &v); err != nil {
		t.Fatalf("verify output is not JSON: %v\n%s", err, out)
	}
	if v.Verdict != "UNTRUSTED" || len(v.Signers) != 1 || !v.Signers[0].ValidSignature {
		t.Fatalf("unexpected verdict: %+v", v)
	}

	// 二次签名：增量追加，第一个签名必须仍然有效。
	dbl := filepath.Join(dir, "double.pdf")
	if _, code := run(t, bin, keyPEM, "sign", "-input", signed, "-output", dbl,
		"-cert", certPath, "-key-stdin", "-name", "E2E2", "-rect", "10,80,170,130", "-page", "1"); code != 0 {
		t.Fatalf("second sign exit %d", code)
	}
	out, _ = run(t, bin, nil, "verify", dbl)
	if err := json.Unmarshal([]byte(out), &v); err != nil {
		t.Fatalf("double verify: %v", err)
	}
	if len(v.Signers) != 2 {
		t.Fatalf("signer count = %d, want 2", len(v.Signers))
	}
	for i, s := range v.Signers {
		if !s.ValidSignature {
			t.Fatalf("signer %d invalid after append", i)
		}
	}

	// 篡改：翻转页码对象里 /Count 后的一个字节，签名覆盖区改变。
	tamperedBytes, _ := os.ReadFile(dbl)
	if !bytes.Contains(tamperedBytes, []byte("/Count 1")) {
		t.Fatal("fixture changed; cannot pick a tamper byte")
	}
	tamperedBytes = bytes.Replace(tamperedBytes, []byte("/Count 1"), []byte("/Count 2"), 1)
	tampered := filepath.Join(dir, "tampered.pdf")
	_ = os.WriteFile(tampered, tamperedBytes, 0o644)

	out, code = run(t, bin, nil, "verify", tampered)
	if code != 4 {
		t.Fatalf("tampered verify exit = %d, want 4 (invalid)\n%s", code, out)
	}
	if !strings.Contains(out, `"INVALID"`) {
		t.Fatalf("tampered verdict should be INVALID:\n%s", out)
	}
}
