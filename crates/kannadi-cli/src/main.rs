//! `kannadi`: converts folders of documents between legacy Kannada fonts and Unicode.

mod convert;
mod docx;
mod edits;
mod files;
mod fonts;
mod html;
mod normalize;
mod pdf;
mod report;
mod txt;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use kannadi::legacy::{Layout, Options};

use convert::{Direction, Settings};
use files::{Format, Job};
use report::Report;

const EXIT_STATUS: &str = "\
Exit status:
  0  Success. Reports may still list problems and likely typos.
  1  --strict was given and a file has a problem.
  2  Bad arguments, or a file that could not be read, converted or written.
     2 wins over 1 when both apply.";

const TO_UNICODE_EXAMPLES: &str = "\
Examples:
  kannadi to-unicode old-orders/ --out converted/
  kannadi to-unicode notes.txt --layout akruti --out converted/
  kannadi to-unicode page.html --legacy-font \"My Font=akruti\" --out converted/";

const NORMALIZE_EXAMPLES: &str = "\
Examples:
  kannadi normalize page.txt > page.normalized.txt
  kannadi normalize --form legacy --stats converted.txt > clean.txt
  kannadi normalize --check corpus/*.txt

Exit status:
  0  Success.
  1  --check was given and an input is not in the form.
  2  A file could not be read or is not UTF-8.";

const TO_NUDI_EXAMPLES: &str = "\
Examples:
  kannadi to-nudi letter.txt --out for-printing/ --strict
  kannadi to-nudi site/ --font \"Nudi 05 e\" --out nudi-site/";

/// Convert Kannada documents between legacy fonts and Unicode
#[derive(Parser)]
#[command(
    name = "kannadi",
    bin_name = "kannadi",
    version,
    propagate_version = true,
    long_about = "Convert Kannada documents between legacy fonts and Unicode.\n\n\
        to-unicode reads text typed in Nudi, Baraha ANSI, Nudi B-Akshara, Kan-Eng, Akruti and \
        Shree-Lipi fonts in TXT, HTML, DOCX and PDF files. to-nudi writes Unicode Kannada as \
        Nudi glyph codes.\n\n\
        Each converted file gets a report next to it, FILE.report.tsv, when anything could not \
        be converted cleanly or the result has likely typos.",
    after_help = EXIT_STATUS
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Convert documents in legacy Kannada fonts to Unicode
    ///
    /// Reads TXT, HTML, DOCX and PDF files. Text in HTML, DOCX and PDF files is
    /// converted only where it is set in a legacy font. The text of a PDF goes to
    /// NAME.pdf.txt.
    #[command(after_help = EXIT_STATUS, after_long_help = format!("{TO_UNICODE_EXAMPLES}\n\n{EXIT_STATUS}"))]
    ToUnicode {
        #[command(flatten)]
        files: Files,
        /// Layout of text files, and of fonts added with --legacy-font that name
        /// none. nudi covers the Nudi and Baraha ANSI fonts
        #[arg(long, value_name = "LAYOUT", default_value = "nudi", ignore_case = true, value_parser = layout_parser())]
        layout: Layout,
        /// Write digits as Kannada digits, for text set in the k variants of the
        /// Nudi fonts
        #[arg(long)]
        kannada_digits: bool,
        /// Keep a zero typed between letters as a digit; by default it becomes ಂ
        #[arg(long)]
        keep_zeros: bool,
        /// Font to set converted text in, in HTML and DOCX files
        #[arg(long, value_name = "NAME", default_value = convert::UNICODE_FONT)]
        font: String,
        /// Treat this font as a legacy font too, in the layout after `=`
        /// ("My Font=akruti") or else the --layout one. Repeat for more fonts
        #[arg(long, value_name = "NAME[=LAYOUT]", value_parser = parse_legacy_font)]
        legacy_font: Vec<(String, Option<Layout>)>,
    },
    /// Convert Unicode Kannada documents to Nudi glyph codes
    ///
    /// Writes TXT, HTML and DOCX files. Text files are written in Windows-1252.
    /// A PDF cannot be written back, so each PDF counts as an error.
    #[command(after_help = EXIT_STATUS, after_long_help = format!("{TO_NUDI_EXAMPLES}\n\n{EXIT_STATUS}"))]
    ToNudi {
        #[command(flatten)]
        files: Files,
        /// Font to set converted text in, in HTML and DOCX files
        #[arg(long, value_name = "NAME", default_value = convert::LEGACY_FONT)]
        font: String,
        /// Treat this font as a legacy font too, so text already set in it is
        /// left as it is. Repeat for more fonts
        #[arg(long, value_name = "NAME")]
        legacy_font: Vec<String>,
    },
    /// Normalize Unicode Kannada text, following the Kannada text normalization profile
    ///
    /// Reads UTF-8 text files, or standard input, and writes the normalized text to
    /// standard output. canonical gives 1 spelling to text that renders the same;
    /// legacy also repairs mistakes of legacy font converters and OCR, such as a zero
    /// typed for anusvara; search also folds Old Kannada letters, joiners and the nukta,
    /// for search keys. NORMALIZATION.md in the kannadi repository lists every rule.
    #[command(after_long_help = NORMALIZE_EXAMPLES)]
    Normalize {
        /// UTF-8 text files; standard input when none is given or for -
        inputs: Vec<PathBuf>,
        /// How far to go
        #[arg(long, default_value = "canonical", value_parser = form_parser())]
        form: kannadi::normalize::Form,
        /// Print how many times each rule changed the text to standard error, as TSV
        #[arg(long)]
        stats: bool,
        /// Write nothing; exit with status 1 if any input is not already in the form
        #[arg(long)]
        check: bool,
    },
    /// Print a shell completion script to standard output
    #[command(after_help = "Example:\n  kannadi completions zsh > ~/.zfunc/_kannadi")]
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Write the man pages to a folder
    #[command(after_help = "Example:\n  kannadi manpages /usr/local/share/man/man1")]
    Manpages {
        /// Folder to write kannadi.1 and a page per command to
        dir: PathBuf,
    },
}

#[derive(Args)]
struct Files {
    /// Files and folders to convert. Folders are searched for .txt, .htm, .html,
    /// .docx and .pdf files
    #[arg(required = true)]
    inputs: Vec<PathBuf>,
    /// Folder to write the converted files and reports to
    #[arg(short, long, value_name = "DIR")]
    out: PathBuf,
    /// Exit with status 1 if any file has unknown glyphs, malformed syllables or
    /// unencodable characters; likely typos do not count
    #[arg(long)]
    strict: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let (files, settings) = match cli.command {
        Command::ToUnicode {
            files,
            layout,
            kannada_digits,
            keep_zeros,
            font,
            legacy_font,
        } => {
            let options = Options {
                kannada_digits,
                zero_as_anusvara: !keep_zeros,
            };
            let direction = Direction::ToUnicode(options);
            (files, settings(direction, layout, font, &legacy_font))
        }
        Command::ToNudi {
            files,
            font,
            legacy_font,
        } => {
            let legacy_font: Vec<_> = legacy_font.into_iter().map(|name| (name, None)).collect();
            (
                files,
                settings(Direction::ToNudi, Layout::Nudi, font, &legacy_font),
            )
        }
        Command::Normalize {
            inputs,
            form,
            stats,
            check,
        } => return run_normalize(&inputs, form, stats, check),
        Command::Completions { shell } => {
            clap_complete::generate(
                shell,
                &mut Cli::command(),
                "kannadi",
                &mut std::io::stdout(),
            );
            return ExitCode::SUCCESS;
        }
        Command::Manpages { dir } => {
            return match write_manpages(&dir) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("error: {e:#}");
                    ExitCode::from(2)
                }
            };
        }
    };
    match run(&files, &settings) {
        Ok(Outcome { failed: true, .. }) => ExitCode::from(2),
        Ok(Outcome { issues: true, .. }) if files.strict => ExitCode::from(1),
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn run_normalize(
    inputs: &[PathBuf],
    form: kannadi::normalize::Form,
    stats: bool,
    check: bool,
) -> ExitCode {
    let outcome = match normalize::run(inputs, form, check, &mut std::io::stdout().lock()) {
        Ok(outcome) => outcome,
        Err(e) if normalize::is_broken_pipe(&e) => return ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            return ExitCode::from(2);
        }
    };
    if stats {
        eprint!("{}", normalize::stats_tsv(&outcome.counts, form));
    }
    if check && !outcome.changed.is_empty() {
        for name in &outcome.changed {
            eprintln!("{name}: not in the {} form", form.name());
        }
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

/// Form names for --form, listed in help, man pages and completions.
fn form_parser() -> impl clap::builder::TypedValueParser<Value = kannadi::normalize::Form> {
    use clap::builder::{PossibleValuesParser, TypedValueParser};
    use kannadi::normalize::Form;
    PossibleValuesParser::new(Form::ALL.map(Form::name))
        .map(|name| Form::from_name(&name).expect("a listed form"))
}

/// The settings the command line asks for; `layout` is the --layout value.
fn settings(
    direction: Direction,
    layout: Layout,
    font: String,
    legacy_font: &[(String, Option<Layout>)],
) -> Settings {
    let mut settings = Settings::new(direction);
    settings.text_layout = layout;
    match direction {
        Direction::ToUnicode(_) => settings.fonts.unicode = font,
        Direction::ToNudi => settings.fonts.legacy = font,
    }
    settings.fonts.extra_legacy = legacy_font
        .iter()
        .map(|(name, named)| (name.clone(), named.unwrap_or(layout)))
        .collect();
    settings
}

/// Writes kannadi.1 and a page for each command, such as kannadi-to-unicode.1.
fn write_manpages(dir: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    clap_mangen::generate_to(Cli::command(), dir)
        .with_context(|| format!("writing man pages to {}", dir.display()))
}

/// Layout names for --layout, listed in help, man pages and completions. `baraha`
/// is accepted as another name for nudi.
fn layout_parser() -> impl clap::builder::TypedValueParser<Value = Layout> {
    use clap::builder::{PossibleValue, PossibleValuesParser, TypedValueParser};
    let names = Layout::ALL
        .map(|l| PossibleValue::new(l.name()))
        .into_iter()
        .chain([PossibleValue::new("baraha").hide(true)]);
    PossibleValuesParser::new(names).try_map(|name| parse_layout(&name))
}

fn parse_layout(name: &str) -> Result<Layout, String> {
    name.parse()
        .map_err(|e: kannadi::legacy::UnknownLayout| e.to_string())
}

/// A `--legacy-font` value: the name, and the layout after its last `=`, if any.
fn parse_legacy_font(value: &str) -> Result<(String, Option<Layout>), String> {
    match value.rsplit_once('=') {
        Some((name, layout)) => Ok((name.to_string(), Some(parse_layout(layout)?))),
        None => Ok((value.to_string(), None)),
    }
}

struct Outcome {
    issues: bool,
    failed: bool,
}

fn run(files: &Files, settings: &Settings) -> Result<Outcome> {
    let plan = files::plan(&files.inputs, &files.out)?;
    for (path, e) in &plan.unreadable {
        eprintln!("error: {}: {e}", path.display());
    }
    let (mut converted, mut with_issues, mut failed) = (0, 0, plan.unreadable.len());
    for job in &plan.jobs {
        match convert_job(job, settings) {
            Ok(report) => {
                converted += 1;
                if report.has_problems() {
                    with_issues += 1;
                }
                if !report.is_empty() {
                    eprintln!("{}: {}", job.input.display(), report.summary());
                }
            }
            Err(e) => {
                failed += 1;
                eprintln!("error: {}: {e:#}", job.input.display());
            }
        }
    }
    let files = |n: usize| if n == 1 { "file" } else { "files" };
    eprintln!(
        "converted {converted} {}, {with_issues} with issues, {failed} failed, {} skipped",
        files(converted),
        plan.skipped
    );
    Ok(Outcome {
        issues: with_issues > 0,
        failed: failed > 0,
    })
}

/// Converts 1 file and writes it with its report.
fn convert_job(job: &Job, settings: &Settings) -> Result<Report> {
    let bytes = std::fs::read(&job.input).context("reading")?;
    let (out, report) = match job.format {
        Format::Txt => txt::convert_file(&bytes, settings)?,
        Format::Html => html::convert_file(&bytes, settings)?,
        Format::Docx => docx::convert_file(&bytes, settings)?,
        Format::Pdf => pdf::convert_file(&bytes, settings)?,
    };
    if let Some(dir) = job.output.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    std::fs::write(&job.output, out)
        .with_context(|| format!("writing {}", job.output.display()))?;
    let report_path = report::path_for(&job.output);
    if report.is_empty() {
        match std::fs::remove_file(&report_path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                return Err(e).with_context(|| format!("removing {}", report_path.display()))
            }
            _ => {}
        }
    } else {
        std::fs::write(&report_path, report.to_tsv())
            .with_context(|| format!("writing {}", report_path.display()))?;
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn added_fonts_may_name_their_layout() {
        assert_eq!(
            parse_legacy_font("My Font=akruti"),
            Ok(("My Font".to_string(), Some(Layout::Akruti)))
        );
        assert_eq!(
            parse_legacy_font("My Font"),
            Ok(("My Font".to_string(), None))
        );
        assert!(parse_legacy_font("My Font=akrti").is_err());
    }

    #[test]
    fn layouts_ignore_case_and_baraha_is_nudi() {
        for (name, layout) in [("AKRUTI", Layout::Akruti), ("baraha", Layout::Nudi)] {
            let cli =
                Cli::try_parse_from(["kannadi", "to-unicode", "in", "-o", "out", "--layout", name])
                    .unwrap();
            let Command::ToUnicode { layout: got, .. } = cli.command else {
                panic!("to-unicode")
            };
            assert_eq!(got, layout);
        }
        assert!(
            Cli::try_parse_from(["kannadi", "to-unicode", "in", "-o", "out", "--layout", "x"])
                .is_err()
        );
    }

    #[test]
    fn to_nudi_keeps_a_legacy_font_name_whole() {
        let cli = Cli::parse_from([
            "kannadi",
            "to-nudi",
            "in",
            "-o",
            "out",
            "--legacy-font",
            "Odd=Name",
        ]);
        let Command::ToNudi { legacy_font, .. } = cli.command else {
            panic!("to-nudi")
        };
        assert_eq!(legacy_font, ["Odd=Name"]);
    }

    #[test]
    fn the_layout_option_reaches_text_files_and_added_fonts() {
        let cli = Cli::parse_from([
            "kannadi",
            "to-unicode",
            "in",
            "--out",
            "out",
            "--layout",
            "akruti",
            "--legacy-font",
            "A",
            "--legacy-font",
            "B=nudi",
        ]);
        let Command::ToUnicode {
            layout,
            font,
            legacy_font,
            ..
        } = cli.command
        else {
            panic!("to-unicode")
        };
        let s = settings(
            Direction::ToUnicode(Options::default()),
            layout,
            font,
            &legacy_font,
        );
        assert_eq!(s.text_layout, Layout::Akruti);
        assert_eq!(
            s.fonts.extra_legacy,
            [
                ("A".to_string(), Layout::Akruti),
                ("B".to_string(), Layout::Nudi)
            ]
        );
    }
}
