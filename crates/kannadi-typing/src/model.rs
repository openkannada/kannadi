//! The trained model and its file format.
//!
//! The file is `KNTY`, a format version, then the sections in a fixed order:
//! the graphones, the n-gram model, the lexicon (an `fst` automaton over unit
//! bytes whose value is a frequency class), the word pairs, and the ranking
//! weights. All numbers are little-endian.

use crate::bytes::{Reader, Writer};
use crate::complete::Completions;
use crate::lm::{Lm, Quant};
use crate::roman::{chunk_code, CHUNK_CODES};
use crate::units::{Unit, COUNT};
use fst::raw::Fst;

const MAGIC: &[u8; 4] = b"KNTY";
const VERSION: u32 = 1;

/// A pairing of a Latin chunk with 1 Kannada unit, such as "aa" with ಾ.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Graphone {
    pub chunk: u16,
    pub unit: Unit,
}

/// How the features of a candidate combine into its score.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Weights {
    /// Per natural-log unit of the word's corpus frequency.
    pub freq: f32,
    /// Added when the word is in the lexicon.
    pub known: f32,
    /// Per natural-log unit of how much likelier the previous word makes this one.
    pub pair: f32,
    /// Added when the strict Baraha reading of the input gives this word.
    pub rule: f32,
    /// Added to the rule reading when the input uses Baraha capitals, such as kannaDa.
    pub capitals: f32,
    /// Per natural-log unit of how often the user picked the word.
    pub user: f32,
    /// Added for each unit a completion adds after the letters run out.
    pub extend: f32,
    /// Beam width of the search through the lexicon.
    pub beam: f32,
    /// Beam width of the search outside the lexicon.
    pub open_beam: f32,
}

impl Weights {
    const FIELDS: usize = 9;

    fn to_array(self) -> [f32; Self::FIELDS] {
        [
            self.freq,
            self.known,
            self.pair,
            self.rule,
            self.capitals,
            self.user,
            self.extend,
            self.beam,
            self.open_beam,
        ]
    }

    fn from_array(a: [f32; Self::FIELDS]) -> Weights {
        Weights {
            freq: a[0],
            known: a[1],
            pair: a[2],
            rule: a[3],
            capitals: a[4],
            user: a[5],
            extend: a[6],
            beam: a[7],
            open_beam: a[8],
        }
    }
}

impl Default for Weights {
    fn default() -> Weights {
        Weights {
            freq: 0.5,
            known: 4.0,
            pair: 0.5,
            rule: 1.0,
            capitals: 40.0,
            user: 3.0,
            extend: -4.0,
            beam: 32.0,
            open_beam: 8.0,
        }
    }
}

/// Word pairs that occur together more often than chance, keyed by a 32-bit
/// fingerprint of the 2 words. A false match is possible and harmless.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pairs {
    pub keys: Vec<u32>,
    pub values: Vec<u8>,
    pub q: Option<Quant>,
}

impl Pairs {
    /// The fingerprint of `prev` followed by `word`.
    pub fn key(prev: &str, word: &str) -> u32 {
        let mut h: u32 = 0x811c_9dc5;
        for b in prev.bytes().chain([0xff]).chain(word.bytes()) {
            h ^= b as u32;
            h = h.wrapping_mul(0x0100_0193);
        }
        h
    }

    /// How much likelier `word` is after `prev` than on its own, as a natural log; 0 when unknown.
    pub fn lift(&self, prev: &str, word: &str) -> f32 {
        let (Some(q), Ok(i)) = (self.q, self.keys.binary_search(&Pairs::key(prev, word))) else {
            return 0.0;
        };
        q.decode(self.values[i])
    }
}

/// The trained model.
pub struct Model {
    pub graphones: Vec<Graphone>,
    pub lm: Lm,
    pub lexicon: Fst<Vec<u8>>,
    pub pairs: Pairs,
    pub weights: Weights,
    /// Graphone ids for each chunk code, built on load.
    by_chunk: Vec<Vec<u16>>,
    /// The best completion from each lexicon state, built on load.
    pub completions: Completions,
}

/// Why a model file could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BadModel(pub &'static str);

impl std::fmt::Display for BadModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "not a kannadi-typing model: {}", self.0)
    }
}

impl std::error::Error for BadModel {}

