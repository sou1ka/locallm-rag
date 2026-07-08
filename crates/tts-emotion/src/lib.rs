//! tts-emotion: 感情抽出 + 文章分割 + TTS呼び出し
//!
//! LLM応答テキストをTTS発話単位に分割し、キーワードマッチで感情を推定、
//! OpenAI互換TTS APIで音声合成する部品クレート。
//! 疎結合な流用を想定しているため rag-core には依存しない。

use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

// ── 感情抽出 ──────────────────────────────────────────────────────────────────

/// 感情抽出結果
#[derive(Debug, Clone, PartialEq)]
pub struct Emotion {
    pub name: String,
    /// Live2Dモーション対応キー（Electron側で表情+モーション発火に使う）
    pub motion_id: Option<String>,
}

/// 感情ルール（emotion_rules.toml から読み込み）
#[derive(Debug, Deserialize)]
pub struct EmotionRules {
    default: String,
    #[serde(default)]
    emotions: Vec<EmotionRule>,
}

#[derive(Debug, Deserialize)]
struct EmotionRule {
    name: String,
    keywords: Vec<String>,
    motion_id: Option<String>,
}

impl EmotionRules {
    /// tomlファイルから読み込み
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(|e| {
            anyhow::anyhow!(
                "Failed to read emotion rules {}: {}",
                path.as_ref().display(),
                e
            )
        })?;
        Self::parse(&content)
    }

    /// toml文字列からパース
    pub fn parse(toml_str: &str) -> Result<Self> {
        toml::from_str(toml_str)
            .map_err(|e| anyhow::anyhow!("Failed to parse emotion rules: {}", e))
    }

    /// テキストから感情を抽出する
    ///
    /// キーワード出現数の合計が最多の感情を返す。同数は定義順が先の感情が勝つ。
    /// 無マッチ時は default 名の感情を返す（emotions に同名定義があればその motion_id を使う）。
    pub fn extract(&self, text: &str) -> Emotion {
        let mut best: Option<(&EmotionRule, usize)> = None;

        for rule in &self.emotions {
            let score: usize = rule
                .keywords
                .iter()
                .filter(|k| !k.is_empty())
                .map(|k| text.matches(k.as_str()).count())
                .sum();
            if score > 0 && best.map_or(true, |(_, s)| score > s) {
                best = Some((rule, score));
            }
        }

        match best {
            Some((rule, _)) => Emotion {
                name: rule.name.clone(),
                motion_id: rule.motion_id.clone(),
            },
            None => {
                let motion_id = self
                    .emotions
                    .iter()
                    .find(|r| r.name == self.default)
                    .and_then(|r| r.motion_id.clone());
                Emotion {
                    name: self.default.clone(),
                    motion_id,
                }
            }
        }
    }
}

// ── キャラクター ──────────────────────────────────────────────────────────────

/// キャラクター設定（chara/*.toml から読み込み）
#[derive(Debug, Deserialize)]
pub struct Character {
    pub name: String,                    // 表示名
    pub system_prompt: String,           // ペルソナのシステムプロンプト
    #[serde(default)]
    pub voice: Option<String>,           // TTSボイス上書き（省略可）
    #[serde(default)]
    pub emotion: Option<EmotionRules>,   // キャラ固有の感情ルール（省略可）
}

/// キャラクター設定のストア
pub struct CharacterStore {
    characters: HashMap<String, Character>,
}

impl CharacterStore {
    /// dir 内の *.toml を全て読み込む。
    /// - dir が読めない場合は Err
    /// - 個々のファイルがパース不能な場合は eprintln!("[WARN] ...") で警告して読み飛ばす
    ///   （rag-core の history.rs load_all と同じ流儀）
    /// - キャラIDはファイル名から拡張子を除いたもの（"naruto.toml" → "naruto"）
    /// - 拡張子が .toml 以外のファイルは無視
    pub fn load_dir(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref();
        let entries = std::fs::read_dir(dir)
            .map_err(|e| anyhow::anyhow!("Failed to read chara dir {}: {}", dir.display(), e))?;

        let mut characters = HashMap::new();

        for entry in entries {
            let entry =
                entry.map_err(|e| anyhow::anyhow!("Failed to read dir entry: {}", e))?;
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }

            let id = match path.file_stem().and_then(|s| s.to_str()) {
                Some(id) => id.to_string(),
                None => continue,
            };

            match std::fs::read_to_string(&path)
                .map_err(|e| anyhow::anyhow!("{}", e))
                .and_then(|content| {
                    toml::from_str::<Character>(&content)
                        .map_err(|e| anyhow::anyhow!("{}", e))
                }) {
                Ok(character) => {
                    characters.insert(id, character);
                }
                Err(e) => {
                    eprintln!("[WARN] Failed to load character {}: {}", path.display(), e)
                }
            }
        }

