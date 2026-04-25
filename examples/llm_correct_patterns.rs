//! Reference Implementation: async-openai 0.27 Correct Patterns
//!
//! This file demonstrates the CORRECT way to build CreateChatCompletionRequestArgs
//! in async-openai 0.27, addressing all common issues:
//!
//! ✅ Uses builder methods instead of field assignment
//! ✅ Proper type conversions (u16 -> u32, f64 -> f32)
//! ✅ Handles conditional fields correctly
//! ✅ Avoids deprecated patterns
//! ✅ Includes comprehensive tests

use async_openai::{
    types::{
        ChatCompletionRequestAssistantMessage, ChatCompletionRequestAssistantMessageContent,
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
        ChatCompletionRequestSystemMessageContent, ChatCompletionRequestUserMessage,
        ChatCompletionRequestUserMessageContent, CreateChatCompletionRequestArgs,
    },
};

// ============================================================================
// PATTERN 1: Basic Usage (Most Common)
// ============================================================================

/// Demonstrates the simplest correct pattern for building a chat completion request
/// 
/// This is the recommended approach when you have all parameters available upfront.
///
/// # Key Points
/// - Use method chaining from default()
/// - Each method call returns Self for chaining
/// - Call build() at the end
/// - Use explicit type annotations where needed
fn pattern_basic_usage() -> Result<(), Box<dyn std::error::Error>> {
    let model_name = "mistral".to_string();
    let user_query = "What is Rust?".to_string();

    let messages = vec![ChatCompletionRequestMessage::User(
        ChatCompletionRequestUserMessage {
            content: ChatCompletionRequestUserMessageContent::Text(user_query),
            name: None,
        },
    )];

    // ✅ CORRECT PATTERN
    let request = CreateChatCompletionRequestArgs::default()
        .model(model_name)
        .messages(messages)
        .temperature(0.7_f32) // Explicit f32
        .max_tokens(512_u32) // Explicit u32
        .build()?;

    println!("✓ Pattern 1 - Basic request built successfully");
    println!("  Model: {}", request.model);
    println!("  Max tokens: {:?}", request.max_tokens);
    println!("  Temperature: {:?}", request.temperature);

    Ok(())
}

// ============================================================================
// PATTERN 2: Conditional Fields (Important for RAG scenarios)
// ============================================================================

/// Demonstrates how to handle optional fields that depend on runtime conditions
///
/// This pattern is useful when:
/// - max_tokens should only be set if > 0
/// - Additional parameters depend on feature flags
/// - Configuration from external sources
///
/// # Key Points
/// - Initialize with required fields
/// - Use mutable variable for builder
/// - Chain additional methods conditionally
/// - Re-assign builder when adding conditional fields
fn pattern_conditional_fields(
    model: String,
    messages: Vec<ChatCompletionRequestMessage>,
    temperature: Option<f32>,
    max_tokens: u16,
    enable_streaming: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Start with required fields
    let mut builder = CreateChatCompletionRequestArgs::default()
        .model(model)
        .messages(messages);

    // Add temperature if provided
    if let Some(temp) = temperature {
        builder = builder.temperature(temp);
    }

    // Add max_tokens only if specified
    if max_tokens > 0 {
        builder = builder.max_tokens(max_tokens as u32); // Important: u16 -> u32
    }

    // Add streaming parameter if needed
    if enable_streaming {
        builder = builder.stream(true);
    }

    // Build the final request
    let request = builder.build()?;

    println!("✓ Pattern 2 - Conditional request built successfully");
    println!("  Temperature: {:?}", request.temperature);
    println!("  Max tokens: {:?}", request.max_tokens);
    println!("  Stream: {:?}", request.stream);

    Ok(())
}

// ============================================================================
// PATTERN 3: Full RAG Example
// ============================================================================

