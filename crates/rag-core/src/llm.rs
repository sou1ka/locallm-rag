//! LLM (Ollama) Integration Module
//!
//! Provides interface for communicating with Ollama via OpenAI-compatible API.
//! Supports both standard completions and streaming responses.

use crate::config::LlmConfig;
use async_openai::{Client, config::OpenAIConfig};
use async_openai::types::{
    ChatCompletionRequestAssistantMessage, ChatCompletionRequestAssistantMessageContent,
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
    ChatCompletionRequestSystemMessageContent, ChatCompletionRequestUserMessage,
    ChatCompletionRequestUserMessageContent, CreateChatCompletionRequest,
};
use futures::stream::StreamExt;
use std::sync::Arc;

/// Message role in conversation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// Single chat message
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

impl ChatMessage {
    pub fn new(role: MessageRole, content: String) -> Self {
        Self { role, content }
    }

    pub fn system(content: String) -> Self {
        Self {
            role: MessageRole::System,
            content,
        }
    }

    pub fn user(content: String) -> Self {
        Self {
            role: MessageRole::User,
            content,
        }
    }

    pub fn assistant(content: String) -> Self {
        Self {
            role: MessageRole::Assistant,
            content,
        }
    }

    /// Convert to async_openai format
    fn to_openai_message(&self) -> ChatCompletionRequestMessage {
        match self.role {
            MessageRole::System => {
                ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                    content: ChatCompletionRequestSystemMessageContent::Text(
                        self.content.clone(),
                    ),
                    name: None,
                })
            }
            MessageRole::User => {
                ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                    content: ChatCompletionRequestUserMessageContent::Text(
                        self.content.clone(),
                    ),
                    name: None,
                })
            }
            MessageRole::Assistant => {
                ChatCompletionRequestMessage::Assistant(ChatCompletionRequestAssistantMessage {
                    content: Some(ChatCompletionRequestAssistantMessageContent::Text(
                        self.content.clone(),
                    )),
                    name: None,
                    tool_calls: None,
                    audio: None,
                    refusal: None,
                    function_call: None,
                })
            }
        }
    }
}

/// LLM client wrapper for Ollama
pub struct LlmClient {
    client: Arc<Client<OpenAIConfig>>,
    config: LlmConfig,
}

impl LlmClient {
    /// Create a new LLM client from configuration
    pub fn new(config: LlmConfig) -> crate::Result<Self> {
        // Create OpenAI-compatible config for Ollama
        let openai_config = OpenAIConfig::new()
            .with_api_key(&config.api_key)
            .with_api_base(&config.base_url);

        let client = Client::with_config(openai_config);

        Ok(Self {
            client: Arc::new(client),
            config,
        })
    }

    /// Send a completion request and get the full response
    ///
    /// # Arguments
    /// * `messages` - List of chat messages in conversation order
    /// * `temperature` - Sampling temperature (0.0-2.0)
    /// * `max_tokens` - Maximum tokens in response
    ///
    /// # Returns
    /// The complete response text from the model
    pub async fn complete(
        &self,
        messages: Vec<ChatMessage>,
        temperature: f32,
        _max_tokens: u16,
    ) -> crate::Result<String> {
        let openai_messages: Vec<ChatCompletionRequestMessage> =
            messages.iter().map(|m| m.to_openai_message()).collect();

        let request = CreateChatCompletionRequest {
            model: self.config.model.clone(),
            messages: openai_messages,
            temperature: Some(temperature),
            ..Default::default()
        };

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .map_err(|e| crate::anyhow!("Failed to call Ollama API: {}", e))?;

        // Extract content from response
        response
            .choices
            .first()
            .and_then(|choice| choice.message.content.clone())
            .ok_or_else(|| crate::anyhow!("No content in Ollama response"))
    }

