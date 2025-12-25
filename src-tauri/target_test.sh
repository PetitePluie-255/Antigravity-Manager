#!/bin/bash

# Configuration
PORT=8045
BASE_URL="http://127.0.0.1:$PORT"
MODEL="claude-sonnet-4-5-thinking"

echo "==========================================="
echo "Targeted Test: $MODEL (All Protocols)"
echo "==========================================="
echo "Target: $BASE_URL"
echo ""

# Helper function
run_test() {
    local protocol="$1"
    local endpoint="$2"
    local body="$3"
    local headers="$4"

    echo "-------------------------------------------"
    echo "Testing [$protocol]"
    
    response=$(curl -s -X POST "$BASE_URL$endpoint" \
      -H "Content-Type: application/json" \
      $headers \
      -d "$body")

    # Simple validation
    if echo "$response" | grep -q -E "\"content\"|\"candidates\"|429|RESOURCE_EXHAUSTED"; then
        if echo "$response" | grep -q "error"; then
             echo "⚠️  Response (Error):"
             echo "$response" 
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

# 1. OpenAI Protocol
run_test "OpenAI" "/v1/chat/completions" \
  '{ "model": "'"$MODEL"'", "messages": [{"role": "user", "content": "Hi"}], "stream": false }' ""

# 2. Claude Protocol
CLAUE_HEADERS="-H \"x-api-key: dummy\" -H \"anthropic-version: 2023-06-01\""
run_test "Claude" "/v1/messages" \
  '{ "model": "'"$MODEL"'", "messages": [{"role": "user", "content": "Hi"}], "max_tokens": 100, "stream": false }' \
  "$CLAUE_HEADERS"

# 3. Gemini Protocol
# Payload structure must be strict for v1internal pass-through.
run_test "Gemini" "/v1beta/models/${MODEL}:generateContent" \
  '{ 
     "contents": [{"role": "user", "parts": [{"text": "Hi"}]}],
     "generationConfig": {
        "thinkingConfig": {
            "includeThoughts": true,
            "thinkingBudget": 4000
        }
    }
  }' ""

echo "Targeted Test Completed."
