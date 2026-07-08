#[cfg(test)]
mod tests {
    use tts_emotion::{Emotion, EmotionRules, split_sentences, TtsClient, TtsConfig};

    // ============ EmotionRules::parse のテスト ============

    #[test]
    fn parse_正常系_基本的なtoml形式() {
        let toml_str = r#"
            default = "neutral"

            [[emotions]]
            name = "joy"
            keywords = ["嬉しい", "楽しい"]
            motion_id = "joy"
        "#;

        let result = EmotionRules::parse(toml_str);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_正常系_複数感情定義() {
        let toml_str = r#"
            default = "neutral"

            [[emotions]]
            name = "joy"
            keywords = ["嬉しい", "楽しい"]
            motion_id = "joy"

            [[emotions]]
            name = "sad"
            keywords = ["悲しい"]
            motion_id = "sad"
        "#;

        let result = EmotionRules::parse(toml_str);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_正常系_motion_id_省略() {
        let toml_str = r#"
            default = "neutral"

            [[emotions]]
            name = "surprise"
            keywords = ["驚き"]
        "#;

        let result = EmotionRules::parse(toml_str);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_異常系_不正なtoml形式() {
        let toml_str = r#"
            default = "neutral"
            [[emotions
        "#;

        let result = EmotionRules::parse(toml_str);
        assert!(result.is_err());
    }

    #[test]
    fn parse_異常系_default_キー欠落() {
        let toml_str = r#"
            [[emotions]]
            name = "joy"
            keywords = ["嬉しい"]
            motion_id = "joy"
        "#;

        let result = EmotionRules::parse(toml_str);
        assert!(result.is_err());
    }

    // ============ EmotionRules::extract のテスト ============

    fn create_test_rules() -> EmotionRules {
        let toml_str = r#"
            default = "neutral"

            [[emotions]]
            name = "joy"
            keywords = ["嬉しい", "楽しい", "ありがとう", "やった"]
            motion_id = "joy"

            [[emotions]]
            name = "sad"
            keywords = ["悲しい", "残念", "ごめん"]
            motion_id = "sad"

            [[emotions]]
            name = "surprise"
            keywords = ["驚き", "びっくり", "まさか"]
            motion_id = "surprise"
        "#;

        EmotionRules::parse(toml_str).unwrap()
    }

    #[test]
    fn extract_正常系_joy単一キーワード() {
        let rules = create_test_rules();
        let result = rules.extract("これは嬉しいです");

        assert_eq!(result.name, "joy");
        assert_eq!(result.motion_id, Some("joy".to_string()));
    }

    #[test]
    fn extract_正常系_sad単一キーワード() {
        let rules = create_test_rules();
        let result = rules.extract("とても悲しいです");

        assert_eq!(result.name, "sad");
        assert_eq!(result.motion_id, Some("sad".to_string()));
    }

    #[test]
    fn extract_正常系_surprise単一キーワード() {
        let rules = create_test_rules();
        let result = rules.extract("びっくりしました");

        assert_eq!(result.name, "surprise");
        assert_eq!(result.motion_id, Some("surprise".to_string()));
    }

    #[test]
    fn extract_正常系_複数キーワード同一感情() {
        let rules = create_test_rules();
        let result = rules.extract("楽しい楽しいやった");

        assert_eq!(result.name, "joy");
    }

    #[test]
    fn extract_正常系_複数感情キーワード混在_多い方が勝つ() {
        let rules = create_test_rules();
        let result = rules.extract("嬉しい楽しいありがとう悲しい残念");

        // joy: 3個（嬉しい、楽しい、ありがとう）
        // sad: 2個（悲しい、残念）
        assert_eq!(result.name, "joy");
        assert_eq!(result.motion_id, Some("joy".to_string()));
    }

    #[test]
    fn extract_正常系_複数感情同数_定義順が優先() {
        let rules = create_test_rules();
        let result = rules.extract("嬉しい悲しい");

        // joy: 1個（嬉しい）
        // sad: 1個（悲しい）
        // joy が先に定義されているので joy が勝つ
        assert_eq!(result.name, "joy");
        assert_eq!(result.motion_id, Some("joy".to_string()));
    }

    #[test]
    fn extract_正常系_複数感情同数_中間の感情() {
        let rules = create_test_rules();
        let result = rules.extract("楽しい驚き");

        // joy: 1個（楽しい）
        // surprise: 1個（驚き）
        // joy が先に定義されているので joy が勝つ
        assert_eq!(result.name, "joy");
    }

    #[test]
    fn extract_正常系_無マッチ_defaultの感情が定義されている() {
        let toml_str = r#"
            default = "neutral"

            [[emotions]]
            name = "joy"
            keywords = ["嬉しい"]
            motion_id = "joy"

            [[emotions]]
            name = "neutral"
            keywords = []
            motion_id = "neutral"
        "#;
        let rules = EmotionRules::parse(toml_str).unwrap();
        let result = rules.extract("こんにちは");

        assert_eq!(result.name, "neutral");
        assert_eq!(result.motion_id, Some("neutral".to_string()));
    }

    #[test]
    fn extract_正常系_無マッチ_defaultの感情が定義されていない() {
        let rules = create_test_rules();
        let result = rules.extract("こんにちは");

        // default = "neutral" だが、emotions に "neutral" が定義されていない
        assert_eq!(result.name, "neutral");
        assert_eq!(result.motion_id, None);
    }

    #[test]
    fn extract_正常系_motion_id_省略の感情がマッチ() {
        let toml_str = r#"
            default = "neutral"

            [[emotions]]
            name = "surprise"
            keywords = ["驚き"]
        "#;
        let rules = EmotionRules::parse(toml_str).unwrap();
        let result = rules.extract("驚きました");

        assert_eq!(result.name, "surprise");
        assert_eq!(result.motion_id, None);
    }

    #[test]
    fn extract_正常系_空文字列_default返す() {
        let rules = create_test_rules();
        let result = rules.extract("");

        assert_eq!(result.name, "neutral");
        assert_eq!(result.motion_id, None);
    }

    #[test]
    fn extract_正常系_空白のみ_default返す() {
        let rules = create_test_rules();
        let result = rules.extract("   ");

        assert_eq!(result.name, "neutral");
    }

    #[test]
    fn extract_正常系_部分マッチ_キーワード内の一部() {
        let rules = create_test_rules();
        // "嬉しい" がテキストに含まれている
        let result = rules.extract("嬉しいことがあった");

        assert_eq!(result.name, "joy");
    }

    #[test]
    fn extract_正常系_複数回出現() {
        let rules = create_test_rules();
        let result = rules.extract("嬉しい嬉しい嬉しい");

        // 同じキーワードが複数回出現してもカウントは変わらない（1回カウント）
        // または複数回カウントされる可能性もある
        // 仕様では「出現したキーワードの種類数」ではなく「カウント」なので複数回カウント
        assert_eq!(result.name, "joy");
    }

    // ============ split_sentences のテスト ============

    #[test]
    fn split_sentences_正常系_句点で分割() {
        let result = split_sentences("こんにちは。元気ですか。");

        assert_eq!(result, vec!["こんにちは。", "元気ですか。"]);
    }

    #[test]
    fn split_sentences_正常系_疑問符で分割() {
        let result = split_sentences("本当？そうです。");

        assert_eq!(result, vec!["本当？", "そうです。"]);
    }

    #[test]
    fn split_sentences_正常系_感嘆符で分割() {
        let result = split_sentences("すごい！素晴らしい！");

        assert_eq!(result, vec!["すごい！", "素晴らしい！"]);
    }

    #[test]
    fn split_sentences_正常系_改行で分割() {
        let result = split_sentences("第一行\n第二行");

        assert_eq!(result, vec!["第一行", "第二行"]);
    }

    #[test]
    fn split_sentences_正常系_複数改行() {
        let result = split_sentences("行1\n\n行2");

        // 空セグメントは除外
        assert_eq!(result, vec!["行1", "行2"]);
    }

    #[test]
    fn split_sentences_正常系_複数の区切り文字混在() {
        let result = split_sentences("どうした！本当？素晴らしい。");

        assert_eq!(result, vec!["どうした！", "本当？", "素晴らしい。"]);
    }

    #[test]
    fn split_sentences_正常系_連続する区切り文字() {
        let result = split_sentences("えっ！？");

        // 連続する区切り文字は1つのセグメントにまとめられる
        assert_eq!(result, vec!["えっ！？"]);
    }

    #[test]
    fn split_sentences_正常系_連続する区切り文字複数() {
        let result = split_sentences("ええ！？！？");

        assert_eq!(result, vec!["ええ！？！？"]);
    }

    #[test]
    fn split_sentences_正常系_前後の空白トリム() {
        let result = split_sentences("  こんにちは。  元気？  ");

        assert_eq!(result, vec!["こんにちは。", "元気？"]);
    }

    #[test]
    fn split_sentences_正常系_セグメント内の空白は保持() {
        let result = split_sentences("こん に ちは。");

        assert_eq!(result, vec!["こん に ちは。"]);
    }

    #[test]
    fn split_sentences_正常系_区切り文字なし() {
        let result = split_sentences("これはテキストです");

        assert_eq!(result, vec!["これはテキストです"]);
    }

    #[test]
    fn split_sentences_正常系_空文字列() {
        let result = split_sentences("");

        assert_eq!(result, Vec::<String>::new());
    }

    #[test]
    fn split_sentences_正常系_空白のみ() {
        let result = split_sentences("   ");

        assert_eq!(result, Vec::<String>::new());
    }

    #[test]
    fn split_sentences_正常系_改行のみ() {
        let result = split_sentences("\n\n");

        assert_eq!(result, Vec::<String>::new());
    }

    #[test]
    fn split_sentences_正常系_末尾の句点() {
        let result = split_sentences("終わりです。");

        assert_eq!(result, vec!["終わりです。"]);
    }

    #[test]
    fn split_sentences_正常系_複雑なケース() {
        let result = split_sentences("第一文。\n第二文！？\n\n第三文");

        assert_eq!(result, vec!["第一文。", "第二文！？", "第三文"]);
    }

    #[test]
    fn split_sentences_正常系_半角句点と全角句点() {
        let result = split_sentences("これ。それ");

        assert_eq!(result, vec!["これ。", "それ"]);
    }

    #[test]
    fn split_sentences_正常系_連続する改行と句点() {
        let result = split_sentences("文。\n\n文");

        assert_eq!(result, vec!["文。", "文"]);
    }

    #[test]
    fn split_sentences_正常系_空白を含む連続区切り文字() {
        let result = split_sentences("あ ! ?");

        // "! ?" のように空白が挟まっている場合
        // 仕様では「連続する区切り文字」がどう定義されるか曖昧だが
        // 空白を挟んでいたら別セグメントになる可能性が高い
        // テストの目的上、最も自然な動作を想定
        assert_eq!(result, vec!["あ !", "?"]);
    }

    #[test]
    fn split_sentences_正常系_Unicode句点() {
        let result = split_sentences("こんにちは。世界。");

        assert_eq!(result, vec!["こんにちは。", "世界。"]);
    }

    // ============ TtsClient のテスト ============

    #[test]
    fn tts_client_new_基本() {
        let config = TtsConfig {
            base_url: "http://127.0.0.1:8000/v1".to_string(),
            model: "tts-1".to_string(),
            voice: "alloy".to_string(),
        };

        let _client = TtsClient::new(config);
        // インスタンス生成できることを確認
    }

    #[test]
    fn tts_client_new_異なるURLとボイス() {
        let config = TtsConfig {
            base_url: "https://api.openai.com/v1".to_string(),
            model: "tts-1-hd".to_string(),
            voice: "nova".to_string(),
        };

        let _client = TtsClient::new(config);
        // インスタンス生成できることを確認
    }

    #[test]
    fn tts_client_new_複数インスタンス() {
        let config1 = TtsConfig {
            base_url: "http://localhost:8000".to_string(),
            model: "tts-1".to_string(),
            voice: "alloy".to_string(),
        };

        let config2 = TtsConfig {
            base_url: "http://localhost:8001".to_string(),
            model: "tts-2".to_string(),
            voice: "echo".to_string(),
        };

        let _client1 = TtsClient::new(config1);
        let _client2 = TtsClient::new(config2);
        // 複数のインスタンスを同時に生成できることを確認
    }
}
