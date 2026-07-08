use std::fs;
use std::path::PathBuf;
use tts_emotion::CharacterStore;

fn create_test_dir(test_name: &str) -> PathBuf {
    let base = std::env::temp_dir();
    let pid = std::process::id();
    let dir = base.join(format!("tts_emotion_chara_test_{}_{}", test_name, pid));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn cleanup_test_dir(dir: &PathBuf) {
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn load_all_fields() {
    let test_dir = create_test_dir("load_all_fields");

    let toml_content = r#"
name = "なると"
system_prompt = """
あなたは「なると」という名前のアシスタントです。
明るく元気な口調で話します。
"""
voice = "coral"

[emotion]
default = "joy"

[[emotion.emotions]]
name = "joy"
keywords = ["嬉しい", "やった"]
motion_id = "joy"
"#;

    fs::write(test_dir.join("naruto.toml"), toml_content).unwrap();

    let store = CharacterStore::load_dir(&test_dir).unwrap();
    let chara = store.get("naruto").unwrap();

    assert_eq!(chara.name, "なると");
    assert!(chara.system_prompt.contains("なると"));
    assert_eq!(chara.voice, Some("coral".to_string()));
    assert!(chara.emotion.is_some());

    cleanup_test_dir(&test_dir);
}

#[test]
fn load_minimal_fields() {
    let test_dir = create_test_dir("load_minimal_fields");

    let toml_content = r#"
name = "minimal"
system_prompt = "You are a minimal character."
"#;

    fs::write(test_dir.join("minimal.toml"), toml_content).unwrap();

    let store = CharacterStore::load_dir(&test_dir).unwrap();
    let chara = store.get("minimal").unwrap();

    assert_eq!(chara.name, "minimal");
    assert_eq!(chara.system_prompt, "You are a minimal character.");
    assert!(chara.voice.is_none());
    assert!(chara.emotion.is_none());

    cleanup_test_dir(&test_dir);
}

#[test]
fn load_multiple_files() {
    let test_dir = create_test_dir("load_multiple_files");

    let toml1 = r#"
name = "character1"
system_prompt = "First character"
"#;
    let toml2 = r#"
name = "character2"
system_prompt = "Second character"
"#;

    fs::write(test_dir.join("first.toml"), toml1).unwrap();
    fs::write(test_dir.join("second.toml"), toml2).unwrap();

    let store = CharacterStore::load_dir(&test_dir).unwrap();

    assert_eq!(store.len(), 2);
    assert_eq!(store.ids(), vec!["first".to_string(), "second".to_string()]);
    assert!(store.get("first").is_some());
    assert!(store.get("second").is_some());

    cleanup_test_dir(&test_dir);
}

#[test]
fn get_nonexistent_id() {
    let test_dir = create_test_dir("get_nonexistent_id");

    let toml_content = r#"
name = "test"
system_prompt = "Test"
"#;

    fs::write(test_dir.join("test.toml"), toml_content).unwrap();

    let store = CharacterStore::load_dir(&test_dir).unwrap();

    assert!(store.get("nonexistent").is_none());

    cleanup_test_dir(&test_dir);
}

#[test]
fn load_nonexistent_directory() {
    let nonexistent = std::env::temp_dir().join("nonexistent_tts_emotion_dir_12345678");
    let result = CharacterStore::load_dir(&nonexistent);

    assert!(result.is_err());
}

#[test]
fn load_empty_directory() {
    let test_dir = create_test_dir("load_empty_directory");

    let store = CharacterStore::load_dir(&test_dir).unwrap();

    assert!(store.is_empty());
    assert_eq!(store.len(), 0);
    assert_eq!(store.ids(), vec![] as Vec<String>);

    cleanup_test_dir(&test_dir);
}

#[test]
fn ignore_non_toml_files() {
    let test_dir = create_test_dir("ignore_non_toml_files");

    let toml_content = r#"
name = "valid"
system_prompt = "Valid character"
"#;

    fs::write(test_dir.join("valid.toml"), toml_content).unwrap();
    fs::write(test_dir.join("readme.txt"), "This should be ignored").unwrap();
    fs::write(test_dir.join("data.json"), "{}").unwrap();

    let store = CharacterStore::load_dir(&test_dir).unwrap();

    assert_eq!(store.len(), 1);
    assert_eq!(store.ids(), vec!["valid".to_string()]);

    cleanup_test_dir(&test_dir);
}

#[test]
fn skip_unparseable_toml() {
    let test_dir = create_test_dir("skip_unparseable_toml");

    let valid_toml = r#"
name = "valid"
system_prompt = "Valid character"
"#;

    let broken_toml = r#"
name = "broken
system_prompt = [invalid
"#;

    fs::write(test_dir.join("valid.toml"), valid_toml).unwrap();
    fs::write(test_dir.join("broken.toml"), broken_toml).unwrap();

    let store = CharacterStore::load_dir(&test_dir).unwrap();

    assert_eq!(store.len(), 1);
    assert!(store.get("valid").is_some());
    assert!(store.get("broken").is_none());

    cleanup_test_dir(&test_dir);
}

#[test]
fn skip_missing_required_field() {
    let test_dir = create_test_dir("skip_missing_required_field");

    let valid_toml = r#"
name = "valid"
system_prompt = "Valid character"
"#;

    let missing_system_prompt = r#"
name = "incomplete"
"#;

    fs::write(test_dir.join("valid.toml"), valid_toml).unwrap();
    fs::write(test_dir.join("incomplete.toml"), missing_system_prompt).unwrap();

    let store = CharacterStore::load_dir(&test_dir).unwrap();

    assert_eq!(store.len(), 1);
    assert!(store.get("valid").is_some());
    assert!(store.get("incomplete").is_none());

    cleanup_test_dir(&test_dir);
}

#[test]
fn character_emotion_rules_work() {
    let test_dir = create_test_dir("character_emotion_rules_work");

    let toml_content = r#"
name = "emotional"
system_prompt = "Character with emotion rules"

[emotion]
default = "neutral"

[[emotion.emotions]]
name = "joy"
keywords = ["嬉しい", "やった"]
motion_id = "joy"
"#;

    fs::write(test_dir.join("emotional.toml"), toml_content).unwrap();

    let store = CharacterStore::load_dir(&test_dir).unwrap();
    let chara = store.get("emotional").unwrap();

    let emotion_result = chara.emotion.as_ref().unwrap().extract("やった");
    assert_eq!(emotion_result.name, "joy");
    assert_eq!(emotion_result.motion_id, Some("joy".to_string()));

    cleanup_test_dir(&test_dir);
}

#[test]
fn japanese_filename() {
    let test_dir = create_test_dir("japanese_filename");

    let toml_content = r#"
name = "なると"
system_prompt = "Japanese character"
"#;

    fs::write(test_dir.join("なると.toml"), toml_content).unwrap();

    let store = CharacterStore::load_dir(&test_dir).unwrap();

    assert_eq!(store.len(), 1);
    assert!(store.get("なると").is_some());

    let chara = store.get("なると").unwrap();
    assert_eq!(chara.name, "なると");

    cleanup_test_dir(&test_dir);
}

#[test]
fn ids_sorted() {
    let test_dir = create_test_dir("ids_sorted");

    let toml = r#"
name = "test"
system_prompt = "Test"
"#;

    fs::write(test_dir.join("zebra.toml"), toml).unwrap();
    fs::write(test_dir.join("apple.toml"), toml).unwrap();
    fs::write(test_dir.join("monkey.toml"), toml).unwrap();

    let store = CharacterStore::load_dir(&test_dir).unwrap();
    let ids = store.ids();

    assert_eq!(ids, vec!["apple".to_string(), "monkey".to_string(), "zebra".to_string()]);

    cleanup_test_dir(&test_dir);
}

#[test]
fn len_matches_loaded_characters() {
    let test_dir = create_test_dir("len_matches_loaded_characters");

    let toml = r#"
name = "test"
system_prompt = "Test"
"#;

    for i in 0..5 {
        fs::write(test_dir.join(format!("chara{}.toml", i)), toml).unwrap();
    }

    let store = CharacterStore::load_dir(&test_dir).unwrap();

    assert_eq!(store.len(), 5);
    assert!(!store.is_empty());

    cleanup_test_dir(&test_dir);
}

#[test]
fn is_empty_true() {
    let test_dir = create_test_dir("is_empty_true");

    let store = CharacterStore::load_dir(&test_dir).unwrap();

    assert!(store.is_empty());
    assert_eq!(store.len(), 0);

    cleanup_test_dir(&test_dir);
}
