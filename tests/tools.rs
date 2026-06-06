use agent_ease::shadowbox::Shadowbox;
use agent_ease::tools::{FileReadTool, SearchTool, Tool};
use serde_json::json;

#[tokio::test]
async fn shadowbox_blocks_outside_workspace() {
    let temp = tempfile::tempdir().expect("tempdir");
    let shadowbox = Shadowbox::new(temp.path());
    let outside = temp.path().parent().expect("parent").join("outside.txt");
    let err = shadowbox
        .ensure_inside_workspace(&outside)
        .expect_err("outside rejected");
    assert!(err.to_string().contains("outside workspace"));
}

#[tokio::test]
async fn file_read_reads_workspace_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("note.txt");
    tokio::fs::write(&path, "hello file").await.expect("write");
    let shadowbox = Shadowbox::new(temp.path());
    let output = FileReadTool
        .call(&shadowbox, json!({"path":"note.txt"}))
        .await
        .expect("read");
    assert_eq!(output["content"], "hello file");
}

#[tokio::test]
async fn search_finds_text_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    tokio::fs::write(temp.path().join("a.txt"), "alpha beta")
        .await
        .expect("write");
    let shadowbox = Shadowbox::new(temp.path());
    let output = SearchTool
        .call(&shadowbox, json!({"query":"beta"}))
        .await
        .expect("search");
    assert!(output["matches"].as_array().expect("array")[0]["path"]
        .as_str()
        .expect("path")
        .ends_with("a.txt"));
}
