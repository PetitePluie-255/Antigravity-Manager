#!/bin/bash

# Kill any existing server
pkill -f antigravity_tools

# Build and start server in background
echo "Starting server..."
cargo run --bin antigravity_tools > server.log 2>&1 &
SERVER_PID=$!

# Wait for server to start
echo "Waiting for server to start..."
for i in {1..30}; do
    if grep -q "反代服务器启动在" server.log; then
        echo "Server started!"
        break
    fi
    sleep 1
done

# Test OpenAI List Models
echo "Testing OpenAI List Models..."
curl -s http://127.0.0.1:8045/v1/models | grep "gpt-4" && echo "✅ OpenAI List Models Passed" || echo "❌ OpenAI List Models Failed"

# Test OpenAI Chat Completions (Non-stream)
echo "Testing OpenAI Chat Completions..."
# Note: This requires valid tokens. If no tokens, it might return 503.
response=$(curl -s -X POST http://127.0.0.1:8045/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello!"}],
    "stream": false
  }')
echo "Response: $response"

if echo "$response" | grep -q "ok" || echo "$response" | grep -q "choices" || echo "$response" | grep -q "No available accounts"; then
     echo "✅ OpenAI Chat Handling (Logic Reachable)"
else
     echo "❌ OpenAI Chat Handling Failed (Unexpected Response)"
fi

# Test Gemini List Models
echo "Testing Gemini List Models..."
curl -s http://127.0.0.1:8045/v1beta/models | grep "gemini-2.5-pro" && echo "✅ Gemini List Models Passed" || echo "❌ Gemini List Models Failed"

# Test Gemini Generate Content
echo "Testing Gemini Generate..."
response=$(curl -v -s -X POST http://127.0.0.1:8045/v1beta/models/gemini-2.5-pro:generateContent \
  -H "Content-Type: application/json" \
  -d '{
    "contents": [{
      "parts": [{"text": "Hello"}]
    }]
  }' 2>&1)
echo "Response: $response"

if echo "$response" | grep -q "candidates" || echo "$response" | grep -q "No available accounts"; then
    echo "✅ Gemini Generate Handling (Logic Reachable)"
else
    echo "❌ Gemini Generate Handling Failed"
fi

# Cleanup
kill $SERVER_PID
