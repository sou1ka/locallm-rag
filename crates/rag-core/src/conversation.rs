//! Conversation management module
//!
//! Manages chat history, summary compression, and RAG-integrated query flow.
//! Summary compression triggers when history exceeds max_history_turns.

use crate::config::ConversationConfig;
use crate::llm::{build_rag_prompt, ChatMessage, LlmClient};
use crate::retriever::RetrievalResult;
use serde::{Deserialize, Serialize};

/// Single message in conversation history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String, // "user" | "assistant" | "system"
    pub content: String,
    pub created_at: String,
}

impl Message {
    pub fn user(content: String) -> Self {
        Self {
            role: "user".to_string(),
            content,
            created_at: crate::ingestor::now_iso8601(),
        }
    }

    pub fn assistant(content: String) -> Self {
        Self {
            role: "assistant".to_string(),
            content,
            created_at: crate::ingestor::now_iso8601(),
        }
    }
}

/// Conversation session with history and optional summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: Option<String>,
    pub summary: Option<String>,       // 要約圧縮されたテキスト
    pub messages: Vec<Message>,        // 直近の生メッセージ
    pub created_at: String,
    pub updated_at: String,
}

impl Conversation {
    /// Create a new conversation
    pub fn new(id: String) -> Self {
        let now = crate::ingestor::now_iso8601();
        Self {
            id,
            title: None,
            summary: None,
            messages: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Add a message to history
    pub fn push(&mut self, message: Message) {
        self.updated_at = crate::ingestor::now_iso8601();
        self.messages.push(message);
    }

    /// Total turn count (user messages only)
    pub fn turn_count(&self) -> usize {
        self.messages.iter().filter(|m| m.role == "user").count()
    }

    /// Build ChatMessage list for LLM (summary + recent messages)
    pub fn to_chat_messages(&self) -> Vec<ChatMessage> {
        let mut chat_messages = Vec::new();

        // 要約がある場合はシステムメッセージとして先頭に挿入
        if let Some(summary) = &self.summary {
            chat_messages.push(ChatMessage::system(format!(
                "これまでの会話の要約:\n\n{}",
                summary
            )));
        }

        // 直近の生メッセージを追加
        for msg in &self.messages {
            match msg.role.as_str() {
                "user"      => chat_messages.push(ChatMessage::user(msg.content.clone())),
                "assistant" => chat_messages.push(ChatMessage::assistant(msg.content.clone())),
                "system"    => chat_messages.push(ChatMessage::system(msg.content.clone())),
                _           => {}
            }
        }

        chat_messages
    }
}

/// Conversation manager handling RAG query flow and summary compression
pub struct ConversationManager {
    pub conversation: Conversation,
    config: ConversationConfig,
    llm: LlmClient,
}

impl ConversationManager {
    /// Create a new conversation manager
    pub fn new(
        conversation: Conversation,
        config: ConversationConfig,
        llm: LlmClient,
    ) -> Self {
        Self { conversation, config, llm }
    }

    /// Send a user message through RAG pipeline and get response
    ///
    /// # Arguments
    /// * `user_input` - User's prompt text
    /// * `rag_results` - Retrieved chunks from retriever (empty = RAG bypass)
    /// * `system_prompt` - System prompt for this session
    ///
    /// # Returns
    /// Assistant's response text
    pub async fn chat(
        &mut self,
        user_input: &str,
        rag_results: &[RetrievalResult],
        system_prompt: &str,
    ) -> crate::Result<String> {
        // 要約圧縮チェック
        if self.needs_compression() {
            self.compress().await?;
        }

        // RAGコンテキスト構築
        let context = build_context(rag_results);

        // 会話履歴をChatMessage形式に変換
        let history = self.conversation.to_chat_messages();

        // プロンプト構築
        let messages = build_rag_prompt(system_prompt, &context, history, user_input);

        // Ollama呼び出し
        let response = self.llm.complete(messages, 0.7, 2048).await?;

        // 履歴に保存
        self.conversation.push(Message::user(user_input.to_string()));
        self.conversation.push(Message::assistant(response.clone()));

        // タイトル自動生成（初回のみ）
        if self.conversation.title.is_none() && self.conversation.turn_count() == 1 {
            if let Ok(title) = self.generate_title(user_input).await {
                self.conversation.title = Some(title);
            }
        }

        Ok(response)
    }

    /// Stream version of chat - calls token_callback for each token
    pub async fn chat_stream<F>(
        &mut self,
        user_input: &str,
        rag_results: &[RetrievalResult],
        system_prompt: &str,
        token_callback: F,
    ) -> crate::Result<String>
    where
        F: FnMut(String),
    {
        if self.needs_compression() {
            self.compress().await?;
        }

        let context = build_context(rag_results);
        let history = self.conversation.to_chat_messages();
        let messages = build_rag_prompt(system_prompt, &context, history, user_input);

        let response = self.llm
            .complete_stream(messages, 0.7, 2048, token_callback)
            .await?;

        self.conversation.push(Message::user(user_input.to_string()));
        self.conversation.push(Message::assistant(response.clone()));

        if self.conversation.title.is_none() && self.conversation.turn_count() == 1 {
            if let Ok(title) = self.generate_title(user_input).await {
                self.conversation.title = Some(title);
            }
        }

        Ok(response)
    }

    /// Check if summary compression is needed
    fn needs_compression(&self) -> bool {
        self.conversation.turn_count() > self.config.max_history_turns
    }

    /// Compress old messages into summary, keep recent N turns
    async fn compress(&mut self) -> crate::Result<()> {
        let keep = self.config.summary_keep_recent * 2; // user + assistant のペア
        let total = self.conversation.messages.len();

        if total <= keep {
            return Ok(());
        }

        let old_messages = &self.conversation.messages[..total - keep];

        // 古いメッセージをテキスト化
        let history_text: String = old_messages
            .iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");

        // Ollamaに要約を依頼
        let summary_prompt = vec![
            ChatMessage::user(format!(
                "以下の会話を3〜5文で簡潔に要約してください。重要な事実・決定事項・文脈を保持してください。\n\n{}",
                history_text
            ))
        ];

        let new_summary = self.llm.complete(summary_prompt, 0.3, 512).await?;

        // 既存の要約と結合
        let combined_summary = if let Some(existing) = &self.conversation.summary {
            format!("{}\n\n{}", existing, new_summary)
        } else {
            new_summary
        };

        // 要約を保存し、古いメッセージを削除
        self.conversation.summary = Some(combined_summary.clone());
        self.conversation.messages = self.conversation.messages[total - keep..].to_vec();

        // mdファイルに書き出し（設定で有効な場合）
        if self.config.auto_save_summary {
            self.save_summary_md(&combined_summary).await?;
        }

        Ok(())
    }

    /// Save summary to markdown file
    async fn save_summary_md(&self, summary: &str) -> crate::Result<()> {
        let dir = std::path::Path::new(&self.config.summary_dir);
        std::fs::create_dir_all(dir)
            .map_err(|e| crate::anyhow!("Failed to create summary dir: {}", e))?;

        let filename = format!(
            "{}_{}.md",
            self.conversation.id,
            self.conversation.updated_at.replace(':', "-")
        );
        let path = dir.join(filename);

        let content = format!(
            "# Session: {}\n\n## Summary\n{}\n\n## Recent Messages (last {} turns)\n{}\n",
            self.conversation.id,
            summary,
            self.config.summary_keep_recent,
            self.conversation.messages
                .iter()
                .map(|m| format!("**{}**: {}", m.role, m.content))
                .collect::<Vec<_>>()
                .join("\n\n")
        );

        std::fs::write(&path, content)
            .map_err(|e| crate::anyhow!("Failed to write summary md: {}", e))?;

        Ok(())
    }

    /// Generate a short title from the first user message
    async fn generate_title(&self, first_message: &str) -> crate::Result<String> {
        let prompt = vec![ChatMessage::user(format!(
            "以下のメッセージから会話タイトルを10文字以内で生成してください。タイトルのみ返してください。\n\n{}",
            first_message
        ))];

        let title = self.llm.complete(prompt, 0.3, 64).await?;
        Ok(title.trim().to_string())
    }

    /// Get conversation reference
    pub fn conversation(&self) -> &Conversation {
        &self.conversation
    }
}

/// Build RAG context string from retrieval results
pub fn build_context(results: &[RetrievalResult]) -> String {
    if results.is_empty() {
        return String::new();
    }

    results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            format!(
                "[{}] (source: {}, score: {:.2})\n{}",
                i + 1,
                r.chunk.file_path,
                r.score,
                r.chunk.content
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n---\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Chunk;
    use crate::retriever::RetrievalResult;

    fn create_test_conversation() -> Conversation {
        Conversation::new("test-session".to_string())
    }

    fn create_test_result(content: &str, score: f32) -> RetrievalResult {
        RetrievalResult {
            chunk: Chunk {
                id: 0,
                source_type: "text".to_string(),
                file_path: "./test.txt".to_string(),
                page: None,
                chunk_index: 0,
                content: content.to_string(),
                ingested_at: "2026-01-01T00:00:00Z".to_string(),
            },
            score,
        }
    }

    #[test]
    fn test_conversation_new() {
        let conv = create_test_conversation();
        assert_eq!(conv.id, "test-session");
        assert_eq!(conv.turn_count(), 0);
        assert!(conv.summary.is_none());
        assert!(conv.title.is_none());
    }

    #[test]
    fn test_conversation_push_and_turn_count() {
        let mut conv = create_test_conversation();
        conv.push(Message::user("質問です".to_string()));
        conv.push(Message::assistant("回答です".to_string()));

        assert_eq!(conv.turn_count(), 1);
        assert_eq!(conv.messages.len(), 2);
    }

    #[test]
    fn test_to_chat_messages_no_summary() {
        let mut conv = create_test_conversation();
        conv.push(Message::user("こんにちは".to_string()));
        conv.push(Message::assistant("はい、こんにちは".to_string()));

        let messages = conv.to_chat_messages();
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_to_chat_messages_with_summary() {
        let mut conv = create_test_conversation();
        conv.summary = Some("過去の会話の要約".to_string());
        conv.push(Message::user("続きの質問".to_string()));

        let messages = conv.to_chat_messages();
        // system(summary) + user
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_build_context_empty() {
        let context = build_context(&[]);
        assert!(context.is_empty());
    }

    #[test]
    fn test_build_context_single() {
        let results = vec![create_test_result("テストコンテンツ", 0.95)];
        let context = build_context(&results);

        assert!(context.contains("テストコンテンツ"));
        assert!(context.contains("0.95"));
        assert!(context.contains("test.txt"));
    }

    #[test]
    fn test_build_context_multiple() {
        let results = vec![
            create_test_result("コンテンツ1", 0.95),
            create_test_result("コンテンツ2", 0.85),
        ];
        let context = build_context(&results);

        assert!(context.contains("コンテンツ1"));
        assert!(context.contains("コンテンツ2"));
        assert!(context.contains("---"));
    }

    #[test]
    fn test_needs_compression() {
        let config = ConversationConfig {
            max_history_turns: 3,
            summary_keep_recent: 2,
            auto_save_summary: false,
            summary_dir: "./summaries".to_string(),
        };
        let conv = create_test_conversation();
        let llm_config = crate::config::LlmConfig {
            base_url: "http://localhost:11434/v1".to_string(),
            model: "llama3.2".to_string(),
            api_key: "ollama".to_string(),
        };
        let llm = LlmClient::new(llm_config).unwrap();
        let mut manager = ConversationManager::new(conv, config, llm);

        // 3ターン未満なので圧縮不要
        assert!(!manager.needs_compression());

        // 4ターン追加して圧縮必要な状態に
        for _ in 0..4 {
            manager.conversation.push(Message::user("質問".to_string()));
            manager.conversation.push(Message::assistant("回答".to_string()));
        }
        assert!(manager.needs_compression());
    }
}
