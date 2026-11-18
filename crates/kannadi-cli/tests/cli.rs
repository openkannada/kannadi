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

const ASSETS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets");
const REGENERATE: &str = "run the tests with KANNADI_BLESS=1 to rewrite crates/kannadi-cli/assets";

/// Compares a generated asset with the committed copy, or rewrites the copy
/// when KANNADI_BLESS is set.
fn check_asset(path: &str, fresh: &[u8]) {
    let committed = format!("{ASSETS}/{path}");
    if std::env::var_os("KANNADI_BLESS").is_some() {
        std::fs::write(&committed, fresh).unwrap();
        return;
    }
    let old = std::fs::read(&committed).unwrap_or_default();
    assert!(old == fresh, "{path} is out of date; {REGENERATE}");
}

#[test]
fn committed_man_pages_match_the_cli() {
    let tmp = tempfile::tempdir().unwrap();
    let run = kannadi(&["manpages", "man"], tmp.path());
    assert_eq!(run.status.code(), Some(0), "{run:?}");
    let mut made: Vec<String> = std::fs::read_dir(tmp.path().join("man"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    made.sort();
    assert_eq!(
        made,
        [
            "kannadi-completions.1",
            "kannadi-manpages.1",
            "kannadi-to-nudi.1",
            "kannadi-to-unicode.1",
            "kannadi.1"
        ]
    );
    for name in made {
        let fresh = std::fs::read(tmp.path().join("man").join(&name)).unwrap();
        check_asset(&format!("man/{name}"), &fresh);
    }
}

#[test]
fn committed_completions_match_the_cli() {
    let tmp = tempfile::tempdir().unwrap();
    for (shell, file) in [
        ("bash", "kannadi.bash"),
        ("zsh", "_kannadi"),
        ("fish", "kannadi.fish"),
        ("powershell", "_kannadi.ps1"),
        ("elvish", "kannadi.elv"),
    ] {
        let run = kannadi(&["completions", shell], tmp.path());
        assert_eq!(run.status.code(), Some(0), "{run:?}");
        check_asset(&format!("completions/{file}"), &run.stdout);
        let script = String::from_utf8_lossy(&run.stdout);
        assert!(script.contains("to-unicode"), "{file}");
        if ["bash", "zsh", "fish"].contains(&shell) {
            assert!(script.contains("shree-lipi"), "{file} completes layouts");
        }
    }
}

#[test]
fn help_names_the_exit_statuses_and_version_prints_the_package_version() {
    let tmp = tempfile::tempdir().unwrap();
    for args in [
        &["--help"][..],
        &["to-unicode", "-h"],
        &["to-nudi", "--help"],
        &["to-nudi", "-h"],
        &["to-unicode", "--help"],
    ] {
        let run = kannadi(args, tmp.path());
        assert_eq!(run.status.code(), Some(0));
        let help = String::from_utf8_lossy(&run.stdout);
        assert!(
            help.contains("Exit status:\n  0  Success."),
            "{args:?}: {help}"
        );
    }
    let long = String::from_utf8_lossy(&kannadi(&["to-unicode", "--help"], tmp.path()).stdout)
        .into_owned();
    assert!(long.contains("Examples:\n  kannadi to-unicode"), "{long}");
    assert!(long.contains("[default: \"Noto Sans Kannada\"]"), "{long}");
    let nudi =
        String::from_utf8_lossy(&kannadi(&["to-nudi", "--help"], tmp.path()).stdout).into_owned();
    assert!(nudi.contains("[default: \"Nudi 01 e\"]"), "{nudi}");
    assert!(!nudi.contains("--layout"), "{nudi}");
    for (args, name) in [
        (&["--version"][..], "kannadi"),
        (&["-V"], "kannadi"),
        (&["to-unicode", "--version"], "kannadi-to-unicode"),
    ] {
        let run = kannadi(args, tmp.path());
        assert_eq!(
            String::from_utf8_lossy(&run.stdout),
            format!("{name} {}\n", env!("CARGO_PKG_VERSION"))
        );
    }
}

#[test]
fn manpages_into_a_file_path_is_an_error() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("taken"), "x").unwrap();
    let run = kannadi(&["manpages", "taken"], tmp.path());
    assert_eq!(run.status.code(), Some(2), "{run:?}");
    assert!(String::from_utf8_lossy(&run.stderr).starts_with("error: creating taken"));
}

#[test]
fn no_command_shows_help_and_exits_with_2() {
    let tmp = tempfile::tempdir().unwrap();
    let run = kannadi(&[], tmp.path());
    assert_eq!(run.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&run.stderr).contains("Usage: kannadi <COMMAND>"));
}
