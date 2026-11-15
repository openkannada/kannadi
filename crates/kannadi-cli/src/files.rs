//! Finds the files to convert and where each one is written.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Txt,
}

impl Format {
    pub fn of(path: &Path) -> Option<Format> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "txt" => Some(Format::Txt),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Job {
    pub input: PathBuf,
    pub output: PathBuf,
    pub format: Format,
}

/// The jobs for `inputs`, and the number of files skipped in folders because
/// their format is not supported.
///
/// A file goes to `out/<file name>`. A folder goes to `out/<folder name>/`, with
/// its subfolders kept. Files already under `out` are left out.
pub fn jobs(inputs: &[PathBuf], out: &Path) -> Result<(Vec<Job>, usize)> {
    let out_abs = absolute(out);
    let mut jobs = Vec::new();
    let mut skipped = 0;
    for input in inputs {
        let meta = std::fs::metadata(input).with_context(|| format!("{}", input.display()))?;
        if meta.is_dir() {
            let name = absolute(input)
                .file_name()
                .map(PathBuf::from)
                .unwrap_or_default();
            let mut files = Vec::new();
            walk(input, &mut files)?;
            for file in files {
                if absolute(&file).starts_with(&out_abs) {
                    continue;
                }
                let Some(format) = Format::of(&file) else {
                    skipped += 1;
                    continue;
                };
                let relative = file.strip_prefix(input).unwrap_or(&file);
                jobs.push(Job {
                    output: out.join(&name).join(relative),
                    input: file,
                    format,
                });
            }
        } else {
            let Some(format) = Format::of(input) else {
                bail!("{}: unsupported file type", input.display());
            };
            let name = input.file_name().unwrap_or_default();
            jobs.push(Job {
                input: input.clone(),
                output: out.join(name),
                format,
            });
        }
    }
    for job in &jobs {
        if absolute(&job.output) == absolute(&job.input) {
            bail!("{}: output would overwrite the input", job.input.display());
        }
    }
    Ok((jobs, skipped))
}

fn walk(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("{}", dir.display()))?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<_>>()?;
    entries.sort();
    for path in entries {
        let hidden = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with('.'));
        if hidden {
            continue;
        }
        if path.is_dir() {
            walk(&path, files)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
}

fn absolute(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folders_keep_their_structure_and_skip_other_files() {
        let tmp = tempfile::tempdir().unwrap();
        let docs = tmp.path().join("docs");
        std::fs::create_dir_all(docs.join("sub")).unwrap();
        std::fs::write(docs.join("a.txt"), "").unwrap();
        std::fs::write(docs.join("sub/B.TXT"), "").unwrap();
        std::fs::write(docs.join("c.pdf"), "").unwrap();
        std::fs::write(docs.join(".hidden.txt"), "").unwrap();
        let out = tmp.path().join("out");
        let (jobs, skipped) = jobs(&[docs], &out).unwrap();
        let outputs: Vec<PathBuf> = jobs.iter().map(|j| j.output.clone()).collect();
        assert_eq!(
            outputs,
            [out.join("docs/a.txt"), out.join("docs/sub/B.TXT")]
        );
        assert_eq!(skipped, 1);
    }

    #[test]
    fn output_inside_the_input_folder_is_not_reconverted() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("out/x")).unwrap();
        std::fs::write(tmp.path().join("a.txt"), "").unwrap();
        std::fs::write(tmp.path().join("out/x/a.txt"), "").unwrap();
        let (jobs, _) = jobs(&[tmp.path().to_path_buf()], &tmp.path().join("out")).unwrap();
        assert_eq!(jobs.len(), 1);
    }

    #[test]
    fn named_files_must_be_supported_and_not_overwritten() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("a.txt");
        std::fs::write(&file, "").unwrap();
        assert!(jobs(std::slice::from_ref(&file), tmp.path()).is_err());
        let pdf = tmp.path().join("a.pdf");
        std::fs::write(&pdf, "").unwrap();
        assert!(jobs(&[pdf], &tmp.path().join("out")).is_err());
    }
}
