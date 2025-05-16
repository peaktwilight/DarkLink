package common

import (
	"encoding/hex"
	"fmt"
)

// XORDeobfuscate takes a hex-encoded string and a key, and returns the deobfuscated string.
// It mirrors the XOR deobfuscation logic used by the agent.
func XORDeobfuscate(hexStr string, key string) (string, error) {
	keyBytes := []byte(key)
	if len(keyBytes) == 0 {
		return "", fmt.Errorf("deobfuscation key cannot be empty")
	}

	encryptedBytes, err := hex.DecodeString(hexStr)
	if err != nil {
		return "", fmt.Errorf("failed to decode hex string: %w", err)
	}

	decryptedBytes := make([]byte, len(encryptedBytes))
	for i, b := range encryptedBytes {
		decryptedBytes[i] = b ^ keyBytes[i%len(keyBytes)]
	}

	return string(decryptedBytes), nil
}