impl Model {
    /// Puts a model together from its parts. Graphone 0 is the word boundary,
    /// so `graphones[0]` is a placeholder.
    pub fn new(
        graphones: Vec<Graphone>,
        lm: Lm,
        lexicon: Fst<Vec<u8>>,
        pairs: Pairs,
        weights: Weights,
    ) -> Model {
        let mut by_chunk = vec![Vec::new(); CHUNK_CODES];
        for (id, g) in graphones.iter().enumerate().skip(1) {
            by_chunk[g.chunk as usize].push(id as u16);
        }
        let completions = Completions::new(&lexicon);
        Model {
            graphones,
            lm,
            lexicon,
            pairs,
            weights,
            by_chunk,
            completions,
        }
    }

    /// The graphones whose chunk is `chunk`.
    pub fn graphones_for(&self, chunk: &[u8]) -> &[u16] {
        &self.by_chunk[chunk_code(chunk) as usize]
    }

    /// The model as bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = Writer::default();
        w.u32(u32::from_le_bytes(*MAGIC));
        w.u32(VERSION);
        let chunks: Vec<u16> = self.graphones.iter().map(|g| g.chunk).collect();
        let units: Vec<u8> = self.graphones.iter().map(|g| g.unit).collect();
        w.u16s(&chunks);
        w.bytes(&units);
        self.lm.write(&mut w);
        w.bytes(self.lexicon.as_bytes());
        w.u32s(&self.pairs.keys);
        w.bytes(&self.pairs.values);
        let q = self.pairs.q.unwrap_or(Quant {
            min: 0.0,
            step: 0.0,
        });
        w.f32(q.min);
        w.f32(q.step);
        let weights = self.weights.to_array();
        w.u32(weights.len() as u32);
        weights.iter().for_each(|&x| w.f32(x));
        w.finish()
    }

    /// Reads a model written by [`Model::to_bytes`].
    pub fn from_bytes(bytes: &[u8]) -> Result<Model, BadModel> {
        let mut r = Reader::new(bytes);
        let short = BadModel("the file ends early");
        if r.u32() != Some(u32::from_le_bytes(*MAGIC)) {
            return Err(BadModel("wrong magic number"));
        }
        if r.u32() != Some(VERSION) {
            return Err(BadModel("unsupported version"));
        }
        let chunks = r.u16s().ok_or(short.clone())?;
        let units = r.bytes().ok_or(short.clone())?;
        if chunks.len() != units.len() || chunks.is_empty() {
            return Err(BadModel("graphone table"));
        }
        let mut graphones = Vec::with_capacity(chunks.len());
        for (i, (&chunk, &unit)) in chunks.iter().zip(units).enumerate() {
            let valid =
                (chunk as usize) < CHUNK_CODES && (unit as usize) < COUNT && (unit > 0 || i == 0);
            if !valid {
                return Err(BadModel("graphone out of range"));
            }
            graphones.push(Graphone { chunk, unit });
        }
        let lm = Lm::read(&mut r).ok_or(BadModel("n-gram model"))?;
        if lm.levels[0].prob.len() != graphones.len()
            || lm
                .levels
                .iter()
                .any(|l| l.ids.iter().any(|&id| id as usize >= graphones.len()))
        {
            return Err(BadModel("n-gram model does not match the graphones"));
        }
        let lexicon =
            Fst::new(r.bytes().ok_or(short.clone())?.to_vec()).map_err(|_| BadModel("lexicon"))?;
        let keys = r.u32s().ok_or(short.clone())?;
        let values = r.bytes().ok_or(short.clone())?.to_vec();
        let (min, step) = (r.f32().ok_or(short.clone())?, r.f32().ok_or(short.clone())?);
        if keys.len() != values.len() || keys.windows(2).any(|w| w[0] >= w[1]) {
            return Err(BadModel("word pairs"));
        }
        let q = (!keys.is_empty()).then_some(Quant { min, step });
        let n = r.u32().ok_or(short.clone())? as usize;
        if n != Weights::FIELDS {
            return Err(BadModel("weights"));
        }
        let mut a = [0.0; Weights::FIELDS];
        for x in &mut a {
            *x = r.f32().ok_or(short.clone())?;
        }
        if !r.is_empty() {
            return Err(BadModel("trailing bytes"));
        }
        Ok(Model::new(
            graphones,
            lm,
            lexicon,
            Pairs { keys, values, q },
            Weights::from_array(a),
        ))
    }
}

/// The frequency class stored in the lexicon for a word seen `count` times.
pub fn freq_class(count: u64) -> u64 {
    ((count as f64 + 1.0).log2() * 4.0).round() as u64
}

/// The natural log of the count a frequency class stands for, plus 1.
pub fn class_ln(class: u64) -> f32 {
    class as f32 / 4.0 * std::f32::consts::LN_2
}
