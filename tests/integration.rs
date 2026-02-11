//! 통합 테스트: parse → 상태 조작 → output 파이프라인

use diffy::model::ReviewStatus;
use diffy::parse::parse_diff;
use diffy::output::{write_diff, write_json};
use diffy::hook::write_feedback;
use indoc::indoc;
use serde_json::Value;

const SAMPLE_DIFF: &str = indoc! {"
    diff --git a/src/main.rs b/src/main.rs
    --- a/src/main.rs
    +++ b/src/main.rs
    @@ -1,3 +1,4 @@
     fn main() {
    +    println!(\"hello\");
         // existing
     }
    @@ -10,3 +11,2 @@
     fn helper() {
    -    old_code();
         new_code();
     }
"};

#[test]
fn test_parse_accept_all_roundtrip() {
    let mut diff = parse_diff(SAMPLE_DIFF).unwrap();
    assert_eq!(diff.files.len(), 1);
    assert_eq!(diff.files[0].hunks.len(), 2);

    // Accept all hunks
    for hunk in &mut diff.files[0].hunks {
        hunk.status = ReviewStatus::Accepted;
    }

    let mut output = Vec::new();
    let has_output = write_diff(&diff, &mut output).unwrap();
    assert!(has_output);

    let text = String::from_utf8(output).unwrap();
    // Output should contain both hunks
    assert!(text.contains("@@ -1,3 +1,4 @@"));
    assert!(text.contains("@@ -10,3 +11,2 @@"));
    assert!(text.contains("+    println!(\"hello\");"));
    assert!(text.contains("-    old_code();"));
}

#[test]
fn test_parse_reject_all_empty() {
    let mut diff = parse_diff(SAMPLE_DIFF).unwrap();

    // Reject all hunks
    for hunk in &mut diff.files[0].hunks {
        hunk.status = ReviewStatus::Rejected;
    }

    let mut output = Vec::new();
    let has_output = write_diff(&diff, &mut output).unwrap();
    assert!(!has_output);
    assert!(output.is_empty());
}

#[test]
fn test_parse_partial_accept() {
    let mut diff = parse_diff(SAMPLE_DIFF).unwrap();

    // Accept first hunk only
    diff.files[0].hunks[0].status = ReviewStatus::Accepted;
    diff.files[0].hunks[1].status = ReviewStatus::Rejected;

    let mut output = Vec::new();
    let has_output = write_diff(&diff, &mut output).unwrap();
    assert!(has_output);

    let text = String::from_utf8(output).unwrap();
    assert!(text.contains("@@ -1,3 +1,4 @@"));
    assert!(text.contains("+    println!(\"hello\");"));
    // Second hunk should NOT be in output
    assert!(!text.contains("@@ -10,3 +11,2 @@"));
    assert!(!text.contains("-    old_code();"));
}

#[test]
fn test_json_roundtrip() {
    let mut diff = parse_diff(SAMPLE_DIFF).unwrap();

    // Partial review: accept first, reject second
    diff.files[0].hunks[0].status = ReviewStatus::Accepted;
    diff.files[0].hunks[1].status = ReviewStatus::Rejected;

    let mut buf = Vec::new();
    write_json(&diff, &mut buf).unwrap();
    let json: Value = serde_json::from_slice(&buf).unwrap();

    assert_eq!(json["summary"]["total_hunks"], 2);
    assert_eq!(json["summary"]["accepted"], 1);
    assert_eq!(json["summary"]["rejected"], 1);
    assert_eq!(json["summary"]["pending"], 0);
    assert_eq!(json["files"][0]["path"], "src/main.rs");
    assert_eq!(json["files"][0]["hunks"][0]["status"], "accepted");
    assert_eq!(json["files"][0]["hunks"][1]["status"], "rejected");
}

#[test]
fn test_hook_feedback_integration() {
    let mut diff = parse_diff(SAMPLE_DIFF).unwrap();

    // Reject second hunk
    diff.files[0].hunks[0].status = ReviewStatus::Accepted;
    diff.files[0].hunks[1].status = ReviewStatus::Rejected;

    let mut output = Vec::new();
    let all_accepted = write_feedback(&diff, &mut output).unwrap();
    assert!(!all_accepted);

    let text = String::from_utf8(output).unwrap();
    assert!(text.contains("rejected 1 of 2 hunks"));
    assert!(text.contains("src/main.rs"));
    assert!(text.contains("please fix"));
}

#[test]
fn test_empty_diff_pipeline() {
    let diff = parse_diff("").unwrap();
    assert!(diff.files.is_empty());

    let mut output = Vec::new();
    let has_output = write_diff(&diff, &mut output).unwrap();
    assert!(!has_output);
    assert!(output.is_empty());
}

#[test]
fn test_json_comment_in_output() {
    let input = indoc! {"
        --- a/file.rs
        +++ b/file.rs
        @@ -1,2 +1,2 @@
         context
        -old
        +new
    "};
    let mut diff = parse_diff(input).unwrap();
    diff.files[0].hunks[0].status = ReviewStatus::Rejected;
    diff.files[0].hunks[0].comment = Some("needs improvement".to_string());

    let mut buf = Vec::new();
    write_json(&diff, &mut buf).unwrap();
    let json: Value = serde_json::from_slice(&buf).unwrap();

    assert_eq!(json["files"][0]["hunks"][0]["comment"], "needs improvement");
}
