# async-openai 0.27 Refactoring Guide

## 概要

このガイドはプロジェクトの `crates/rag-core/src/llm.rs` を async-openai 0.27 の正しいパターンに変更するためのものです。

## 現在のコードの問題

### 問題 1: 直接フィールド割り当て

**現在のコード（✗ 間違い）:**
```rust
let mut builder = CreateChatCompletionRequestArgs::default();
builder.model = Some(self.config.model.clone());      // ❌ Private field
builder.messages = Some(openai_messages);             // ❌ Private field
builder.temperature = Some(temperature);               // ❌ Private field
if max_tokens > 0 {
    builder.max_tokens = Some(max_tokens);            // ❌ Private field
}
let request = builder.build()?;
```

**理由:** `CreateChatCompletionRequestArgs` のフィールドは `private` です。直接割り当てはできません。

### 問題 2: 型の不一致

- `temperature`: `f32` が必要だが、時々 `f64` が渡される
- `max_tokens`: `u32` が必要だが、コード内では `u16` で定義
- ビルダーメソッド呼び出しの戻り値を正しく処理していない

## 解決策

### パターン A: シンプルな場合（メソッドチェーン）

**推奨コード（✅ 正解）:**
```rust
let request = CreateChatCompletionRequestArgs::default()
    .model(self.config.model.clone())
    .messages(openai_messages)
    .temperature(temperature)
    .max_tokens(max_tokens as u32)  // Important: u16 -> u32 conversion
    .build()
    .map_err(|e| crate::anyhow!("Failed to build request: {}", e))?;
```

**利点:**
- ✅ ビルダーメソッドを使用（public API）
- ✅ 型は自動推論される
- ✅ 一時値の問題なし
- ✅ 簡潔で読みやすい

### パターン B: 条件付きパラメータがある場合

**推奨コード（✅ 正解）:**
```rust
let mut request_builder = CreateChatCompletionRequestArgs::default()
    .model(self.config.model.clone())
    .messages(openai_messages)
    .temperature(temperature);

// 条件付きでmax_tokensを追加
if max_tokens > 0 {
    request_builder = request_builder.max_tokens(max_tokens as u32);
}

let request = request_builder
    .build()
    .map_err(|e| crate::anyhow!("Failed to build request: {}", e))?;
```

**ポイント:**
- ✅ 可変変数で中間状態を保持
- ✅ 条件分岐後に再割り当て
- ✅ 一時値エラーの回避

## llm.rs での具体的な修正内容

### 修正 1: `complete()` メソッド

**Before:**
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
    builder.model = Some(self.config.model.clone());
    builder.messages = Some(openai_messages);
    builder.temperature = Some(temperature);
    if max_tokens > 0 {
        builder.max_tokens = Some(max_tokens);
    }

    let request = builder
        .build()
        .map_err(|e| crate::anyhow!("Failed to build chat completion request: {}", e))?;
    
    // ...
}
```

**After:**
```rust
pub async fn complete(
    &self,
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u16,
) -> crate::Result<String> {
    let openai_messages: Vec<ChatCompletionRequestMessage> =
        messages.iter().map(|m| m.to_openai_message()).collect();

    let mut request_builder = CreateChatCompletionRequestArgs::default()
        .model(self.config.model.clone())
        .messages(openai_messages)
        .temperature(temperature);

    if max_tokens > 0 {
        request_builder = request_builder.max_tokens(max_tokens as u32);
    }

    let request = request_builder
        .build()
        .map_err(|e| crate::anyhow!("Failed to build chat completion request: {}", e))?;
    
    // ...
}
```

**変更点:**
1. `CreateChatCompletionRequestArgs::default()` 直後からメソッドチェーン開始
2. `builder.model = Some(...)` を `.model(...)` メソッドに変更
3. `max_tokens` を `u16` から `u32` に変換（`as u32`）
4. 可変ビルダーで条件付きフィールド追加

### 修正 2: `complete_stream()` メソッド

同様の修正を `complete_stream()` メソッドにも適用します：

**Before:**
```rust
let mut builder = CreateChatCompletionRequestArgs::default();
builder.model = Some(self.config.model.clone());
builder.messages = Some(openai_messages);
builder.temperature = Some(temperature);
if max_tokens > 0 {
    builder.max_tokens = Some(max_tokens);
}