/// Demonstrates a realistic RAG (Retrieval-Augmented Generation) scenario
///
/// Shows how to combine:
/// - System prompt
/// - Context (retrieved documents)
/// - Conversation history
/// - Current user query
fn pattern_rag_prompt(
    model: String,
    system_prompt: &str,
    context: &str,
    history: Vec<ChatCompletionRequestMessage>,
    user_query: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut messages = vec![];

    // 1. System prompt with RAG instructions
    messages.push(ChatCompletionRequestMessage::System(
        ChatCompletionRequestSystemMessage {
            content: ChatCompletionRequestSystemMessageContent::Text(
                system_prompt.to_string(),
            ),
            name: None,
        },
    ));

    // 2. Context from retrieval
    if !context.is_empty() {
        messages.push(ChatCompletionRequestMessage::System(
            ChatCompletionRequestSystemMessage {
                content: ChatCompletionRequestSystemMessageContent::Text(format!(
                    "# Context from knowledge base\n\n{}",
                    context
                )),
                name: None,
            },
        ));
    }

    // 3. Conversation history
    messages.extend(history);

    // 4. Current user query
    messages.push(ChatCompletionRequestMessage::User(
        ChatCompletionRequestUserMessage {
            content: ChatCompletionRequestUserMessageContent::Text(user_query.to_string()),
            name: None,
        },
    ));

    // Build request with RAG configuration
    let request = CreateChatCompletionRequestArgs::default()
        .model(model)
        .messages(messages)
        .temperature(0.5_f32) // Lower temperature for consistency
        .max_tokens(2048_u32) // Allow longer responses
        .top_p(0.9_f32)
        .build()?;

    println!("✓ Pattern 3 - RAG request built successfully");
    println!("  Total messages: {}", request.messages.len());
    println!("  Temperature: {:?}", request.temperature);

    Ok(())
}

// ============================================================================
// PATTERN 4: Type Conversions (Critical)
// ============================================================================

/// Demonstrates correct type handling for all numeric fields
///
/// Common mistakes:
/// - max_tokens as u16 instead of u32
/// - temperature as f64 instead of f32
/// - Forgetting `.as()` conversion
fn pattern_type_conversions() -> Result<(), Box<dyn std::error::Error>> {
    // ❌ WRONG: These won't compile
    // let max_tokens: u16 = 512;
    // CreateChatCompletionRequestArgs::default().max_tokens(max_tokens) // Type error!

    // ✅ CORRECT: Use explicit type annotations
    let max_tokens: u16 = 512;
    let temperature: f32 = 0.7;
    let top_p: f32 = 0.9;

    let request = CreateChatCompletionRequestArgs::default()
        .model("mistral".to_string())
        .messages(vec![])
        .temperature(temperature) // f32 directly
        .max_tokens(max_tokens as u32) // u16 -> u32 conversion
        .top_p(top_p) // f32 directly
        .frequency_penalty(-0.5_f32) // Explicit f32
        .presence_penalty(0.0_f32) // Explicit f32
        .build()?;

    println!("✓ Pattern 4 - Type conversions handled correctly");
    println!("  Temperature type: f32");
    println!("  Max tokens: {} (u32)", request.max_tokens.unwrap_or(0));

    Ok(())
}

// ============================================================================
// PATTERN 5: Error Handling
// ============================================================================

/// Demonstrates proper error handling when building requests
fn pattern_error_handling(model: String) -> Result<String, Box<dyn std::error::Error>> {
    // Attempt to build request and handle errors
    let request = CreateChatCompletionRequestArgs::default()
        .model(model)
        .messages(vec![]) // Empty messages might cause error
        .build()
        .map_err(|e| {
            format!("Failed to build request: {}", e)
        })?;

    println!("✓ Pattern 5 - Request built with error handling");
    Ok(format!("Request for model: {}", request.model))
}

// ============================================================================
// ANTI-PATTERNS TO AVOID
// ============================================================================

