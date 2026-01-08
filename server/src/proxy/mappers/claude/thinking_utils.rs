use super::models::{ContentBlock, Message, MessageContent};
use tracing::info;

#[derive(Debug, Default)]
pub struct ConversationState {
    pub in_tool_loop: bool,
    pub interrupted_tool: bool,
    pub last_assistant_idx: Option<usize>,
}

/// Analyze the conversation to detect tool loops or interrupted tool calls
pub fn analyze_conversation_state(messages: &[Message]) -> ConversationState {
    let mut state = ConversationState::default();

    if messages.is_empty() {
        return state;
    }

    // Find last assistant message index
    for (i, msg) in messages.iter().enumerate().rev() {
        if msg.role == "assistant" {
            state.last_assistant_idx = Some(i);
            break;
        }
    }

    // Check if the very last message is a Tool Result (User role with ToolResult block)
    if let Some(last_msg) = messages.last() {
        if last_msg.role == "user" {
            if let MessageContent::Array(blocks) = &last_msg.content {
                if blocks
                    .iter()
                    .any(|b| matches!(b, ContentBlock::ToolResult { .. }))
                {
                    state.in_tool_loop = true;
                }
            }
        }
    }

    state
}

/// Recover from broken tool loops by injecting synthetic messages
pub fn close_tool_loop_for_thinking(messages: &mut Vec<Message>) {
    let state = analyze_conversation_state(messages);

    if !state.in_tool_loop {
        return;
    }

    // Check if the last assistant message has a thinking block
    let mut has_thinking = false;
    if let Some(idx) = state.last_assistant_idx {
        if let Some(msg) = messages.get(idx) {
            if let MessageContent::Array(blocks) = &msg.content {
                has_thinking = blocks
                    .iter()
                    .any(|b| matches!(b, ContentBlock::Thinking { .. }));
            }
        }
    }

    if !has_thinking {
        info!("[Thinking-Recovery] Detected broken tool loop (ToolResult without preceding Thinking). Injecting synthetic messages.");

        messages.push(Message {
            role: "assistant".to_string(),
            content: MessageContent::Array(vec![ContentBlock::Text {
                text: "[Tool execution completed. Please proceed.]".to_string(),
            }]),
        });
        messages.push(Message {
            role: "user".to_string(),
            content: MessageContent::Array(vec![ContentBlock::Text {
                text: "Proceed.".to_string(),
            }]),
        });
    }
}
