#!/bin/bash
# 测试 Kimi API tool_call_id 问题

API_KEY="${KIMI_API_KEY:?Need API key}"
BASE_URL="https://api.kimi.com/coding/v1/messages"

echo "=== First Request ==="

RESPONSE=$(curl -s -X POST "${BASE_URL}" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${API_KEY}" \
  -d '{
    "model": "kimi-k2-turbo-preview",
    "messages": [
      {
        "role": "user",
        "content": [
          {
            "type": "text",
            "text": "List files in /tmp"
          }
        ]
      }
    ],
    "tools": [
      {
        "type": "function",
        "name": "read",
        "description": "Read file or directory",
        "parameters": {
          "type": "object",
          "properties": {
            "filePath": {
              "type": "string",
              "description": "The absolute path to file or directory"
            }
          },
          "required": ["filePath"]
        }
      }
    ],
    "stream": false
  }')

echo "$RESPONSE" | jq -c . 2>/dev/null || echo "$RESPONSE"

# 检查是否有 tool_calls
CONTENT=$(echo "$RESPONSE" | jq -r '.choices[0].message.content[] | select(.type == "tool_use") | tostring' 2>/dev/null)
echo ""
echo "Content with tool_use: $CONTENT"
