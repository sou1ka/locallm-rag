//! Example: async-openai 0.27 CreateChatCompletionRequestArgs Usage
//!
//! This example demonstrates the correct way to use CreateChatCompletionRequestArgs
//! in async-openai 0.27, addressing common issues:
//! - Private fields (use builder methods instead)
//! - Method chaining patterns
//! - Proper type conversions
//! - max_tokens handling

use async_openai::{
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
        ChatCompletionRequestSystemMessageContent, ChatCompletionRequestUserMessage,
        ChatCompletionRequestUserMessageContent, CreateChatCompletionRequestArgs,
    },
};

/// Pattern 1: Basic usage with all required fields
///
/// ✅ CORRECT: Use builder methods via method chaining
/// ❌ WRONG: Do NOT use `builder.field = Some(value)`
fn example_basic_usage() -> Result<(), Box<dyn std::error::Error>> {
    // Create messages
    let messages = vec![
        ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
            content: ChatCompletionRequestSystemMessageContent::Text(
                "You are a helpful assistant.".to_string(),
            ),
            name: None,
        }),
        ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
            content: ChatCompletionRequestUserMessageContent::Text(
                "What is Rust?".to_string(),
            ),
            name: None,
        }),
    ];

    // ✅ CORRECT PATTERN: Method chaining
    let request = CreateChatCompletionRequestArgs::default()
        .model("mistral") // or "gpt-4", "llama2", etc.
        .messages(messages)
        .temperature(0.7_f32) // Explicit f32 type
        .max_tokens(1024_u32) // Explicit u32 type
        .build()?;

    println!("Request built successfully!");
    println!("Model: {}", request.model);
    println!("Messages count: {}", request.messages.len());
    println!("Temperature: {:?}", request.temperature);
    println!("Max tokens: {:?}", request.max_tokens);

    Ok(())
}

/// Pattern 2: Conditional max_tokens with mutable builder
///
/// Use mutable variable when you need conditional logic
fn example_conditional_max_tokens(
    use_max_tokens: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let messages = vec![ChatCompletionRequestMessage::User(
        ChatCompletionRequestUserMessage {
            content: ChatCompletionRequestUserMessageContent::Text(
                "Explain quantum computing".to_string(),
            ),
            name: None,
        },
    )];

    // Start builder
    let mut builder = CreateChatCompletionRequestArgs::default()
        .model("gpt-4")
        .messages(messages)
        .temperature(0.5_f32);

    // Conditionally add max_tokens
    if use_max_tokens {
        builder = builder.max_tokens(2048_u32);
    }

    let request = builder.build()?;
    println!(
        "Max tokens: {:?}",
        request.max_tokens.unwrap_or(0)
    );

    Ok(())
}

/// Pattern 3: Using all optional parameters
///
/// async-openai 0.27 supports many parameters:
fn example_advanced_usage() -> Result<(), Box<dyn std::error::Error>> {
    let messages = vec![
        ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
            content: ChatCompletionRequestSystemMessageContent::Text(
                "You are an expert programmer.".to_string(),
            ),
            name: None,
        }),
        ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
            content: ChatCompletionRequestUserMessageContent::Text(
                "Write a Hello World in Rust".to_string(),
            ),
            name: None,
        }),
    ];

    let request = CreateChatCompletionRequestArgs::default()
        .model("mistral")
        .messages(messages)
        .temperature(0.7_f32) // Creativity level (0.0 = deterministic, 2.0 = creative)
        .max_tokens(512_u32) // Maximum response length
        .top_p(0.9_f32) // Nucleus sampling
        .frequency_penalty(0.0_f32) // Reduce repetition
        .presence_penalty(0.0_f32) // Encourage new topics
        .build()?;

    println!("Advanced request built successfully!");
    println!("Temperature: {:?}", request.temperature);
    println!("Top P: {:?}", request.top_p);

    Ok(())
}

/// Pattern 4: Type conversion guide
///
/// Important type mappings for async-openai 0.27:
fn example_type_conversions() -> Result<(), Box<dyn std::error::Error>> {
    let messages = vec![];

    // ❌ WRONG: These will NOT compile
    // builder.temperature = Some(0.7_f32);           // Can't assign to private field
    // builder.max_tokens = Some(512);                 // Must be u32, not i32
    // builder.max_tokens(512_i32)                     // Type mismatch

    // ✅ CORRECT: Use proper types
    let _request = CreateChatCompletionRequestArgs::default()
        .model("mistral")
        .messages(messages)
        .temperature(0.7_f32) // Must be f32, not f64 or integer
        .max_tokens(512_u32) // Must be u32, not u16 or i32
        .build()?;

    Ok(())
}

