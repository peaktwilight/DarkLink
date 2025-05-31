#!/bin/bash

echo "🔐 Regenerating TLS certificates for DarkLink server..."
echo "=================================================="

# Create certs directory if it doesn't exist
mkdir -p certs

# Backup existing certificates
if [ -f "certs/server.crt" ]; then
    echo "📁 Backing up existing certificates..."
    mv certs/server.crt certs/server.crt.backup 2>/dev/null
    mv certs/server.key certs/server.key.backup 2>/dev/null
fi

# Generate new self-signed certificate with proper extensions
echo "🔨 Generating new certificate with correct key usage for HTTPS..."
openssl req -x509 -newkey rsa:4096 \
    -keyout certs/server.key \
    -out certs/server.crt \
    -days 365 \
    -nodes \
    -subj "/C=US/ST=Local/L=Local/O=DarkLink/OU=C2/CN=localhost" \
    -extensions v3_ca \
    -config <(cat <<EOF
[req]
distinguished_name = req_distinguished_name
req_extensions = v3_ca
prompt = no

[req_distinguished_name]
C = US
ST = Local
L = Local
O = DarkLink
OU = C2
CN = localhost

[v3_ca]
basicConstraints = CA:FALSE
keyUsage = critical, digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
DNS.2 = 127.0.0.1
IP.1 = 127.0.0.1
IP.2 = ::1
EOF
)

# Check if generation was successful
if [ $? -eq 0 ]; then
    echo "✅ Certificates generated successfully!"
    echo ""
    echo "📋 Certificate Details:"
    openssl x509 -in certs/server.crt -text -noout | grep -A 5 -E "(Not Before|Not After|Subject:|X509v3 Subject Alternative Name)"
    echo ""
    echo "🎯 Next Steps:"
    echo "1. Add certificate to macOS Keychain (see instructions below)"
    echo "2. Restart the DarkLink server"
    echo "3. Navigate to https://localhost:8443"
    echo ""
    echo "🔑 To make your browser trust this certificate:"
    echo ""
    echo "   macOS Instructions:"
    echo "   1. Double-click certs/server.crt (opens Keychain Access)"
    echo "   2. Find 'localhost' certificate in 'login' keychain"
    echo "   3. Double-click the certificate"
    echo "   4. Expand 'Trust' section"
    echo "   5. Set 'When using this certificate' to 'Always Trust'"
    echo "   6. Close and enter your password"
    echo ""
    echo "   Alternative command line method:"
    echo "   sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain certs/server.crt"
    echo ""
    echo "💡 After adding to keychain, restart your browser!"
else
    echo "❌ Certificate generation failed!"
    echo "Please ensure OpenSSL is installed and try again"
    exit 1
fi