# async-openai 0.27 CreateChatCompletionRequestArgs チートシート

## 🎯 クイックスタート

### ✅ 正しいパターン（コピペして使用可能）

```rust
// パターン A: シンプル（メソッドチェーン）
let request = CreateChatCompletionRequestArgs::default()
    .model("mistral".to_string())
    .messages(messages)
    .temperature(0.7_f32)
    .max_tokens(512_u32)
    .build()?;

// パターン B: 条件付き（推奨）
let mut builder = CreateChatCompletionRequestArgs::default()
    .model(model)
    .messages(messages)
    .temperature(temperature);

if max_tokens > 0 {
    builder = builder.max_tokens(max_tokens as u32);
}

let request = builder.build()?;
```

---

## ❌ よくある間違いと修正

| ❌ 間違い | ✅ 修正 | 理由 |
|---------|--------|------|
| `builder.model = Some(...)` | `.model(...)` | フィールドは private |
| `builder.temperature = Some(...)` | `.temperature(...)` | メソッドチェーンを使用 |
| `.max_tokens(512_u16)` | `.max_tokens(512_u32)` | u32 が必須 |
| `.temperature(0.7_f64)` | `.temperature(0.7_f32)` | f32 が必須 |
| `.function_call(value)` | 削除（不要） | async-openai 0.27 で deprecated |
| `CreateChatCompletionRequestArgs::default().model(...)` 単独 | `.build()?.` で終了 | ビルダーは Request を返さない |

---

## 📝 型変換ガイド

### max_tokens
```rust
let max_tokens: u16 = 512;
.max_tokens(max_tokens as u32)  // ✅ u16 -> u32
```

### temperature / top_p
```rust
let temp: f32 = 0.7;
.temperature(temp)              // ✅ f32 直接
// または
.temperature(0.7_f32)           // ✅ リテラルで f32 指定
```

### frequency_penalty / presence_penalty
```rust
.frequency_penalty(0.0_f32)     // ✅ f32
.presence_penalty(0.0_f32)      // ✅ f32
```

---

## 🔄 メッセージの構築

### 単一メッセージ
```rust
ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
    content: ChatCompletionRequestUserMessageContent::Text("質問".to_string()),
    name: None,
})
```

### システムメッセージ + 複数メッセージ
```rust
vec![
    ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
        content: ChatCompletionRequestSystemMessageContent::Text(
            "You are helpful.".to_string()
        ),
        name: None,
    }),
    ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
        content: ChatCompletionRequestUserMessageContent::Text("Hello".to_string()),
        name: None,
    }),
    ChatCompletionRequestMessage::Assistant(ChatCompletionRequestAssistantMessage {
        content: Some(ChatCompletionRequestAssistantMessageContent::Text(
            "Hi there!".to_string()
        )),
        name: None,
        tool_calls: None,
        audio: None,
        refusal: None,
    }),
]
```

---

## 🛠️ 実装パターン集

### RAG シナリオ
```rust
fn build_rag_request(
    model: String,
    system_prompt: String,
    context: String,
    history: Vec<ChatCompletionRequestMessage>,
    query: String,
    max_tokens: u16,
) -> Result<CreateChatCompletionRequest, Box<dyn Error>> {
    let mut messages = vec![
        ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
            content: ChatCompletionRequestSystemMessageContent::Text(system_prompt),
            name: None,
        }),
    ];

    // Context
    if !context.is_empty() {
        messages.push(ChatCompletionRequestMessage::System(
            ChatCompletionRequestSystemMessage {
                content: ChatCompletionRequestSystemMessageContent::Text(
                    format!("# Context\n\n{}", context)
                ),
                name: None,
            },
        ));
    }

    // History + Query
    messages.extend(history);
    messages.push(ChatCompletionRequestMessage::User(
        ChatCompletionRequestUserMessage {
            content: ChatCompletionRequestUserMessageContent::Text(query),
            name: None,
        },
    ));

    // Build
    let request = CreateChatCompletionRequestArgs::default()
        .model(model)
        .messages(messages)
        .temperature(0.7_f32)
        .max_tokens(max_tokens as u32)
        .build()?;

    Ok(request)
}
```

