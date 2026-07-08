use tts_emotion::SegmentBuffer;

#[test]
fn 基本的なストリーミング() {
    let mut buf = SegmentBuffer::new();
    assert_eq!(buf.push("こんに"), Vec::<String>::new());
    assert_eq!(
        buf.push("ちは。元気"),
        vec!["こんにちは。".to_string()]
    );
    assert_eq!(buf.push("ですか？"), Vec::<String>::new());
    assert_eq!(buf.finish(), vec!["元気ですか？".to_string()]);
}

#[test]
fn 連続区切り文字がトークン境界で分かれるケース() {
    let mut buf = SegmentBuffer::new();
    assert_eq!(buf.push("えっ！"), Vec::<String>::new());
    assert_eq!(buf.push("？"), Vec::<String>::new());
    assert_eq!(buf.push("そうなの。"), vec!["えっ！？".to_string()]);
    assert_eq!(buf.finish(), vec!["そうなの。".to_string()]);
}

#[test]
fn 複数文を含む単一トークン() {
    let mut buf = SegmentBuffer::new();
    assert_eq!(
        buf.push("はい。そうです。おそらく"),
        vec!["はい。".to_string(), "そうです。".to_string()]
    );
    assert_eq!(buf.finish(), vec!["おそらく".to_string()]);
}

#[test]
fn 区切り文字なしで終わるケース() {
    let mut buf = SegmentBuffer::new();
    assert_eq!(buf.push("これはテスト"), Vec::<String>::new());
    assert_eq!(buf.finish(), vec!["これはテスト".to_string()]);
}

#[test]
fn 改行区切り() {
    let mut buf = SegmentBuffer::new();
    assert_eq!(buf.push("一行目\n二行"), vec!["一行目".to_string()]);
    assert_eq!(buf.finish(), vec!["二行".to_string()]);
}

#[test]
fn pushを呼ばずfinish() {
    let buf = SegmentBuffer::new();
    assert_eq!(buf.finish(), Vec::<String>::new());
}

#[test]
fn 空文字列のみpushしてfinish() {
    let mut buf = SegmentBuffer::new();
    assert_eq!(buf.push(""), Vec::<String>::new());
    assert_eq!(buf.finish(), Vec::<String>::new());
}

#[test]
fn 空白のみのトークン() {
    let mut buf = SegmentBuffer::new();
    assert_eq!(buf.push("   "), Vec::<String>::new());
    assert_eq!(buf.finish(), Vec::<String>::new());
}

#[test]
fn 一文字ずつpush() {
    let mut buf = SegmentBuffer::new();
    assert_eq!(buf.push("こ"), Vec::<String>::new());
    assert_eq!(buf.push("ん"), Vec::<String>::new());
    assert_eq!(buf.push("に"), Vec::<String>::new());
    assert_eq!(buf.push("ち"), Vec::<String>::new());
    assert_eq!(buf.push("は"), Vec::<String>::new());
    assert_eq!(buf.push("。"), Vec::<String>::new());
    assert_eq!(buf.push("元"), vec!["こんにちは。".to_string()]);
    assert_eq!(buf.push("気"), Vec::<String>::new());
    assert_eq!(buf.push("？"), Vec::<String>::new());
    assert_eq!(buf.finish(), vec!["元気？".to_string()]);
}

#[test]
fn 最後が改行で終わる() {
    let mut buf = SegmentBuffer::new();
    assert_eq!(buf.push("終わり。\n"), vec!["終わり。".to_string()]);
    assert_eq!(buf.finish(), Vec::<String>::new());
}

#[test]
fn 複数回のpushで段階的に確定() {
    let mut buf = SegmentBuffer::new();
    assert_eq!(buf.push("最初"), Vec::<String>::new());
    assert_eq!(buf.push("。"), Vec::<String>::new());
    assert_eq!(buf.push("次"), vec!["最初。".to_string()]);
    assert_eq!(buf.push("。"), Vec::<String>::new());
    assert_eq!(buf.push("最後"), vec!["次。".to_string()]);
    assert_eq!(buf.finish(), vec!["最後".to_string()]);
}

#[test]
fn 複数の区切り文字が混在() {
    let mut buf = SegmentBuffer::new();
    assert_eq!(buf.push("質問？"), Vec::<String>::new());
    assert_eq!(
        buf.push("答え。驚き！"),
        vec!["質問？".to_string(), "答え。".to_string()]
    );
    assert_eq!(buf.push("本気？"), vec!["驚き！".to_string()]);
    assert_eq!(buf.finish(), vec!["本気？".to_string()]);
}

