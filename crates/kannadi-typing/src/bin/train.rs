//! Builds, tunes and evaluates kannadi-typing models. See the crate README for the data.
//!
//! ```text
//! kannadi-typing-train build --aksharantar kan_train.json --counts wiki-train.tsv \
//!     --sentences kn.wiki-filt.train.text.tsv --exclude dev.tsv --exclude test.tsv --out kn.typing
//! kannadi-typing-train tune --model kn.typing --dev dev.tsv --out kn.typing
//! kannadi-typing-train eval --model kn.typing --test test.tsv
//! ```

use kannadi_typing::train::{self, data, eval};
use kannadi_typing::units::{text, units};
use kannadi_typing::{Engine, Model};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use std::process::ExitCode;

struct Args(Vec<(String, String)>);

impl Args {
    fn parse(raw: &[String]) -> Result<Args, String> {
        let mut out = Vec::new();
        let mut it = raw.iter();
        while let Some(flag) = it.next() {
            let name = flag
                .strip_prefix("--")
                .ok_or(format!("expected a --flag, got {flag}"))?;
            let value = it.next().ok_or(format!("--{name} needs a value"))?;
            out.push((name.to_string(), value.clone()));
        }
        Ok(Args(out))
    }
    fn all(&self, name: &str) -> Vec<&str> {
        self.0
            .iter()
            .filter(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
            .collect()
    }
    fn get(&self, name: &str) -> Option<&str> {
        self.all(name).last().copied()
    }
    fn path(&self, name: &str) -> Result<PathBuf, String> {
        self.get(name)
            .map(PathBuf::from)
            .ok_or(format!("--{name} is required"))
    }
    fn num<T: std::str::FromStr>(&self, name: &str, default: T) -> Result<T, String> {
        self.get(name).map_or(Ok(default), |v| {
            v.parse()
                .map_err(|_| format!("--{name}: not a number: {v}"))
        })
    }
}

fn main() -> ExitCode {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let result = match raw.split_first() {
        Some((cmd, rest)) => Args::parse(rest).and_then(|a| match cmd.as_str() {
            "build" => build(&a),
            "tune" => tune(&a),
            "eval" => evaluate(&a),
            "show" => show(&a),
            _ => Err(format!(
                "unknown command {cmd}; expected build, tune or eval"
            )),
        }),
        None => Err("usage: kannadi-typing-train build|tune|eval --flag value ...".into()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn io<T>(r: std::io::Result<T>, what: &str) -> Result<T, String> {
    r.map_err(|e| format!("{what}: {e}"))
}

fn build(a: &Args) -> Result<(), String> {
    let mut exclude: HashSet<String> = HashSet::new();
    for p in a.all("exclude") {
        exclude.extend(data::natives(&io(data::dakshina(p.as_ref()), p)?));
    }
    let mut pairs = Vec::new();
    let mut seen_pairs = HashSet::new();
    let mut natives: HashSet<Vec<u8>> = HashSet::new();
    let sources: Vec<&str> = a.all("source");
    // Dakshina's romanizations were typed by people, like the editor's input; the mined pairs were not.
    let dakshina_weight: usize = a.num("dakshina-weight", 1)?;
    for p in a.all("aksharantar") {
        for row in io(data::aksharantar(p.as_ref()), p)? {
            if !sources.is_empty() && !sources.contains(&row.source.as_str()) {
                continue;
            }
            let Some((r, u)) = data::pair(&row.roman, &row.native) else {
                continue;
            };
            if exclude.contains(&text(&u)) || !seen_pairs.insert((r.clone(), u.clone())) {
                continue;
            }
            natives.insert(u.clone());
            let times = if row.source == "Dakshina" {
                dakshina_weight
            } else {
                1
            };
            pairs.extend(std::iter::repeat_n((r, u), times));
        }
    }
    for p in a.all("dakshina") {
        for (native, roman) in io(data::dakshina(p.as_ref()), p)? {
            let Some((r, u)) = data::pair(&roman, &native) else {
                continue;
            };
            if !exclude.contains(&text(&u)) && seen_pairs.insert((r.clone(), u.clone())) {
                natives.insert(u.clone());
                pairs.extend(std::iter::repeat_n((r, u), dakshina_weight));
            }
        }
    }
    let limit: usize = a.num("limit", usize::MAX)?;
    pairs.truncate(limit);
    eprintln!("{} training pairs", pairs.len());
    let min_word: u64 = a.num("min-word-count", 2)?;
    let mut lexicon: BTreeMap<Vec<u8>, u64> = BTreeMap::new();
    let mut counted: HashMap<Vec<u8>, u64> = HashMap::new();
    for p in a.all("counts") {
        for (w, c) in io(data::counts(p.as_ref()), p)? {
            if let Some(u) = units(&w) {
                *counted.entry(u).or_default() += c;
            }
        }
    }
    for (u, &c) in &counted {
        if c >= min_word {
            lexicon.insert(u.clone(), c);
        }
    }
    // Words of the training pairs join the lexicon only if the corpus has them this often (0: always).
    let pair_words: u64 = a.num("pair-word-count", 0)?;
    for u in &natives {
        let c = counted.get(u).copied().unwrap_or(0);
        if c >= pair_words {
            lexicon.entry(u.clone()).or_insert(c);
        }
    }
    if a.get("hide-excluded") == Some("yes") {
        lexicon.retain(|u, _| !exclude.contains(&text(u)));
    }
    let mut sentences = Vec::new();
    for p in a.all("sentences") {
        sentences.extend(io(
            data::sentences(p.as_ref(), a.num("sentence-limit", usize::MAX)?),
            p,
        )?);
    }
    let settings = train::Settings {
        order: a.num("order", 4)?,
        min_count: a
            .get("min-count")
            .unwrap_or("1,1,2,2")
            .split(',')
            .map(|s| s.parse().map_err(|_| format!("--min-count: {s}")))
            .collect::<Result<_, _>>()?,
        em_iterations: a.num("em", 5)?,
        min_graphone: a.num("min-graphone", 3)?,
        min_pair: a.num("min-pair", 5)?,
        max_pairs: a.num("max-pairs", 200_000)?,
        threads: a.num(
            "threads",
            std::thread::available_parallelism().map_or(4, |n| n.get()),
        )?,
    };
    let model = train::build(
        &train::Inputs {
            pairs,
            lexicon,
            sentences,
        },
        &settings,
    );
    let bytes = model.to_bytes();
    let out = a.path("out")?;
    io(std::fs::write(&out, &bytes), "writing the model")?;
    eprintln!("wrote {} ({} bytes)", out.display(), bytes.len());
    Ok(())
}

fn load(a: &Args) -> Result<Engine, String> {
    let p = a.path("model")?;
    let bytes = io(std::fs::read(&p), "reading the model")?;
    Ok(Engine::new(
        Model::from_bytes(&bytes).map_err(|e| e.to_string())?,
    ))
}

/// Word items from `--<flag>` (a Dakshina lexicon), then in-sentence items from
/// `--context-native` and `--context-roman`, either of which may be absent.
fn test_items(a: &Args, flag: &str) -> Result<Vec<eval::Item>, String> {
    let mut items = Vec::new();
    if let Some(p) = a.get(flag) {
        items = eval::items(&io(data::dakshina(p.as_ref()), "test set")?);
    }
    if let (Some(n), Some(r)) = (a.get("context-native"), a.get("context-roman")) {
        let read = |p: &str| {
            io(std::fs::read_to_string(p), p)
                .map(|s| s.lines().map(String::from).collect::<Vec<_>>())
        };
        let limit: usize = a.num("context-limit", usize::MAX)?;
        items.extend(
            eval::context_items(&read(n)?, &read(r)?)
                .into_iter()
                .take(limit),
        );
    }
    if items.is_empty() {
        return Err(format!(
            "give --{flag}, or --context-native and --context-roman"
        ));
    }
    Ok(items)
}

fn tune(a: &Args) -> Result<(), String> {
    let mut engine = load(a)?;
    let items = test_items(a, "dev")?;
    let mut w = engine.model().weights();
    if let Some(b) = a.get("beam") {
        w.beam = b.parse().map_err(|_| "--beam")?;
    }
    if let Some(b) = a.get("open-beam") {
        w.open_beam = b.parse().map_err(|_| "--open-beam")?;
    }
    if !engine.set_weights(w) {
        return Err(format!(
            "beams must be at most {}",
            kannadi_typing::Weights::MAX_BEAM
        ));
    }
    let (lists, _) = eval::candidates(&engine, &items);
    let before = eval::score(&engine, &items, &lists, &w);
    let tuned = eval::tune(&engine, &items, &lists);
    let cut = eval::prefixes(&items);
    let (cut_lists, _) = eval::candidates(&engine, &cut);
    let tuned = eval::tune_extend(&engine, (&items, &lists), (&cut, &cut_lists), tuned);
    let after = eval::score(&engine, &items, &lists, &tuned);
    let cut_after = eval::score(&engine, &cut, &cut_lists, &tuned);
    eprintln!(
        "words typed 2 letters short: intended word in the first 5 for {:.2}%",
        eval::Report::pct(cut_after.top5, cut_after.items)
    );
    eprintln!(
        "dev top-1 {:.2} -> {:.2}, top-5 {:.2} -> {:.2}; weights {tuned:?}",
        eval::Report::pct(before.top1, before.items),
        eval::Report::pct(after.top1, after.items),
        eval::Report::pct(before.top5, before.items),
        eval::Report::pct(after.top5, after.items),
    );
    if let Some(out) = a.get("out") {
        engine.set_weights(tuned);
        io(
            std::fs::write(out, engine.model().to_bytes()),
            "writing the model",
        )?;
    }
    Ok(())
}

fn evaluate(a: &Args) -> Result<(), String> {
    let engine = load(a)?;
    let items = test_items(a, "test")?;
    let mut w = engine.model().weights();
    if let Some(p) = a.get("pair") {
        w.pair = p.parse().map_err(|_| "--pair")?;
    }
    if let Some(b) = a.get("beam") {
        w.beam = b.parse().map_err(|_| "--beam")?;
    }
    let mut engine = engine;
    if !engine.set_weights(w) {
        return Err(format!(
            "beams must be at most {}",
            kannadi_typing::Weights::MAX_BEAM
        ));
    }
    let (lists, times) = eval::candidates(&engine, &items);
    let mut r = eval::score(&engine, &items, &lists, &w);
    r.times = times;
    println!("| Items | Top-1 | Top-5 | Top-1, words in the lexicon | Words in the lexicon | Median time | 99th percentile |");
    println!("|---:|---:|---:|---:|---:|---:|---:|");
    println!(
        "| {} | {:.2}% | {:.2}% | {:.2}% | {:.1}% | {:.2} ms | {:.2} ms |",
        r.items,
        eval::Report::pct(r.top1, r.items),
        eval::Report::pct(r.top5, r.items),
        eval::Report::pct(r.known_top1, r.known),
        eval::Report::pct(r.known, r.items),
        r.quantile(0.5).as_secs_f64() * 1e3,
        r.quantile(0.99).as_secs_f64() * 1e3,
    );
    if a.get("prefixes") == Some("yes") {
        let cut = eval::prefixes(&items);
        let (cl, _) = eval::candidates(&engine, &cut);
        let c = eval::score(&engine, &cut, &cl, &w);
        println!(
            "Words typed 2 letters short: {} items, intended word in the first 5 for {:.2}%",
            c.items,
            eval::Report::pct(c.top5, c.items)
        );
    }
    if let Some(min) = a.get("min-top1") {
        let min: f64 = min.parse().map_err(|_| "--min-top1")?;
        let got = eval::Report::pct(r.top1, r.items);
        if got < min {
            return Err(format!("top-1 is {got:.2}%, below {min}%"));
        }
    }
    if let Some(n) = a.get("errors") {
        let n: usize = n.parse().map_err(|_| "--errors")?;
        for (it, list) in items
            .iter()
            .zip(&lists)
            .filter(|(it, l)| l.first().is_none_or(|c| !it.refs.contains(&c.word)))
            .take(n)
        {
            let top: Vec<String> = list
                .iter()
                .take(3)
                .map(|c| format!("{} {:.1}", c.word, c.score(&w)))
                .collect();
            eprintln!("{}\t{}\t{}", it.input, it.refs.join(","), top.join(" | "));
        }
    }
    Ok(())
}

fn show(a: &Args) -> Result<(), String> {
    let engine = load(a)?;
    let w = engine.model().weights();
    for input in a.all("input") {
        let t = std::time::Instant::now();
        let c = engine.candidates(input, a.get("prev"));
        let took = t.elapsed();
        let top: Vec<String> = c
            .iter()
            .take(6)
            .map(|c| {
                format!(
                    "{}{} {:.1}",
                    c.word,
                    if c.extra > 0 { "+" } else { "" },
                    c.score(&w)
                )
            })
            .collect();
        println!(
            "{input} ({:.2} ms): {}",
            took.as_secs_f64() * 1e3,
            top.join(" | ")
        );
    }
    Ok(())
}
