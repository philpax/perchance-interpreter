#!/bin/bash
set -e

echo "Building Perchance Interpreter Frontend..."
echo ""

echo "Step 1: Building WASM module..."
wasm-pack build perchance-wasm --target web --out-dir ../frontend/src/wasm
echo "✓ WASM module built"
echo ""

echo "Step 2: Installing frontend dependencies..."
cd frontend
npm install
echo "✓ Dependencies installed"
echo ""

echo "Step 3: Building frontend..."
npm run build
echo "✓ Frontend built"
echo ""

echo "============================================"
echo "Build complete!"
echo "============================================"
echo ""
echo "To run the development server:"
echo "  cd frontend && npm run dev"
echo ""
echo "To preview the production build:"
echo "  cd frontend && npm run preview"
echo ""
