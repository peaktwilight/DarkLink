#!/bin/bash

set -e  # Exit on any error

echo "🚀 DarkLink C2 Server - Build & Run"
echo "=================================="
echo ""

# Build frontend
echo "🔨 Building Vue frontend..."
cd web
if npm run build; then
    echo "✅ Frontend build completed successfully"
else
    echo "❌ Frontend build failed"
    exit 1
fi
cd ..
echo ""

# Build server
echo "🔨 Building Go server..."
if go build -o server ./cmd/server.go; then
    echo "✅ Server build completed successfully"
else
    echo "❌ Server build failed"
    exit 1
fi
echo ""

# Show URLs and start server
echo "🚀 Starting DarkLink server..."
echo ""
echo "📡 Server will be available at:"
echo "   🔒 https://localhost:8443"
echo ""
echo "🎯 Navigate to different sections using the sidebar:"
echo "   • Mission Control (Dashboard)"
echo "   • Listeners"
echo "   • Payload Generator"
echo "   • File Drop"
echo "   • Server Terminal"
echo ""
echo "Press Ctrl+C to stop the server"
echo "=================================="
echo ""

./server --config config/settings.yaml