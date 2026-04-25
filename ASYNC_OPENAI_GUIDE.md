# async-openai 0.27 完全ガイド：CreateChatCompletionRequestArgs

> このドキュメントは、async-openai 0.27 で `CreateChatCompletionRequestArgs` を正しく使用するための完全なリファレンスです。

## 📑 目次

1. [問題の整理](#問題の整理)
2. [解決パターン](#解決パターン)
3. [実装例](#実装例)
4. [よくあるエラー](#よくあるエラー)
5. [プロジェクト資料](#プロジェクト資料)
6. [参考資料](#参考資料)

---

## 問題の整理

async-openai 0.27 を使用する際に、`CreateChatCompletionRequestArgs` で以下の問題が発生します：

### 問題 1: フィールドが private で直接設定できない

**❌ 間違った例**
```rust
let mut builder = CreateChatCompletionRequestArgs::default();
builder.model = Some("mistral".to_string());        // ❌ error: field `model` is private
builder.messages = Some(openai_messages);           // ❌ error: field `messages` is private
builder.temperature = Some(temperature);            // ❌ error: can't assign
```

**理由:** `CreateChatCompletionRequestArgs` のフィールドは意図的に `private` にされており、API の安定性と互換性を保つため、ビルダーメソッドを通じてのアクセスのみが許可されています。

---

### 問題 2: ビルダーメソッドのチェーンが一時値エラーになる可能性

複雑な条件分岐で一時値の所有権問題が発生することがあります。

**❌ 潜在的な問題**
```rust
let request_args = CreateChatCompletionRequestArgs::default()
    .model(model)
    .messages(messages);  // ← ここで一時値が返される

// 別の操作...

let request = request_args.build()?;  // ❌ 一時値が消滅している可能性
```

---

### 問題 3: function_call フィールドが deprecated だが、古いコードでは使用されている

async-openai 0.27 では `function_call` は deprecated です。最新の OpenAI API では `tool_calls` を使用します。

**❌ 古いパターン**
```rust
.function_call(some_value)  // ⚠️ deprecated
```

---

### 問題 4: 型の不一致

- `max_tokens` は `u32` が必須だが、コード内では `u16` で定義されている
- `temperature` は `f32` が必須だが、時々 `f64` が使用される

**❌ 型不正**
```rust
let max_tokens: u16 = 512;
.max_tokens(max_tokens)  // ❌ expected u32, found u16

let temp: f64 = 0.7;
.temperature(temp)       // ❌ expected f32, found f64
```

---

## 解決パターン

### ✅ パターン A: メソッドチェーン（最もシンプル）

すべてのパラメータが確定している場合に推奨します。

```rust
let request = CreateChatCompletionRequestArgs::default()
    .model("mistral".to_string())
    .messages(openai_messages)
    .temperature(0.7_f32)           // ← 明示的に f32
    .max_tokens(512_u32)            // ← 明示的に u32
    .top_p(0.9_f32)
    .build()
    .map_err(|e| anyhow!("Failed to build request: {}", e))?;
```

**利点:**
- ✅ シンプルで読みやすい
- ✅ 型安全性が高い
- ✅ 一時値エラーが発生しない
- ✅ 標準的なビルダーパターン

---

### ✅ パターン B: 可変ビルダー（条件付きパラメータ）

パラメータが条件に応じて変わる場合に推奨します。

```rust
// 必須フィールドで初期化
let mut builder = CreateChatCompletionRequestArgs::default()
    .model("mistral".to_string())
    .messages(openai_messages)
    .temperature(0.7_f32);

// 条件付きでパラメータを追加
if max_tokens > 0 {
    builder = builder.max_tokens(max_tokens as u32);  // ← u16 -> u32 変換
}

if enable_streaming {
    builder = builder.stream(true);
}

// 最終的にビルド
let request = builder
    .build()
    .map_err(|e| anyhow!("Failed to build request: {}", e))?;
```

**利点:**
- ✅ 条件付きロジックに対応
- ✅ 可変変数で中間状態を保持
- ✅ 型安全性を維持
- ✅ 複数の条件分岐に対応

---

### ✅ パターン C: RAG シナリオ（コンテキスト付き）

RAG（Retrieval-Augmented Generation）で検索結果を含める場合のパターン。

```rust
pub fn build_rag_request(
    model: String,
    system_prompt: String,
    context: String,              // ← 検索結果
    conversation_history: Vec<ChatCompletionRequestMessage>,
    current_query: String,
    max_tokens: u16,
) -> Result<CreateChatCompletionRequest> {
    let mut messages = vec![];

    // 1. システムプロンプト
    messages.push(ChatCompletionRequestMessage::System(
        ChatCompletionRequestSystemMessage {
            content: ChatCompletionRequestSystemMessageContent::Text(
                system_prompt,
            ),
            name: None,
        },
    ));

    // 2. コンテキスト（検索結果）
    if !context.is_empty() {
        messages.push(ChatCompletionRequestMessage::System(
            ChatCompletionRequestSystemMessage {
                content: ChatCompletionRequestSystemMessageContent::Text(
                    format!("# Context from Knowledge Base\n\n{}", context),
                ),
                name: None,
            },
        ));
    }

    // 3. 会話履歴
    messages.extend(conversation_history);

    // 4. 現在のクエリ
    messages.push(ChatCompletionRequestMessage::User(
        ChatCompletionRequestUserMessage {
            content: ChatCompletionRequestUserMessageContent::Text(current_query),
            name: None,
        },
    ));

    // ビルド
    let request = CreateChatCompletionRequestArgs::default()
        .model(model)
        .messages(messages)
        .temperature(0.7_f32)
        .max_tokens(max_tokens as u32)  // ← 型変換
        .top_p(0.9_f32)
        .build()?;

    Ok(request)
}
```

---

### ✅ パターン D: ストリーミング対応

リアルタイムでトークンを受信する場合のパターン。

```rust
pub async fn stream_completion(
    client: &Client<OpenAIConfig>,
    model: String,
    messages: Vec<ChatCompletionRequestMessage>,
    max_tokens: u16,
) -> Result<()> {
    // リクエスト構築
    let request = CreateChatCompletionRequestArgs::default()
        .model(model)
        .messages(messages)
        .temperature(0.7_f32)
        .max_tokens(max_tokens as u32)
        .stream(true)  // ← ストリーミング有効化
        .build()?;

    // ストリーム取得
    let mut stream = client.chat().create_stream(request).await?;

    // トークンごとに処理
    while let Some(result) = stream.next().await {
        let response = result?;
        if let Some(choice) = response.choices.first() {
            if let Some(content) = &choice.delta.content {
                print!("{}", content);
                io::stdout().flush()?;
            }
        }
    }

    Ok(())
}
```

---

## 実装例

### 例 1: プロジェクトの llm.rs に適用する

**修正前（❌ 間違った例）:**
```rust
pub async fn complete(
    &self,
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u16,
) -> crate::Result<String> {
    let openai_messages: Vec<ChatCompletionRequestMessage> =
        messages.iter().map(|m| m.to_openai_message()).collect();

    let mut builder = CreateChatCompletionRequestArgs::default();
    builder.model = Some(self.config.model.clone());        // ❌ 私有フィールド
    builder.messages = Some(openai_messages);               // ❌ 私有フィールド
    builder.temperature = Some(temperature);                // ❌ 割り当て不可
    if max_tokens > 0 {
        builder.max_tokens = Some(max_tokens);              // ❌ 型不正
    }

    let request = builder.build()?;
    // ...
}
```

**修正後（✅ 正しい例）:**
```rust
pub async fn complete(
    &self,
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u16,
) -> crate::Result<String> {
    let openai_messages: Vec<ChatCompletionRequestMessage> =
        messages.iter().map(|m| m.to_openai_message()).collect();

    // ✅ 可変ビルダーパターン
    let mut builder = CreateChatCompletionRequestArgs::default()
        .model(self.config.model.clone())
        .messages(openai_messages)
        .temperature(temperature);

    if max_tokens > 0 {
        builder = builder.max_tokens(max_tokens as u32);  // ✅ 型変換
    }

    let request = builder
        .build()
        .map_err(|e| crate::anyhow!("Failed to build request: {}", e))?;

    let response = self
        .client
        .chat()
        .create(request)
        .await
        .map_err(|e| crate::anyhow!("Failed to call API: {}", e))?;

    response
        .choices
        .first()
        .and_then(|choice| choice.message.content.clone())
        .ok_or_else(|| crate::anyhow!("No content in response"))
}
```

---

### 例 2: テストコード

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_building() {
        let request = CreateChatCompletionRequestArgs::default()
            .model("mistral".to_string())
            .messages(vec![ChatCompletionRequestMessage::User(
                ChatCompletionRequestUserMessage {
                    content: ChatCompletionRequestUserMessageContent::Text(
                        "Hello".to_string(),
                    ),
                    name: None,
                },
            )])
            .temperature(0.7_f32)
            .max_tokens(512_u32)
            .build();

        assert!(request.is_ok());
        let req = request.unwrap();
        assert_eq!(req.model, "mistral");
        assert_eq!(req.temperature, Some(0.7_f32));
        assert_eq!(req.max_tokens, Some(512_u32));
    }

    #[test]
    fn test_conditional_max_tokens() {
        let mut builder = CreateChatCompletionRequestArgs::default()
            .model("mistral".to_string())
            .messages(vec![]);

        let max_tokens: u16 = 1024;
        if max_tokens > 0 {
            builder = builder.max_tokens(max_tokens as u32);
        }

        let request = builder.build();
        assert!(request.is_ok());
        assert_eq!(request.unwrap().max_tokens, Some(1024_u32));
    }
}
```

---

## よくあるエラー

### エラー 1: `error: field 'model' is private`

**原因:** 直接フィールドに割り当てようとしている

```rust
// ❌ 間違い
builder.model = Some("mistral".to_string());

// ✅ 正解
builder = builder.model("mistral".to_string());
```

---

### エラー 2: `expected u32, found u16`

**原因:** `max_tokens` が `u32` であることを忘れている

```rust
// ❌ 間違い
let max_tokens: u16 = 512;
.max_tokens(max_tokens)

// ✅ 正解
.max_tokens(max_tokens as u32)
```

---

### エラー 3: `expected f32, found f64`

**原因:** `temperature` が `f64` ではなく `f32` である

```rust
// ❌ 間違い
let temp = 0.7;  // f64
.temperature(temp)

// ✅ 正解
.temperature(0.7_f32)
```

---

### エラー 4: `temporary value dropped while borrowed`

**原因:** 中間の一時値が消滅している

```rust
// ❌ 潜在的な問題
let builder = CreateChatCompletionRequestArgs::default()
    .model(model)
    .messages(messages);

// ビルダーが一時値として存在しなくなる

// ✅ 正解
let mut builder = CreateChatCompletionRequestArgs::default()
    .model(model)
    .messages(messages);

if condition {
    builder = builder.max_tokens(512_u32);
}

let request = builder.build()?;
```

---

## プロジェクト資料

このリポジトリには、以下の参考資料が含まれています：

### 1. `examples/async_openai_0_27_usage.rs`
- 5つの基本パターン
- 完全なコード例
- ビルドとテスト可能

実行方法:
```bash
cargo run --example async_openai_0_27_usage
```

### 2. `examples/llm_correct_patterns.rs`
- RAG対応の完全な実装
- パターン 1～5 の詳細な解説
- 包括的なテストスイート

実行方法:
```bash
cargo run --example llm_correct_patterns
cargo test --example llm_correct_patterns
```

### 3. `REFACTORING_GUIDE.md`
- `crates/rag-core/src/llm.rs` の修正方法
- Before/After の詳細な比較
- migration チェックリスト

### 4. `CHEATSHEET.md`
- クイックリファレンス
- よくあるエラーと対策表
- 実装パターン集

---

## 参考資料

### 公式ドキュメント
- [async-openai on docs.rs (0.27)](https://docs.rs/async-openai/0.27/async_openai/)
- [OpenAI API Reference](https://platform.openai.com/docs/api-reference/chat/create)
- [Rust Builder Pattern](https://rust-lang.github.io/api-guidelines/type-safety.html)

### プロジェクト内の関連ファイル
- `crates/rag-core/src/llm.rs` - メイン実装
- `crates/rag-core/src/lib.rs` - ライブラリエントリーポイント
- `Cargo.toml` - 依存関係定義

### コマンドリファレンス

```bash
# ビルド確認
cargo check -p rag-core

# テスト実行
cargo test -p rag-core llm

# ドキュメント生成
cargo doc -p rag-core --open

# サンプル実行
cargo run --example async_openai_0_27_usage
cargo run --example llm_correct_patterns

# 警告確認
cargo clippy -p rag-core
```

---

## チェックリスト: 修正を始める前に

- [ ] `async-openai = "0.27"` が Cargo.toml に指定されている
- [ ] 現在のコードが `builder.field = Some(...)` パターンを使用している
- [ ] `max_tokens` が `u16` で定義されている
- [ ] 修正後、`cargo check` でコンパイルが通るか確認する予定
- [ ] テストを実行して動作確認する予定

---

## クイックスタート

### 1. 基本パターンの確認
```rust
let request = CreateChatCompletionRequestArgs::default()
    .model("mistral".to_string())
    .messages(messages)
    .temperature(0.7_f32)
    .max_tokens(512_u32)
    .build()?;
```

### 2. 条件付きの場合
```rust
let mut builder = CreateChatCompletionRequestArgs::default()
    .model(model)
    .messages(messages);

if condition {
    builder = builder.max_tokens(512_u32);
}

let request = builder.build()?;
```

### 3. RAG シナリオ
```
system_prompt + context + history + query
            ↓
    messages vec
            ↓
    CreateChatCompletionRequestArgs
            ↓
           build()
```

---

## サポート

質問や問題が発生した場合：

1. **基本パターンが動作するか確認**
   - `examples/async_openai_0_27_usage.rs` を実行
   
2. **自分のコードと比較**
   - `CHEATSHEET.md` のパターンと照合
   
3. **エラーメッセージを確認**
   - [よくあるエラー](#よくあるエラー) セクションを参照

4. **テストを書く**
   - サンプルのテストコードを参考にテストを追加

---

## まとめ

### ✅ DO's

- ✅ ビルダーメソッドを使用（`.model()`, `.messages()` など）
- ✅ 型を明示的に指定（`0.7_f32`, `512_u32`）
- ✅ 条件付きは可変ビルダーで再割り当て
- ✅ `.build()` は最後に1回だけ

### ❌ DON'Ts

- ❌ 直接フィールド割り当て（private フィールド）
- ❌ 型指定なしの数値
- ❌ `Some()` でのラッピング
- ❌ deprecated `function_call` 使用

---

**最終確認:** すべてのコンパイルが通り、テストが成功すれば、パターンは正しい ✅