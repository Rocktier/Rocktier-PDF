package main

import (
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"crypto/x509"
	"encoding/pem"
	"testing"
)

func TestLoadPrivateKeyBytesPKCS8(t *testing.T) {
	key, err := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	if err != nil {
		t.Fatal(err)
	}
	der, err := x509.MarshalPKCS8PrivateKey(key)
	if err != nil {
		t.Fatal(err)
	}
	pemBytes := pem.EncodeToMemory(&pem.Block{Type: "PRIVATE KEY", Bytes: der})

	signer, err := loadPrivateKeyBytes(pemBytes, "<test>")
	if err != nil {
		t.Fatalf("loadPrivateKeyBytes: %v", err)
	}
	if signer.Public() == nil {
		t.Fatal("expected a usable signer")
	}
}

func TestLoadPrivateKeyBytesRejectsGarbage(t *testing.T) {
	if _, err := loadPrivateKeyBytes([]byte("not a key"), "<test>"); err == nil {
		t.Fatal("expected an error for non-key input")
	}
}
