#!/bin/bash

# DarkLink Frontend Migration Script
# Migrates from vanilla JS/HTML to modern Vue.js 3 frontend

echo "🚀 DarkLink Frontend Migration"
echo "=============================="

# Create backup directory
BACKUP_DIR="backup-$(date +%Y%m%d-%H%M%S)"
echo "📦 Creating backup in: $BACKUP_DIR"
mkdir -p "$BACKUP_DIR"

# Backup original files
echo "💾 Backing up original files..."
cp -r css js *.html "$BACKUP_DIR/" 2>/dev/null || true

# Build the new Vue.js app
echo "🔨 Building Vue.js application..."
npm run build

# Check if build was successful
if [ $? -eq 0 ]; then
    echo "✅ Build successful!"
    
    # Move new files
    echo "📁 Deploying new frontend..."
    
    # Copy the main entry point
    cp index-new.html index.html
    
    # The dist folder contains the built assets
    echo "📂 Built files available in 'dist/' directory"
    echo "   - dist/index.html (Main dashboard)"
    echo "   - dist/listeners.html"
    echo "   - dist/payload.html"
    echo "   - dist/file_drop.html"
    echo "   - dist/server_terminal.html"
    
    echo ""
    echo "🎉 Migration complete!"
    echo ""
    echo "Development:"
    echo "  npm run dev    # Start development server (http://localhost:3000)"
    echo ""
    echo "Production:"
    echo "  The Go server should serve files from the 'dist/' directory"
    echo ""
    echo "Rollback:"
    echo "  cp $BACKUP_DIR/* . # Restore original files"
    
else
    echo "❌ Build failed! Check the errors above."
    exit 1
fi