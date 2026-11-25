//! Prints the kannadi-search keys of each line of standard input, as a Postgres
//! array_to_tsvector value: `cargo run -p kannadi-search --example postgres_keys < docs.txt`.

use std::io::{self, BufRead};

fn main() {
    let analyzer = kannadi_search::Analyzer::new();
    for line in io::stdin().lock().lines() {
        let line = line.expect("standard input is UTF-8 text");
        let keys: Vec<String> = analyzer
            .keys(&line)
            .into_iter()
            .map(|k| format!("'{}'", k.replace('\'', "''")))
            .collect();
        println!("array_to_tsvector(ARRAY[{}]::text[])", keys.join(", "));
    }
}