#[test]
fn 連続改行は1つのセグメント終了() {
    let mut buf = SegmentBuffer::new();
    assert_eq!(buf.push("一行\n\n二行"), vec!["一行".to_string()]);
    assert_eq!(buf.finish(), vec!["二行".to_string()]);
}

#[test]
fn 半角区切り文字() {
    let mut buf = SegmentBuffer::new();
    assert_eq!(buf.push("What!"), Vec::<String>::new());
    assert_eq!(buf.push("?"), Vec::<String>::new());
    assert_eq!(buf.push("Amazing."), vec!["What!?".to_string()]);
    assert_eq!(buf.finish(), vec!["Amazing.".to_string()]);
}

#[test]
fn 短いトークンの連続() {
    let mut buf = SegmentBuffer::new();
    assert_eq!(buf.push("a"), Vec::<String>::new());
    assert_eq!(buf.push("b"), Vec::<String>::new());
    assert_eq!(buf.push("。"), Vec::<String>::new());
    assert_eq!(buf.push("c"), vec!["ab。".to_string()]);
    assert_eq!(buf.push("d"), Vec::<String>::new());
    assert_eq!(buf.finish(), vec!["cd".to_string()]);
}

#[test]
fn 区切り文字で始まるトークン() {
    let mut buf = SegmentBuffer::new();
    assert_eq!(buf.push("あ"), Vec::<String>::new());
    assert_eq!(buf.push("。"), Vec::<String>::new());
    assert_eq!(buf.push("。い"), vec!["あ。。".to_string()]);
    assert_eq!(buf.push("。"), Vec::<String>::new());
    assert_eq!(buf.push("う"), vec!["い。".to_string()]);
    assert_eq!(buf.finish(), vec!["う".to_string()]);
}

#[test]
fn 中央の空白は保持される() {
    let mut buf = SegmentBuffer::new();
    assert_eq!(buf.push("単語  単語。他"), vec!["単語  単語。".to_string()]);
    assert_eq!(buf.finish(), vec!["他".to_string()]);
}

#[test]
fn 改行のみでセグメント分割() {
    let mut buf = SegmentBuffer::new();
    assert_eq!(buf.push("開始\n"), vec!["開始".to_string()]);
    assert_eq!(buf.push("\n終了"), Vec::<String>::new());
    assert_eq!(buf.finish(), vec!["終了".to_string()]);
}

#[test]
fn 連続の異なる区切り文字() {
    let mut buf = SegmentBuffer::new();
    assert_eq!(buf.push("驚き！！！"), Vec::<String>::new());
    assert_eq!(buf.push("？"), Vec::<String>::new());
    assert_eq!(buf.push("本当。"), vec!["驚き！！！？".to_string()]);
    assert_eq!(buf.finish(), vec!["本当。".to_string()]);
}

#[test]
fn 区切り文字だけのトークン() {
    let mut buf = SegmentBuffer::new();
    assert_eq!(buf.push("あ"), Vec::<String>::new());
    assert_eq!(buf.push("。"), Vec::<String>::new());
    assert_eq!(buf.push("！"), Vec::<String>::new());
    assert_eq!(buf.push("い"), vec!["あ。！".to_string()]);
    assert_eq!(buf.finish(), vec!["い".to_string()]);
}

#[test]
fn 複数回空文字列push() {
    let mut buf = SegmentBuffer::new();
    assert_eq!(buf.push(""), Vec::<String>::new());
    assert_eq!(buf.push(""), Vec::<String>::new());
    assert_eq!(buf.push("テスト"), Vec::<String>::new());
    assert_eq!(buf.push(""), Vec::<String>::new());
    assert_eq!(buf.finish(), vec!["テスト".to_string()]);
}

#[test]
fn finish後に新しいバッファを使用() {
    let mut buf1 = SegmentBuffer::new();
    assert_eq!(buf1.push("一目。"), Vec::<String>::new());
    assert_eq!(buf1.finish(), vec!["一目。".to_string()]);

    let mut buf2 = SegmentBuffer::new();
    assert_eq!(buf2.push("独立。"), Vec::<String>::new());
    assert_eq!(buf2.finish(), vec!["独立。".to_string()]);
}
