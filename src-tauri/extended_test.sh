#!/bin/bash

# Configuration
PORT=8045
BASE_URL="http://127.0.0.1:$PORT"

echo "==========================================="
echo "Antigravity Extended Model Test Suite"
echo "==========================================="
echo "Target: $BASE_URL"
echo ""

# Helper function
run_test() {
    local protocol="$1"
    local model="$2"
    local endpoint="$3"
    local body="$4"
    local headers="$5"

    echo "-------------------------------------------"
    echo "Testing [$protocol] Model: $model"
    
    response=$(curl -s -X POST "$BASE_URL$endpoint" \
      -H "Content-Type: application/json" \
      $headers \
      -d "$body")

    # Simple validation
    if echo "$response" | grep -q -E "\"content\"|\"candidates\"|429|RESOURCE_EXHAUSTED"; then
        if echo "$response" | grep -q "error"; then
             echo "⚠️  Response (Error):"
             echo "$response" | jq -r '.error.message // .error' | head -n 3
        else
             echo "✅ Success"
             # Extract content based on protocol
             if [ "$protocol" == "OpenAI" ]; then
                echo "Result: $(echo "$response" | jq -r '.choices[0].message.content' | head -n 1)..."
             elif [ "$protocol" == "Claude" ]; then
                echo "Result: $(echo "$response" | jq -r '.content[0].text' | head -n 1)..."
             elif [ "$protocol" == "Gemini" ]; then
                echo "Result: $(echo "$response" | jq -r '.candidates[0].content.parts[0].text' | head -n 1)..."
             fi
        fi
    else
        echo "❌ Failed / Parse Error"
        echo "Raw: $response"
    fi
    echo ""
}

# 1. Gemini 2.5 Flash
run_test "OpenAI" "gemini-2.5-flash" "/v1/chat/completions" \
  '{ "model": "gemini-2.5-flash", "messages": [{"role": "user", "content": "Hi"}], "stream": false }' ""

run_test "Gemini" "gemini-2.5-flash" "/v1beta/models/gemini-2.5-flash:generateContent" \
  '{ "contents": [{"parts": [{"text": "Hi"}]}] }' ""

# 2. Gemini 3 Flash
run_test "OpenAI" "gemini-3-flash" "/v1/chat/completions" \
  '{ "model": "gemini-3-flash", "messages": [{"role": "user", "content": "Hi"}], "stream": false }' ""

run_test "Gemini" "gemini-3-flash" "/v1beta/models/gemini-3-flash:generateContent" \
  '{ "contents": [{"parts": [{"text": "Hi"}]}] }' ""

# 3. Claude Sonnet 4.5 Thinking
# Need to check headers for Claude protocol
CLAUE_HEADERS="-H \"x-api-key: dummy\" -H \"anthropic-version: 2023-06-01\""

run_test "Claude" "claude-sonnet-4-5-thinking" "/v1/messages" \
  '{ "model": "claude-sonnet-4-5-thinking", "messages": [{"role": "user", "content": "Hi"}], "max_tokens": 100, "stream": false }' \
  "$CLAUE_HEADERS"

# 4. Claude Opus 4.5 Thinking
run_test "Claude" "claude-opus-4-5-thinking" "/v1/messages" \
  '{ "model": "claude-opus-4-5-thinking", "messages": [{"role": "user", "content": "Hi"}], "max_tokens": 100, "stream": false }' \
  "$CLAUE_HEADERS"

echo "Extended Test Suite Completed."
