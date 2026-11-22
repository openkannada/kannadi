//! Writes the `unaccent` rules for Kannada: `cargo run -p kannadi-search --example postgres_rules > kannada.rules`.

fn main() {
    print!("{}", kannadi_search::postgres::unaccent_rules());
}
