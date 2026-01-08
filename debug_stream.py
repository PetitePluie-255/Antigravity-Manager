import requests
import json

url = "http://127.0.0.1:3000/v1/chat/completions"
headers = {"Content-Type": "application/json"}
data = {
    "model": "gemini-3-pro-high",
    "messages": [{"role": "user", "content": "What is 2+2? Think step by step."}],
    "stream": True
}

print("Starting stream capture...")
response = requests.post(url, headers=headers, json=data, stream=True)

for line in response.iter_lines():
    if line:
        line_str = line.decode('utf-8')
        if line_str.startswith("data: "):
            json_str = line_str[6:]
            if json_str == "[DONE]":
                print("\n[STREAM DONE]")
                break
            try:
                chunk = json.loads(json_str)
                # Look for reasoning_content or content
                for choice in chunk.get("choices", []):
                    delta = choice.get("delta", {})
                    if "reasoning_content" in delta and delta["reasoning_content"]:
                        print(f"\n[REASONING]: {delta['reasoning_content']}")
                    if "content" in delta and delta["content"]:
                        print(f"[CONTENT]: {delta['content']}", end="", flush=True)
            except Exception as e:
                print(f"\nError parsing: {line_str} - {e}")