    /// Stream a completion request and call callback for each token
    ///
    /// # Arguments
    /// * `messages` - List of chat messages in conversation order
    /// * `temperature` - Sampling temperature (0.0-2.0)
    /// * `max_tokens` - Maximum tokens in response
    /// * `token_callback` - Function to call for each token
    ///
    /// # Returns
    /// The complete accumulated response text
    pub async fn complete_stream<F>(
        &self,
        messages: Vec<ChatMessage>,
        temperature: f32,
        _max_tokens: u16,
        mut token_callback: F,
    ) -> crate::Result<String>
    where
        F: FnMut(String),
    {
        let openai_messages: Vec<ChatCompletionRequestMessage> =
            messages.iter().map(|m| m.to_openai_message()).collect();

        let request = CreateChatCompletionRequest {
            model: self.config.model.clone(),
            messages: openai_messages,
            temperature: Some(temperature),
            ..Default::default()
        };

        let mut stream = self
            .client
            .chat()
            .create_stream(request)
            .await
            .map_err(|e| crate::anyhow!("Failed to start streaming from Ollama: {}", e))?;

        let mut full_response = String::new();

        while let Some(result) = stream.next().await {
            let response = result
                .map_err(|e| crate::anyhow!("Error in streaming response: {}", e))?;

            if let Some(choice) = response.choices.first() {
                if let Some(delta_content) = &choice.delta.content {
                    full_response.push_str(delta_content);
                    token_callback(delta_content.to_string());
                }
            }
        }

        Ok(full_response)
    }

    /// Get the model name configured in this client
    pub fn model_name(&self) -> &str {
        &self.config.model
    }

    /// Get the API base URL
    pub fn api_base(&self) -> &str {
        &self.config.base_url
    }
}

/// Build a RAG prompt with context and conversation history
///
/// # Arguments
/// * `system_prompt` - System instruction for the model
/// * `context` - Retrieved context from RAG
/// * `history` - Conversation history
/// * `query` - Current user query
///
/// # Returns
/// List of chat messages to send to LLM
pub fn build_rag_prompt(
    system_prompt: &str,
    context: &str,
    history: Vec<ChatMessage>,
    query: &str,
) -> Vec<ChatMessage> {
    let mut messages = vec![ChatMessage::system(system_prompt.to_string())];

    // Add context window if not empty
    if !context.is_empty() {
        let context_msg = format!("以下のコンテキストを参考に回答してください。\n\n{}", context);
        messages.push(ChatMessage::user(context_msg));
    }

    // Add conversation history
    messages.extend(history);

    // Add current query
    messages.push(ChatMessage::user(query.to_string()));

    messages
}

/// Default system prompt for RAG-based QA
pub fn default_system_prompt() -> &'static str {
    "You are a helpful assistant. Answer the user's question based on the provided context. \
     If the context doesn't contain relevant information, say so honestly."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_roles() {
        let system_msg = ChatMessage::system("System message".to_string());
        assert_eq!(system_msg.role, MessageRole::System);

        let user_msg = ChatMessage::user("User message".to_string());
        assert_eq!(user_msg.role, MessageRole::User);

        let assistant_msg = ChatMessage::assistant("Assistant message".to_string());
        assert_eq!(assistant_msg.role, MessageRole::Assistant);
    }

    #[test]
    fn test_build_rag_prompt() {
        let system_prompt = "You are helpful.";
        let context = "The sky is blue.";
        let history = vec![
            ChatMessage::user("What color is the sky?".to_string()),
            ChatMessage::assistant("The sky is blue.".to_string()),
        ];
        let query = "Why?";

        let messages = build_rag_prompt(system_prompt, context, history, query);

        // Should have: system, context-system, user, assistant, user
        assert_eq!(messages.len(), 5);
        assert_eq!(messages[1].role, MessageRole::User);
        assert!(messages[1].content.contains("コンテキスト"));
        assert_eq!(messages[2].role, MessageRole::User);
        assert_eq!(messages[3].role, MessageRole::Assistant);
        assert_eq!(messages[4].role, MessageRole::User);
        assert_eq!(messages[4].content, "Why?");
    }

    #[test]
    fn test_build_rag_prompt_empty_context() {
        let system_prompt = "You are helpful.";
        let context = "";
        let history = vec![];
        let query = "Hello?";

        let messages = build_rag_prompt(system_prompt, context, history, query);

        // Should have: system, user (no context system)
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, MessageRole::System);
        assert_eq!(messages[1].role, MessageRole::User);
    }

    #[test]
    fn test_llm_client_creation() {
        let config = LlmConfig {
            base_url: "http://localhost:11434/v1".to_string(),
            model: "mistral".to_string(),
            api_key: "ollama".to_string(),
        };

        let client = LlmClient::new(config);
        assert!(client.is_ok());

        let client = client.unwrap();
        assert_eq!(client.model_name(), "mistral");
        assert!(client.api_base().contains("localhost:11434"));
    }
}
