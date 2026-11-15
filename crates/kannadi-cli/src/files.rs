//! Finds the files to convert and where each one is written.

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

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
/// its subfolders kept. Files already under `out` are left out. It is an error for
/// an output to be an input, or for 2 inputs to share an output, comparing paths
/// with symbolic links and `..` resolved, and ignoring case.
pub fn jobs(inputs: &[PathBuf], out: &Path) -> Result<(Vec<Job>, usize)> {
    let out_abs = resolve(out);
    let mut jobs = Vec::new();
    let mut skipped = 0;
    for input in inputs {
        let meta = std::fs::metadata(input).with_context(|| format!("{}", input.display()))?;
        if meta.is_dir() {
            let name = resolve(input)
                .file_name()
                .map(PathBuf::from)
                .unwrap_or_default();
            let mut files = Vec::new();
            walk(input, &mut files)?;
            for file in files {
                if resolve(&file).starts_with(&out_abs) {
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
    check_outputs(&jobs)?;
    Ok((jobs, skipped))
}

fn check_outputs(jobs: &[Job]) -> Result<()> {
    let key = |p: &Path| resolve(p).to_string_lossy().to_lowercase();
    let inputs: HashSet<String> = jobs.iter().map(|j| key(&j.input)).collect();
    let mut outputs: HashMap<String, &Path> = HashMap::new();
    for job in jobs {
        let output = key(&job.output);
        if inputs.contains(&output) {
            bail!("{}: output would overwrite an input", job.output.display());
        }
        if let Some(other) = outputs.insert(output, &job.input) {
            bail!(
                "{} and {} would both be written to {}",
                other.display(),
                job.input.display(),
                job.output.display()
            );
        }
    }
    Ok(())
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
        let kind = std::fs::symlink_metadata(&path)?.file_type();
        if kind.is_dir() {
            walk(&path, files)?;
        } else if !kind.is_symlink() || path.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

/// `path` made absolute, with symbolic links and `..` resolved as far as the
/// path exists.
fn resolve(path: &Path) -> PathBuf {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut existing = absolute.as_path();
    let mut rest = Vec::new();
    loop {
        if let Ok(real) = existing.canonicalize() {
            let mut out = real;
            for part in rest.iter().rev() {
                match part {
                    Component::ParentDir => {
                        out.pop();
                    }
                    Component::CurDir => {}
                    other => out.push(other),
                }
            }
            return out;
        }
        let (Some(parent), Some(last)) = (existing.parent(), existing.components().next_back())
        else {
            return absolute;
        };
        rest.push(last);
        existing = parent;
    }
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
    fn aliases_of_an_input_are_caught() {
        let tmp = tempfile::tempdir().unwrap();
        let docs = tmp.path().join("docs");
        std::fs::create_dir(&docs).unwrap();
        std::fs::write(docs.join("a.txt"), "").unwrap();
        let err = jobs(&[docs.join("a.txt")], &docs.join("x/..")).unwrap_err();
        assert!(err.to_string().contains("overwrite"), "{err}");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&docs, tmp.path().join("link")).unwrap();
            assert!(jobs(&[docs.join("a.txt")], &tmp.path().join("link")).is_err());
        }
    }

    #[test]
    fn outputs_must_not_collide_or_overwrite_other_inputs() {
        let tmp = tempfile::tempdir().unwrap();
        for dir in ["first", "second", "out/docs", "docs"] {
            std::fs::create_dir_all(tmp.path().join(dir)).unwrap();
        }
        for file in [
            "first/a.txt",
            "second/a.txt",
            "docs/a.txt",
            "out/docs/a.txt",
        ] {
            std::fs::write(tmp.path().join(file), "").unwrap();
        }
        let out = tmp.path().join("out");
        let p = |s: &str| tmp.path().join(s);
        let err = jobs(&[p("first/a.txt"), p("second/a.txt")], &out).unwrap_err();
        assert!(err.to_string().contains("both"), "{err}");
        let err = jobs(&[p("docs"), p("out/docs/a.txt")], &out).unwrap_err();
        assert!(err.to_string().contains("overwrite"), "{err}");
    }

    #[cfg(unix)]
    #[test]
    fn symbolic_link_loops_are_not_followed() {
        let tmp = tempfile::tempdir().unwrap();
        let docs = tmp.path().join("docs");
        std::fs::create_dir(&docs).unwrap();
        std::fs::write(docs.join("a.txt"), "").unwrap();
        std::os::unix::fs::symlink(&docs, docs.join("loop")).unwrap();
        let (jobs, _) = jobs(&[docs], &tmp.path().join("out")).unwrap();
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