### ストリーミング
```rust
let request = CreateChatCompletionRequestArgs::default()
    .model(model)
    .messages(messages)
    .temperature(0.7_f32)
    .max_tokens(512_u32)
    .stream(true)  // ← ストリーミング有効
    .build()?;

let mut stream = client.chat().create_stream(request).await?;

while let Some(result) = stream.next().await {
    let response = result?;
    if let Some(choice) = response.choices.first() {
        if let Some(content) = &choice.delta.content {
            println!("{}", content);
        }
    }
}
```

---

## 📊 パラメータリファレンス

| パラメータ | 型 | デフォルト | 範囲 | 用途 |
|-----------|----|---------|----|------|
| `model` | String | 必須 | - | 使用モデル |
| `messages` | Vec<...> | 必須 | - | 会話履歴 |
| `temperature` | f32 | 1.0 | 0.0-2.0 | 創意性（低=確定的、高=ランダム） |
| `max_tokens` | u32 | - | 1-∞ | 最大レスポンス長 |
| `top_p` | f32 | 1.0 | 0.0-1.0 | Nucleus sampling |
| `frequency_penalty` | f32 | 0.0 | -2.0-2.0 | 繰り返し抑制 |
| `presence_penalty` | f32 | 0.0 | -2.0-2.0 | 新トピック促進 |

---

## 🧪 テンプレート

### 単体テスト
```rust
#[test]
fn test_build_request() {
    let request = CreateChatCompletionRequestArgs::default()
        .model("mistral".to_string())
        .messages(vec![])
        .temperature(0.7_f32)
        .max_tokens(512_u32)
        .build();

    assert!(request.is_ok());
    let req = request.unwrap();
    assert_eq!(req.model, "mistral");
    assert_eq!(req.max_tokens, Some(512));
    assert_eq!(req.temperature, Some(0.7_f32));
}
```

### エラーハンドリング
```rust
let request = CreateChatCompletionRequestArgs::default()
    .model(model)
    .messages(messages)
    .build()
    .map_err(|e| anyhow!("Failed to build request: {}", e))?;
```

---

## 🎓 重要な補足

### なぜ私有フィールドなのか？
`CreateChatCompletionRequestArgs` のフィールドは private にしてあります。これにより：
- API が変わっても互換性を保ちやすい
- ビルダーメソッドで検証が可能
- 型安全性が向上

### max_tokens を u32 にする理由
OpenAI API の仕様が u32 のため。u16（最大 65,535）では足りない場合があります。

### function_call はなぜ deprecated？
最新の OpenAI API では `tool_calls` を使用します。本プロジェクト（Ollama）では通常不要。

---

## 🚀 実行例

```bash
# サンプルコード実行
cargo run --example async_openai_0_27_usage

# テスト実行
cargo test llm

# 正しいパターン実行
cargo run --example llm_correct_patterns
```

---

## 📚 関連ファイル

| ファイル | 用途 |
|---------|------|
| `examples/async_openai_0_27_usage.rs` | 詳細な使用例 5パターン |
| `examples/llm_correct_patterns.rs` | 実装パターン + テスト |
| `REFACTORING_GUIDE.md` | 詳細なリファクタリングガイド |
| `crates/rag-core/src/llm.rs` | 実装ファイル |

---

## 🔗 型推論の確認方法

```rust
// 型を明示的に表示するには
let _: () = CreateChatCompletionRequestArgs::default()
    .model("mistral".to_string())
    .messages(vec![]);
// コンパイラが型エラーで正しい型を提示
```

---

## 💡 TIP: 便利な定数パターン

```rust
const DEFAULT_TEMPERATURE: f32 = 0.7;
const DEFAULT_MAX_TOKENS: u32 = 512;
const DEFAULT_TOP_P: f32 = 0.9;

let request = CreateChatCompletionRequestArgs::default()
    .model(model)
    .messages(messages)
    .temperature(DEFAULT_TEMPERATURE)
    .max_tokens(DEFAULT_MAX_TOKENS)
    .top_p(DEFAULT_TOP_P)
    .build()?;
```

---

## ⚡ 一行コマンド

```bash
# ビルド確認
cargo check -p rag-core

# clippy で警告確認
cargo clippy -p rag-core

# ドキュメント生成
cargo doc -p rag-core --open
```
