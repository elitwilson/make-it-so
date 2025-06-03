#!/bin/bash

# Test script for TypeScript API and types
# Run from project root: ./scripts/test-ts.sh

echo "🧪 Running TypeScript API and Types Tests..."
echo

cd templates/tests

echo "📋 Running Types Tests..."
deno test mis-types.test.ts --allow-read --allow-env --allow-run
echo

echo "🔧 Running API Helper Functions Tests..."
deno test mis-plugin-api.test.ts --allow-read --allow-env --allow-run
echo

echo "📄 Running JSON Extraction Tests..."
deno test json-extraction.test.ts --allow-read --allow-env --allow-run
echo

echo "✅ All TypeScript tests completed!" 