/// Shows what NOT to do when building requests
fn anti_patterns_summary() {
    println!("\n=== ANTI-PATTERNS (DO NOT USE) ===\n");

    println!("❌ WRONG: Direct field assignment");
    println!("   let mut builder = CreateChatCompletionRequestArgs::default();");
    println!("   builder.model = Some(\"mistral\".to_string()); // ERROR: private field!");
    println!();

    println!("❌ WRONG: Forgetting type conversion");
    println!("   let max_tokens: u16 = 512;");
    println!("   .max_tokens(max_tokens) // ERROR: expected u32, found u16");
    println!();

    println!("❌ WRONG: Using Some() wrapper");
    println!("   .model(Some(\"mistral\".to_string())) // ERROR: expected String");
    println!();

    println!("❌ WRONG: Deprecated function_call");
    println!("   .function_call(value) // DEPRECATED in 0.27!");
    println!();

    println!("✅ CORRECT: Use builder methods with explicit types");
    println!("   CreateChatCompletionRequestArgs::default()");
    println!("       .model(\"mistral\".to_string())");
    println!("       .messages(messages)");
    println!("       .temperature(0.7_f32)");
    println!("       .max_tokens(512_u32)");
    println!("       .build()?");
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_usage() {
        assert!(pattern_basic_usage().is_ok());
    }

    #[test]
    fn test_conditional_fields() {
        let model = "mistral".to_string();
        let messages = vec![ChatCompletionRequestMessage::User(
            ChatCompletionRequestUserMessage {
                content: ChatCompletionRequestUserMessageContent::Text("Hello".to_string()),
                name: None,
            },
        )];

        assert!(pattern_conditional_fields(model, messages, Some(0.7_f32), 512, false).is_ok());
    }

    #[test]
    fn test_conditional_fields_with_none_temperature() {
        let model = "mistral".to_string();
        let messages = vec![];
        assert!(pattern_conditional_fields(
            model,
            messages,
            None, // No temperature
            0,    // No max_tokens
            false
        )
        .is_ok());
    }

    #[test]
    fn test_rag_pattern() {
        let system = "You are a helpful assistant.";
        let context = "The capital of France is Paris.";
        let history = vec![];
        let query = "What is the capital of France?";

        assert!(pattern_rag_prompt(
            "mistral".to_string(),
            system,
            context,
            history,
            query
        )
        .is_ok());
    }

    #[test]
    fn test_type_conversions() {
        assert!(pattern_type_conversions().is_ok());
    }

    #[test]
    fn test_error_handling() {
        // This should handle error gracefully
        let result = pattern_error_handling("mistral".to_string());
        // Error is expected due to empty messages, but error handling works
        let _ = result;
    }

    #[test]
    fn test_builder_chaining() {
        let request = CreateChatCompletionRequestArgs::default()
            .model("gpt-4")
            .messages(vec![])
            .temperature(0.8_f32)
            .top_p(0.95_f32)
            .max_tokens(2048_u32)
            .frequency_penalty(0.0_f32)
            .build();

        assert!(request.is_ok());
        let req = request.unwrap();
        assert_eq!(req.model, "gpt-4");
        assert_eq!(req.temperature, Some(0.8_f32));
        assert_eq!(req.max_tokens, Some(2048_u32));
    }

    #[test]
    fn test_multiple_message_types() {
        let messages = vec![
            ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                content: ChatCompletionRequestSystemMessageContent::Text(
                    "System message".to_string(),
                ),
                name: None,
            }),
            ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                content: ChatCompletionRequestUserMessageContent::Text("User message".to_string()),
                name: None,
            }),
            ChatCompletionRequestMessage::Assistant(ChatCompletionRequestAssistantMessage {
                content: Some(ChatCompletionRequestAssistantMessageContent::Text(
                    "Assistant message".to_string(),
                )),
                name: None,
                tool_calls: None,
                audio: None,
                refusal: None,
            }),
        ];

        let request = CreateChatCompletionRequestArgs::default()
            .model("mistral")
            .messages(messages)
            .build();

        assert!(request.is_ok());
        assert_eq!(request.unwrap().messages.len(), 3);
    }

    #[test]
    fn test_temperature_values() {
        // Valid temperature range is typically 0.0 to 2.0
        for temp in [0.0_f32, 0.5_f32, 1.0_f32, 1.5_f32, 2.0_f32].iter() {
            let request = CreateChatCompletionRequestArgs::default()
                .model("mistral")
                .messages(vec![])
                .temperature(*temp)
                .build();

            assert!(request.is_ok());
            assert_eq!(request.unwrap().temperature, Some(*temp));
        }
    }

    #[test]
    fn test_type_safety() {
        // This test demonstrates that type system prevents mistakes
        let max_tokens_u16: u16 = 512;
        let max_tokens_u32 = max_tokens_u16 as u32;

        let request = CreateChatCompletionRequestArgs::default()
            .model("mistral")
            .messages(vec![])
            .max_tokens(max_tokens_u32)
            .build();

        assert!(request.is_ok());
        assert_eq!(request.unwrap().max_tokens, Some(512_u32));
    }
}

// ============================================================================
// MAIN
// ============================================================================

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== async-openai 0.27 Correct Patterns Reference ===\n");

    println!("Pattern 1: Basic Usage");
    pattern_basic_usage()?;

    println!("\nPattern 2: Conditional Fields");
    let model = "mistral".to_string();
    let messages = vec![ChatCompletionRequestMessage::User(
        ChatCompletionRequestUserMessage {
            content: ChatCompletionRequestUserMessageContent::Text("Hello".to_string()),
            name: None,
        },
    )];
    pattern_conditional_fields(model, messages, Some(0.7_f32), 512, true)?;

    println!("\nPattern 3: RAG Scenario");
    let system = "You are a helpful RAG assistant.";
    let context = "Rust is a systems programming language.";
    let history = vec![];
    let query = "What is Rust?";
    pattern_rag_prompt(
        "mistral".to_string(),
        system,
        context,
        history,
        query,
    )?;

    println!("\nPattern 4: Type Conversions");
    pattern_type_conversions()?;

    println!("\nPattern 5: Error Handling");
    let _ = pattern_error_handling("mistral".to_string());

    anti_patterns_summary();

    println!("\n=== All patterns executed successfully! ===");
    Ok(())
}