        Ok(Self { characters })
    }

    pub fn get(&self, id: &str) -> Option<&Character> {
        self.characters.get(id)
    }

    /// 読み込み済みキャラIDの一覧（昇順ソート済み）
    pub fn ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.characters.keys().cloned().collect();
        ids.sort();
        ids
    }

    pub fn len(&self) -> usize {
        self.characters.len()
    }

    pub fn is_empty(&self) -> bool {
        self.characters.is_empty()
    }
}

// ── 文章分割 ──────────────────────────────────────────────────────────────────

/// テキストをTTSに投げる発話単位に分割する
///
/// 区切り: 。！？!? と改行。区切り文字（改行以外）はセグメント末尾に含める。
/// 連続する区切り文字（"えっ！？"など）は1セグメントにまとめる。
/// 空・空白のみのセグメントは除外し、前後の空白はトリムする。
pub fn split_sentences(text: &str) -> Vec<String> {
    const DELIMITERS: [char; 5] = ['。', '！', '？', '!', '?'];

    fn flush(buf: &mut String, segments: &mut Vec<String>) {
        let trimmed = buf.trim();
        if !trimmed.is_empty() {
            segments.push(trimmed.to_string());
        }
        buf.clear();
    }

    let mut segments = Vec::new();
    let mut buf = String::new();
    let mut prev_was_delimiter = false;

    for c in text.chars() {
        if c == '\n' {
            flush(&mut buf, &mut segments);
            prev_was_delimiter = false;
        } else if DELIMITERS.contains(&c) {
            buf.push(c);
            prev_was_delimiter = true;
        } else {
            if prev_was_delimiter {
                flush(&mut buf, &mut segments);
            }
            buf.push(c);
            prev_was_delimiter = false;
        }
    }
    flush(&mut buf, &mut segments);

    segments
}

// ── TTS呼び出し ────────────────────────────────────────────────────────────────

/// TTS接続設定
#[derive(Debug, Clone, Deserialize)]
pub struct TtsConfig {
    /// OpenAI互換TTSエンドポイント（例: "http://127.0.0.1:8000/v1"）
    pub base_url: String,
    pub model: String,
    pub voice: String,
}

/// OpenAI互換TTSクライアント（POST {base_url}/audio/speech）
pub struct TtsClient {
    config: TtsConfig,
    client: reqwest::Client,
}

impl TtsClient {
    pub fn new(config: TtsConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// テキストを音声合成し、音声バイナリを返す
    pub async fn synthesize(&self, text: &str) -> Result<Vec<u8>> {
        self.synthesize_with_voice(text, None).await
    }

    /// テキストを音声合成する。voice が Some なら設定のボイスを上書きする
    pub async fn synthesize_with_voice(&self, text: &str, voice: Option<&str>) -> Result<Vec<u8>> {
        let url = format!(
            "{}/audio/speech",
            self.config.base_url.trim_end_matches('/')
        );

        let res = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "model": self.config.model,
                "input": text,
                "voice": voice.unwrap_or(&self.config.voice),
            }))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("TTS request failed: {}", e))?;

        if !res.status().is_success() {
            return Err(anyhow::anyhow!("TTS returned status {}", res.status()));
        }

        let bytes = res
            .bytes()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read TTS response: {}", e))?;

        Ok(bytes.to_vec())
    }
}

// ── ストリーミング分割バッファ ─────────────────────────────────────────────────

/// ストリーミングトークンを文単位に切り出すバッファ
pub struct SegmentBuffer {
    buf: String,
}

impl SegmentBuffer {
    pub fn new() -> Self {
        Self { buf: String::new() }
    }

    /// トークンを追加し、確定したセグメントを返す。
    /// バッファ末尾のセグメントは（次トークンで伸びる可能性があるため）保持する。
    pub fn push(&mut self, token: &str) -> Vec<String> {
        self.buf.push_str(token);

        if self.buf.ends_with('\n') {
            // 改行は明確な区切りのため、保留分も含めて全セグメントを確定する
            let segments = split_sentences(&self.buf);
            self.buf.clear();
            return segments;
        }

        let mut segments = split_sentences(&self.buf);
        if segments.len() >= 2 {
            let last = segments.pop().unwrap();
            self.buf = last;
            segments
        } else {
            Vec::new()
        }
    }

    /// ストリーム終了。残りのセグメントをすべて返す。
    pub fn finish(self) -> Vec<String> {
        split_sentences(&self.buf)
    }
}
