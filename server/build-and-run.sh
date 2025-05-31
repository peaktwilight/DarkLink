#!/bin/bash

echo "🔨 Building Vue frontend..."
cd web
npm run build
cd ..

echo "🔨 Building Go server..."
go build -o server ./cmd/server.go

echo "🚀 Starting DarkLink server..."
./server --config config/settings.yaml