let request = builder.build()?;
```

**After:**
```rust
let mut request_builder = CreateChatCompletionRequestArgs::default()
    .model(self.config.model.clone())
    .messages(openai_messages)
    .temperature(temperature);

if max_tokens > 0 {
    request_builder = request_builder.max_tokens(max_tokens as u32);
}

let request = request_builder.build()?;
```

## 型変換リファレンス

| パラメータ | 正しい型 | 修正方法 |
|-----------|--------|--------|
| `model` | `String` | `.model(model_string)` |
| `messages` | `Vec<ChatCompletionRequestMessage>` | `.messages(messages_vec)` |
| `temperature` | `f32` | `.temperature(value as f32)` |
| `max_tokens` | `u32` | `.max_tokens(value as u32)` |
| `top_p` | `f32` | `.top_p(value as f32)` |
| `frequency_penalty` | `f32` | `.frequency_penalty(value as f32)` |
| `presence_penalty` | `f32` | `.presence_penalty(value as f32)` |

## 修正の検証

### コンパイルチェック
```bash
cd crates/rag-core
cargo build
```

### テストの実行
```bash
cargo test llm
```

### サンプルの実行
```bash
cargo run --example async_openai_0_27_usage
```

## よくある間違いと対策

### 間違い 1: `Some()` ラッピング
```rust
// ❌ 間違い
builder.temperature = Some(0.7_f32);

// ✅ 正解
builder = builder.temperature(0.7_f32);
```

### 間違い 2: 型の不一致
```rust
// ❌ 間違い
let max_tokens: u16 = 512;
.max_tokens(max_tokens)  // Expected u32

// ✅ 正解
.max_tokens(max_tokens as u32)
```

### 間違い 3: 一時値エラー
```rust
// ❌ 間違い
let request = CreateChatCompletionRequestArgs::default()
    .model("mistral")
    .messages(messages)  // この時点で一時値が返される
    .build()?;           // 一時値の使用


// ✅ 正解
let request = CreateChatCompletionRequestArgs::default()
    .model("mistral")
    .messages(messages)
    .build()?;
```

### 間違い 4: deprecated `function_call`
```rust
// ❌ 間違い（deprecated）
.function_call(some_value)

// ✅ 正解
// function_call は 0.27 では不要。tool_calls を使用する場合は
// ChatCompletionRequestAssistantMessage の tool_calls フィールドを使用
```

## migration チェックリスト

- [ ] `complete()` メソッドをビルダーパターンに変更
- [ ] `complete_stream()` メソッドをビルダーパターンに変更
- [ ] `max_tokens` の型を `u16` -> `u32` に変換
- [ ] 直接フィールド割り当てをすべて削除
- [ ] `temperature` と `top_p` が `f32` 型であることを確認
- [ ] `cargo build` でコンパイルエラーがないことを確認
- [ ] `cargo test` ですべてのテストが通ることを確認
- [ ] コード内の deprecated パターンをすべて削除

## リソース

- [async-openai Documentation](https://docs.rs/async-openai/latest/async_openai/)
- [OpenAI API Reference](https://platform.openai.com/docs/api-reference/chat/create)
- [プロジェクトサンプル](./examples/async_openai_0_27_usage.rs)

## まとめ

async-openai 0.27 での `CreateChatCompletionRequestArgs` の正しい使用方法：

```rust
// パターン A: シンプル（メソッドチェーン）
let request = CreateChatCompletionRequestArgs::default()
    .model(model_name)
    .messages(messages)
    .temperature(0.7_f32)
    .max_tokens(1024_u32)
    .build()?;

// パターン B: 条件付き（可変ビルダー）
let mut builder = CreateChatCompletionRequestArgs::default()
    .model(model_name)
    .messages(messages);

if should_set_tokens {
    builder = builder.max_tokens(1024_u32);
}

let request = builder.build()?;
```

重要なポイント：
- ✅ ビルダーメソッドを使用（private フィールドへのアクセス不可）
- ✅ 明示的な型指定（f32、u32）
- ✅ メソッドチェーンまたは可変変数パターン
- ✅ `.build()` で最終化
- ✅ deprecated フィールドの避回