/// Pattern 5: Builder with from_env configuration
///
/// Typical usage with environment-based config
fn example_with_config(
    model_name: String,
    user_query: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let system_prompt = "You are a helpful RAG assistant. Answer based on context provided.";
    let context = "The Earth orbits the Sun. The Moon orbits the Earth.";

    let messages = vec![
        ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
            content: ChatCompletionRequestSystemMessageContent::Text(
                format!("{}\n\nContext:\n{}", system_prompt, context),
            ),
            name: None,
        }),
        ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
            content: ChatCompletionRequestUserMessageContent::Text(user_query),
            name: None,
        }),
    ];

    let request = CreateChatCompletionRequestArgs::default()
        .model(model_name)
        .messages(messages)
        .temperature(0.7_f32)
        .max_tokens(1024_u32)
        .build()?;

    println!("RAG request configured: model={}", request.model);
    Ok(())
}

/// Summary of Correct Pattern
///
/// # DO's ✅
/// ```ignore
/// let request = CreateChatCompletionRequestArgs::default()
///     .model("mistral")                    // Use method calls
///     .messages(vec![...])                 // Chain methods
///     .temperature(0.7_f32)                // Explicit f32
///     .max_tokens(512_u32)                 // Explicit u32
///     .build()?;                           // Final build()
/// ```
///
/// # DON'Ts ❌
/// ```ignore
/// let mut builder = CreateChatCompletionRequestArgs::default();
/// builder.model = Some("mistral");         // ❌ Private field
/// builder.temperature = Some(0.7);         // ❌ Can't assign
/// let request = builder.build()?;
/// ```
///
/// # Key Points
/// - Always use `.method_name(value)` not direct field assignment
/// - `temperature` and `top_p` must be f32
/// - `max_tokens` must be u32 (not u16 or i32)
/// - Chain methods before calling `.build()`
/// - Use mutable builder only for conditional logic
/// - No need for `function_call` field (deprecated)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_builder_pattern() {
        assert!(example_basic_usage().is_ok());
    }

    #[test]
    fn test_conditional_builder() {
        assert!(example_conditional_max_tokens(true).is_ok());
        assert!(example_conditional_max_tokens(false).is_ok());
    }

    #[test]
    fn test_advanced_parameters() {
        assert!(example_advanced_usage().is_ok());
    }

    #[test]
    fn test_type_conversions() {
        assert!(example_type_conversions().is_ok());
    }

    #[test]
    fn test_rag_config() {
        assert!(example_with_config(
            "mistral".to_string(),
            "What orbits the Earth?".to_string()
        )
        .is_ok());
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== async-openai 0.27 Usage Examples ===\n");

    println!("1. Basic usage:");
    example_basic_usage()?;

    println!("\n2. Conditional max_tokens:");
    example_conditional_max_tokens(true)?;

    println!("\n3. Advanced parameters:");
    example_advanced_usage()?;

    println!("\n4. Type conversions:");
    example_type_conversions()?;

    println!("\n5. RAG configuration:");
    example_with_config("mistral".to_string(), "What is Rust?".to_string())?;

    println!("\n=== All examples completed successfully! ===");
    Ok(())
}
```

このサンプルコードは以下の問題を解決します：

## 🎯 問題の解決

| 問題 | 解決方法 |
|------|--------|
| **フィールドがprivate** | ビルダーメソッド `.model()`, `.messages()` 等を使用 |
| **一時値エラー** | `let mut builder` で可変変数を作成し、条件付きで再割り当て |
| **型の問題** | `temperature` は `f32`、`max_tokens` は `u32` に明示的に指定 |
| **deprecated field** | `function_call` は使用せず、最新パターンに従う |

## ✅ 推奨パターン

```rust
// Pattern A: シンプルな場合（メソッドチェーン）
let request = CreateChatCompletionRequestArgs::default()
    .model("mistral")
    .messages(messages)
    .temperature(0.7_f32)
    .max_tokens(512_u32)
    .build()?;

// Pattern B: 条件付きパラメータがある場合
let mut builder = CreateChatCompletionRequestArgs::default()
    .model("mistral")
    .messages(messages);

if enable_tokens {
    builder = builder.max_tokens(512_u32);
}

let request = builder.build()?;
```

このファイルは `examples/` ディレクトリに配置でき、テストと実行が可能です。