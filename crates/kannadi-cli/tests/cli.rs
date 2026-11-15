use std::path::Path;
use std::process::{Command, Output};

fn kannadi(args: &[&str], dir: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_kannadi"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run kannadi")
}

#[test]
fn converts_a_folder_and_reports_problems() {
    let tmp = tempfile::tempdir().unwrap();
    let docs = tmp.path().join("docs");
    std::fs::create_dir(&docs).unwrap();
    std::fs::write(
        docs.join("clean.txt"),
        [0x50, 0xC0, 0xA3, 0xC0, 0xDF, 0x71, 0xC0],
    )
    .unwrap();
    std::fs::write(docs.join("broken.txt"), [0x50, 0xC0, 0xF1]).unwrap();

    let run = kannadi(&["to-unicode", "docs", "--out", "out"], tmp.path());
    assert_eq!(run.status.code(), Some(0), "{run:?}");
    let out = tmp.path().join("out/docs");
    assert_eq!(
        std::fs::read_to_string(out.join("clean.txt")).unwrap(),
        "ಕನ್ನಡ"
    );
    assert!(!out.join("clean.txt.report.tsv").exists());
    let report = std::fs::read_to_string(out.join("broken.txt.report.tsv")).unwrap();
    assert!(
        report.contains("line 1\tunknown glyph\tbyte F1"),
        "{report}"
    );
    assert!(String::from_utf8_lossy(&run.stderr).contains("1 with issues"));

    let strict = kannadi(
        &["to-unicode", "docs", "--out", "out", "--strict"],
        tmp.path(),
    );
    assert_eq!(strict.status.code(), Some(1));
}

#[test]
fn strict_passes_on_clean_input_both_ways() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("u.txt"), "ಕನ್ನಡ ನಾಡು").unwrap();
    let run = kannadi(
        &["to-nudi", "u.txt", "--out", "nudi", "--strict"],
        tmp.path(),
    );
    assert_eq!(run.status.code(), Some(0), "{run:?}");
    let back = kannadi(
        &["to-unicode", "nudi/u.txt", "--out", "back", "--strict"],
        tmp.path(),
    );
    assert_eq!(back.status.code(), Some(0), "{back:?}");
    let text = std::fs::read_to_string(tmp.path().join("back/u.txt")).unwrap();
    assert_eq!(text, "ಕನ್ನಡ ನಾಡು");
}

#[test]
fn errors_exit_with_2() {
    let tmp = tempfile::tempdir().unwrap();
    let run = kannadi(&["to-unicode", "missing.txt", "--out", "out"], tmp.path());
    assert_eq!(run.status.code(), Some(2));
    std::fs::write(tmp.path().join("bad.txt"), [0xC0]).unwrap();
    let run = kannadi(&["to-nudi", "bad.txt", "--out", "out"], tmp.path());
    assert_eq!(run.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&run.stderr).contains("not UTF-8"));
}

#[test]
fn a_clean_rerun_removes_the_old_report() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("a.txt");
    std::fs::write(&file, [0x50, 0xC0, 0xF1]).unwrap();
    kannadi(&["to-unicode", "a.txt", "--out", "out"], tmp.path());
    let report = tmp.path().join("out/a.txt.report.tsv");
    assert!(report.exists());
    std::fs::write(&file, [0x50, 0xC0]).unwrap();
    kannadi(&["to-unicode", "a.txt", "--out", "out"], tmp.path());
    assert!(!report.exists());
}
