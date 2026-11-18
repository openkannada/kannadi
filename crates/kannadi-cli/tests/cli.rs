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
        report.contains("line 1\tunknown glyph\tglyph F1"),
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
fn likely_typos_are_reported_without_failing_strict() {
    let tmp = tempfile::tempdir().unwrap();
    // ನೈರೃತ್ಯ in Nudi: well formed, with ರೃ where Kannada writes ಋ.
    std::fs::write(tmp.path().join("t.txt"), "£ÉÊgÀÈvÀå").unwrap();
    let run = kannadi(
        &["to-unicode", "t.txt", "--out", "out", "--strict"],
        tmp.path(),
    );
    assert_eq!(run.status.code(), Some(0), "{run:?}");
    let report = std::fs::read_to_string(tmp.path().join("out/t.txt.report.tsv")).unwrap();
    assert!(
        report.contains("line 1\tlikely typo\tಋ for ರೃ (ra-vocalic-r)\tನೈರೃತ್ಯ"),
        "{report}"
    );
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(stderr.contains("t.txt: 1 likely typo"), "{stderr}");
    assert!(stderr.contains("0 with issues"), "{stderr}");
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

#[cfg(unix)]
#[test]
fn an_unreadable_file_fails_the_run_but_not_the_others() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().unwrap();
    let docs = tmp.path().join("docs");
    std::fs::create_dir(&docs).unwrap();
    std::fs::write(docs.join("a.txt"), [0x50, 0xC0]).unwrap();
    std::fs::write(docs.join("b.txt"), [0x50, 0xC0]).unwrap();
    std::fs::set_permissions(docs.join("b.txt"), std::fs::Permissions::from_mode(0o000)).unwrap();
    let run = kannadi(
        &["to-unicode", "docs", "--out", "out", "--strict"],
        tmp.path(),
    );
    std::fs::set_permissions(docs.join("b.txt"), std::fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(run.status.code(), Some(2), "{run:?}");
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains("converted 1 file, 0 with issues, 1 failed"),
        "{stderr}"
    );
    assert!(tmp.path().join("out/docs/a.txt").exists());
}

#[test]
fn a_long_line_full_of_problems_gives_short_rows() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("u.txt"), "ಕನ್ನಡ ".repeat(5_000)).unwrap();
    let run = kannadi(&["to-unicode", "u.txt", "--out", "out"], tmp.path());
    assert_eq!(run.status.code(), Some(0), "{run:?}");
    let report = std::fs::read_to_string(tmp.path().join("out/u.txt.report.tsv")).unwrap();
    assert!(report.lines().count() > 5_000);
    assert!(
        report.lines().all(|l| l.chars().count() < 120),
        "a row is too long"
    );
}

#[test]
fn html_round_trips_through_nudi() {
    let tmp = tempfile::tempdir().unwrap();
    let page = "<html><head><title>t</title></head><body><p>ಕನ್ನಡ ನಾಡು, 2026</p></body></html>";
    std::fs::write(tmp.path().join("p.html"), page).unwrap();
    let run = kannadi(
        &["to-nudi", "p.html", "--out", "nudi", "--strict"],
        tmp.path(),
    );
    assert_eq!(run.status.code(), Some(0), "{run:?}");
    let nudi = std::fs::read_to_string(tmp.path().join("nudi/p.html")).unwrap();
    assert!(nudi.contains("font-family:'Nudi 01 e'"), "{nudi}");
    let back = kannadi(
        &[
            "to-unicode",
            "nudi/p.html",
            "--out",
            "back",
            "--font",
            "Tunga",
            "--strict",
        ],
        tmp.path(),
    );
    assert_eq!(back.status.code(), Some(0), "{back:?}");
    let html = std::fs::read_to_string(tmp.path().join("back/p.html")).unwrap();
    assert!(
        html.contains("<span style=\"font-family:'Tunga'\">ಕನ್ನಡ ನಾಡು</span>, 2026"),
        "{html}"
    );
}

#[test]
fn unicode_pdfs_from_chrome_read_whole() {
    let fixtures = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let expected = "ನಮಸ್ಕಾರ, ಕನ್ನಡ ನಾಡು ನುಡಿ.\n\
        ಶ್ರೀ ಕೃಷ್ಣ ಅರ್ಜುನ ಸ್ತ್ರೀ ಕ್ಷೇತ್ರ ಜ್ಞಾನ ಋಷಿ ೧೨೩\n\
        English line 2026 & more\n\
        ಕಾರ್ಯಕ್ರಮಗಳನ್ನು ಹಮ್ಮಿಕೊಳ್ಳಲಾಗುತ್ತಿದೆ.\n";
    let tmp = tempfile::tempdir().unwrap();
    for name in ["unicode-sans.pdf", "unicode-serif.pdf"] {
        let run = kannadi(
            &[
                "to-unicode",
                &format!("{fixtures}/{name}"),
                "--out",
                "out",
                "--strict",
            ],
            tmp.path(),
        );
        assert_eq!(run.status.code(), Some(0), "{name}: {run:?}");
        let text = std::fs::read_to_string(tmp.path().join(format!("out/{name}.txt"))).unwrap();
        assert_eq!(text, expected, "{name}");
        assert!(!tmp
            .path()
            .join(format!("out/{name}.txt.report.tsv"))
            .exists());
    }
}
