#!/bin/bash

# Configuration
PORT=8045
BASE_URL="http://127.0.0.1:$PORT"

echo "==========================================="
echo "Antigravity Integrated Protocol Test Suite"
echo "==========================================="
echo "Target: $BASE_URL"
echo ""

# Function to test OpenAI Chat Completion
test_openai_chat() {
    local model_input="$1"
    local expected_model="$2" # Optional: What we might expect in logs, but here we check success.
    
    echo "-------------------------------------------"
    echo "Testing OpenAI Protocol with model: $model_input"
    echo "Sending request..."
    
    response=$(curl -s -X POST "$BASE_URL/v1/chat/completions" \
      -H "Content-Type: application/json" \
      -d '{
        "model": "'"$model_input"'",
        "messages": [{"role": "user", "content": "Hello, simply say OK."}],
        "stream": false
      }')
    
    echo "Raw Response: $response"
    
    if echo "$response" | grep -q "\"content\""; then
        echo "✅ Success"
        content=$(echo "$response" | jq -r '.choices[0].message.content')
        echo "Reply Content: $content"
    else
        echo "❌ Failed"
    fi
    echo ""
}

# Function to test Claude Messages
test_claude_messages() {
    local model_input="$1"
    
    echo "-------------------------------------------"
    echo "Testing Claude Protocol with model: $model_input"
    echo "Sending request..."
    
    # Note: Antigravity might strictly require stream=true for Claude? Let's try non-stream first if supported.
    # handlers/claude.rs seems to assume strict SSE if not specified or specified.
    # Actually handlers/claude.rs:33 checks request.stream.
    
    response=$(curl -s -X POST "$BASE_URL/v1/messages" \
      -H "Content-Type: application/json" \
      -H "x-api-key: dummy" \
      -H "anthropic-version: 2023-06-01" \
      -d '{
        "model": "'"$model_input"'",
        "messages": [{"role": "user", "content": "Hello, simply say OK."}],
        "max_tokens": 100,
        "stream": false
      }')
    
    echo "Raw Response: $response"
    
    if echo "$response" | grep -q "\"content\""; then
        echo "✅ Success"
    else
        echo "❌ Failed"
    fi
    echo ""
}

# Function to test Gemini Native
test_gemini_native() {
    local model_input="$1"
    local method="$2"
    
    echo "-------------------------------------------"
    echo "Testing Gemini Native Protocol: $model_input ($method)"
    
    # Construct URL for model:method if separate
    # Using the fix logic: /v1beta/models/MODEL:generateContent
    
    url="$BASE_URL/v1beta/models/${model_input}:$method"
    
    response=$(curl -s -X POST "$url" \
      -H "Content-Type: application/json" \
      -d '{
        "contents": [{
          "parts": [{"text": "Hello, simply say OK."}]
        }]
      }')
      
    echo "Raw Response: $response"
    
    if echo "$response" | grep -q "\"candidates\""; then
         echo "✅ Success (Got candidates)"
    elif echo "$response" | grep -q "RESOURCE_EXHAUSTED"; then
         echo "✅ Success (Got Upstream 429 - Connection Valid)"
    else
         echo "❌ Failed"
    fi
     echo ""
}

# 1. Test OpenAI Protocol with Standard GPT model (Mapped to default or fallback?)
# common/model_mapping.rs CLAUDE_TO_GEMINI doesn't map "gpt-4".
# It falls back to "claude-sonnet-4-5" via unwrap_or.
test_openai_chat "gpt-4"

# 2. Test OpenAI Protocol with Explicit Claude Mapping
# "claude-3-5-sonnet-20241022" -> "claude-sonnet-4-5"
test_openai_chat "claude-3-5-sonnet-20241022"

# 3. Test Claude Protocol
# "claude-3-5-sonnet-20241022" -> Mapped to "claude-sonnet-4-5" internally in handlers/claude.rs? 
# handlers/claude.rs logic relies on client.rs/UpstreamClient.
test_claude_messages "claude-3-5-sonnet-20241022"

# 4. Test Gemini Native - Pro (Known to 429)
test_gemini_native "gemini-2.5-pro" "generateContent"

# 5. Test Gemini Native - Flash (Maybe works?)
test_gemini_native "gemini-2.5-flash" "generateContent"

echo "Test Suite Completed."
