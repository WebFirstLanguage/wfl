#!/bin/bash
# Generate self-signed certificates for testing HTTPS functionality

CERT_DIR="TestPrograms/.ssl"

echo "Generating test certificates for HTTPS testing..."

# Create .ssl directory if it doesn't exist
mkdir -p "$CERT_DIR"

# Generate self-signed certificate valid for 365 days
openssl req -x509 -newkey rsa:2048 -nodes \
    -keyout "$CERT_DIR/key.pem" \
    -out "$CERT_DIR/cert.pem" \
    -days 365 \
    -subj "/CN=localhost/O=WFL Test/C=US" \
    2>/dev/null

if [ $? -eq 0 ]; then
    echo "✓ Test certificates generated successfully in $CERT_DIR/"
    echo "  - cert.pem: Certificate"
    echo "  - key.pem: Private key"
    echo ""
    echo "These certificates are for testing only and should not be used in production."
else
    echo "✗ Failed to generate certificates. Make sure OpenSSL is installed."
    exit 1
fi
