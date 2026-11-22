//! Builds kannadi-search's data and scores search analyzers. See ../README.md, "Evaluation".

mod gold;
mod protect;
mod rules;
mod score;
mod systems;
mod variants;
mod wiki;

use std::fs;
use std::io::{self, BufReader};

const MORPH_REV: &str = "9366c23f32324d0ac631e1c61276d237a4151336";
const SEED: u64 = 2026_11_22;

const USAGE: &str = "usage:
  kannadi-search-eval rules OUT.tsv
  kannadi-search-eval protect WIKI-TRAIN-COUNTS.tsv OUT.txt
  kannadi-search-eval wiki N OUT-DIR < knwiki-pages-articles.xml
  kannadi-search-eval gov BENCHMARK-LINES-DIR OUT-DIR
  kannadi-search-eval gold LEXICON.tsv CLEAN-DOCS.tsv MAX-QUERIES OUT.tsv WIKI-TRAIN-COUNTS.tsv
  kannadi-search-eval score QUERIES.tsv DOCS.tsv";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |i: usize| args.get(i).unwrap_or_else(|| fail()).as_str();
    match args.first().map(String::as_str) {
        Some("rules") => write(arg(1), &rules::table(MORPH_REV, args.get(2).is_some_and(|a| a == "--predicates"))),
        Some("protect") => {
            let counts = protect::read_counts(&read(arg(1)));
            let words = protect::protected(&counts);
            let mut out = String::from(
                "# Words kannadi-search's stemmer keeps whole: they end like an inflected noun, but Kannada\n\
                 # Wikipedia has 2 or more other forms built on them (crates/kannadi-search/eval, `protect`).\n\
                 # Includes words counted in Kannada Wikipedia (https://kn.wikipedia.org), written by its\n\
                 # contributors. This list is licensed under CC BY 4.0.\n",
            );
            for w in words {
                out.push_str(&w);
                out.push('\n');
            }
            write(arg(2), &out);
        }
        Some("wiki") => {
            let n: usize = arg(1).parse().unwrap_or_else(|_| fail());
            let all = wiki::held_out_sentences(BufReader::new(io::stdin().lock()));
            eprintln!("{} held-out sentences", all.len());
            let step = (all.len() / n).max(1);
            let docs: Vec<(String, String)> = all
                .into_iter()
                .step_by(step)
                .take(n)
                .enumerate()
                .map(|(i, s)| (format!("w{:05}", i + 1), s))
                .collect();
            let mut rng = variants::Rng::new(SEED);
            let varied: Vec<(String, String)> = docs
                .iter()
                .map(|(id, t)| (id.clone(), variants::vary(t, &mut rng)))
                .collect();
            write(&format!("{}/wiki-docs.tsv", arg(2)), &docs_tsv(&docs));
            write(&format!("{}/wiki-variant-docs.tsv", arg(2)), &docs_tsv(&varied));
        }
        Some("gov") => {
            let (mut legacy, mut expected) = (Vec::new(), Vec::new());
            let mut paths: Vec<_> = fs::read_dir(arg(1)).expect("read dir").flatten().map(|e| e.path()).collect();
            paths.sort();
            for path in paths.iter().filter(|p| p.extension().is_some_and(|e| e == "tsv")) {
                let text = read(path.to_str().unwrap());
                let mut lines = text.lines();
                let header: Vec<&str> = lines.next().unwrap_or_default().split('\t').collect();
                let col = |name: &str| header.iter().position(|h| *h == name).expect(name);
                let (id, codes, exp) = (col("id"), col("codes"), col("expected"));
                for line in lines {
                    let f: Vec<&str> = line.split('\t').collect();
                    let bytes: Vec<u8> = f[codes].split(' ').filter_map(|h| u8::from_str_radix(h, 16).ok()).collect();
                    legacy.push((f[id].to_string(), kannadi::legacy::decode_windows_1252(&bytes)));
                    expected.push((f[id].to_string(), f[exp].to_string()));
                }
            }
            write(&format!("{}/gov-legacy-docs.tsv", arg(2)), &docs_tsv(&legacy));
            write(&format!("{}/gov-docs.tsv", arg(2)), &docs_tsv(&expected));
        }
        Some("gold") => {
            let docs = read_docs(arg(2));
            let max: usize = arg(3).parse().unwrap_or_else(|_| fail());
            let counts = protect::read_counts(&read(arg(5)));
            let g = gold::build(&read(arg(1)), &counts, &docs, max, SEED);
            write(arg(4), &gold::write(&g));
        }
        Some("score") => {
            let queries = gold::read(&read(arg(1)));
            let docs = read_docs(arg(2));
            println!("| Analyzer | Recall | Precision |\n|---|---:|---:|");
            for system in systems::all() {
                let s = score::score(&system, &docs, &queries);
                println!(
                    "| {} | {:.1}% | {:.1}% |",
                    system.name,
                    100.0 * s.recall,
                    100.0 * s.precision
                );
                eprintln!("{}: found {} of {} relevant", system.name, s.found, s.relevant);
            }
        }
        Some("prefer") => {
            let counts = protect::read_counts(&read(arg(1)));
            let mut out = String::from(
                "# Stems that settle Kannada words with more than 1 reading, and how often Kannada\n\
                 # Wikipedia has each as a word (crates/kannadi-search/eval, `prefer`). Includes words\n\
                 # counted in Kannada Wikipedia (https://kn.wikipedia.org), written by its contributors.\n\
                 # This list is licensed under CC BY 4.0.\n",
            );
            for (w, n) in protect::preferred(&counts) {
                out.push_str(&format!("{w}\t{n}\n"));
            }
            write(arg(2), &out);
        }
        Some("why") => {
            let counts = protect::read_counts(&read(arg(1)));
            for w in &args[2..] {
                let s = kannadi_search::stem::strip_ending(w);
                println!("{w}: strips to {s:?}");
                let of_s: std::collections::HashSet<String> =
                    s.as_deref().map(kannadi_search::stem::forms).unwrap_or_default().into_iter().collect();
                for f in kannadi_search::stem::forms(w) {
                    if let Some(n) = counts.get(&f) {
                        println!("  {f} {n} {}", if of_s.contains(&f) { "(also of s)" } else { "" });
                    }
                }
            }
        }
        Some("errors") => {
            let queries = gold::read(&read(arg(1)));
            let docs = read_docs(arg(2));
            let (false_hits, misses) = score::errors(&docs, &queries);
            println!("# false matches: {}", false_hits.iter().map(|x| x.1).sum::<usize>());
            for (e, n) in false_hits.iter().take(150) {
                println!("{n}\t{e}");
            }
            println!("# misses: {}", misses.iter().map(|x| x.1).sum::<usize>());
            for (e, n) in misses.iter().take(150) {
                println!("{n}\t{e}");
            }
        }
        _ => fail(),
    }
}

fn fail() -> ! {
    eprintln!("{USAGE}");
    std::process::exit(2)
}

fn read(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn write(path: &str, text: &str) {
    fs::write(path, text).unwrap_or_else(|e| panic!("{path}: {e}"));
}

fn docs_tsv(docs: &[(String, String)]) -> String {
    let mut out = String::from("id\ttext\n");
    for (id, text) in docs {
        out.push_str(&format!("{id}\t{}\n", text.replace(['\t', '\n'], " ")));
    }
    out
}

fn read_docs(path: &str) -> Vec<(String, String)> {
    read(path)
        .lines()
        .skip(1)
        .filter_map(|l| l.split_once('\t').map(|(a, b)| (a.to_string(), b.to_string())))
        .collect()
}
