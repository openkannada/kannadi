//! `kannadi`: converts folders of documents between Nudi or Baraha ANSI and Unicode.

mod convert;
mod docx;
mod edits;
mod files;
mod fonts;
mod html;
mod pdf;
mod report;
mod txt;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use kannadi::legacy::Options;

use convert::{Direction, Settings};
use files::{Format, Job};
use report::Report;

#[derive(Parser)]
#[command(name = "kannadi", version, about)]
/// Converts Kannada documents typed in Nudi or Baraha ANSI fonts to Unicode, and back.
///
/// Each converted file gets a report next to it, FILE.report.tsv, when anything
/// could not be converted cleanly.
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Convert Nudi and Baraha text to Unicode.
    ToUnicode {
        #[command(flatten)]
        common: Common,
        /// Write digits as Kannada digits, for text set in the k variants of the
        /// Nudi fonts.
        #[arg(long)]
        kannada_digits: bool,
        /// Keep a zero typed between letters as a digit. By default it becomes ಂ.
        #[arg(long)]
        keep_zeros: bool,
    },
    /// Convert Unicode Kannada to Nudi and Baraha glyph codes.
    ToNudi {
        #[command(flatten)]
        common: Common,
    },
}

#[derive(Args)]
struct Common {
    /// Files and folders to convert. Folders are searched for .txt, .htm, .html,
    /// .docx and .pdf files. PDF text goes to NAME.pdf.txt, to Unicode only.
    #[arg(required = true)]
    inputs: Vec<PathBuf>,
    /// Folder to write the converted files to.
    #[arg(short, long)]
    out: PathBuf,
    /// Exit with status 1 if any file has unknown glyphs, malformed syllables or
    /// unencodable characters.
    #[arg(long)]
    strict: bool,
    /// Font for converted text in HTML and DOCX: Noto Sans Kannada by default for
    /// to-unicode, Nudi 01 e for to-nudi.
    #[arg(long, value_name = "NAME")]
    font: Option<String>,
    /// Also treat this font as a Nudi or Baraha ANSI font. Repeat for more.
    #[arg(long, value_name = "NAME")]
    legacy_font: Vec<String>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let (common, direction) = match cli.command {
        Command::ToUnicode {
            common,
            kannada_digits,
            keep_zeros,
        } => {
            let options = Options {
                kannada_digits,
                zero_as_anusvara: !keep_zeros,
            };
            (common, Direction::ToUnicode(options))
        }
        Command::ToNudi { common } => (common, Direction::ToNudi),
    };
    let mut settings = Settings::new(direction);
    if let Some(font) = &common.font {
        match direction {
            Direction::ToUnicode(_) => settings.fonts.unicode.clone_from(font),
            Direction::ToNudi => settings.fonts.legacy.clone_from(font),
        }
    }
    settings.fonts.extra_legacy.clone_from(&common.legacy_font);
    match run(&common, &settings) {
        Ok(Outcome { failed: true, .. }) => ExitCode::from(2),
        Ok(Outcome { issues: true, .. }) if common.strict => ExitCode::from(1),
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(2)
        }
    }
}

struct Outcome {
    issues: bool,
    failed: bool,
}

fn run(common: &Common, settings: &Settings) -> Result<Outcome> {
    let plan = files::plan(&common.inputs, &common.out)?;
    for (path, e) in &plan.unreadable {
        eprintln!("error: {}: {e}", path.display());
    }
    let (mut converted, mut with_issues, mut failed) = (0, 0, plan.unreadable.len());
    for job in &plan.jobs {
        match convert_job(job, settings) {
            Ok(report) => {
                converted += 1;
                if !report.is_empty() {
                    with_issues += 1;
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
