//! `kannadi`: converts folders of documents between Nudi or Baraha ANSI and Unicode.

mod convert;
mod files;
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
        /// Write digits as Kannada digits, for text set in the `k` Nudi fonts.
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
    /// Files and folders to convert. Folders are searched for .txt files.
    #[arg(required = true)]
    inputs: Vec<PathBuf>,
    /// Folder to write the converted files to.
    #[arg(short, long)]
    out: PathBuf,
    /// Exit with status 1 if any file has unknown glyphs, malformed syllables or
    /// unencodable characters.
    #[arg(long)]
    strict: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let (common, settings) = match cli.command {
        Command::ToUnicode {
            common,
            kannada_digits,
            keep_zeros,
        } => {
            let options = Options {
                kannada_digits,
                zero_as_anusvara: !keep_zeros,
            };
            (common, settings(Direction::ToUnicode, options))
        }
        Command::ToNudi { common } => (common, settings(Direction::ToNudi, Options::default())),
    };
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

fn settings(direction: Direction, options: Options) -> Settings {
    Settings { direction, options }
}

struct Outcome {
    issues: bool,
    failed: bool,
}

fn run(common: &Common, settings: &Settings) -> Result<Outcome> {
    let (jobs, skipped) = files::jobs(&common.inputs, &common.out)?;
    let mut outcome = Outcome {
        issues: false,
        failed: false,
    };
    let mut with_issues = 0;
    for job in &jobs {
        match convert_job(job, settings) {
            Ok(report) if report.is_empty() => {}
            Ok(report) => {
                with_issues += 1;
                eprintln!("{}: {}", job.input.display(), report.summary());
            }
            Err(e) => {
                outcome.failed = true;
                eprintln!("error: {}: {e:#}", job.input.display());
            }
        }
    }
    outcome.issues = with_issues > 0;
    eprintln!(
        "converted {} files, {with_issues} with issues, {skipped} skipped",
        jobs.len()
    );
    Ok(outcome)
}

/// Converts 1 file and writes it with its report.
fn convert_job(job: &Job, settings: &Settings) -> Result<Report> {
    let bytes = std::fs::read(&job.input).context("reading")?;
    let (out, report) = match job.format {
        Format::Txt => txt::convert_file(&bytes, settings)?,
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
