//! The commonest word of the lexicon that starts with a given prefix.
//!
//! In a minimal automaton the words reachable from a state are the same however
//! the state was reached, so each state's best completion is computed once, on
//! load, and the search follows it in a few steps.

use fst::raw::{CompiledAddr, Fst};

/// For each state: the largest sum of outputs from it to a final state, and the
/// transition that starts it, or `STOP` when the state's own final output wins.
pub struct Completions {
    addrs: Vec<CompiledAddr>,
    best: Vec<u64>,
    next: Vec<u16>,
}

const STOP: u16 = u16::MAX;

impl Completions {
    /// Walks every state of `fst` once.
    pub fn new<D: AsRef<[u8]>>(fst: &Fst<D>) -> Completions {
        let mut done: std::collections::HashMap<CompiledAddr, (u64, u16)> =
            std::collections::HashMap::new();
        // (state, next transition to visit)
        let mut stack: Vec<(CompiledAddr, usize)> = vec![(fst.root().addr(), 0)];
        while let Some(&mut (addr, ref mut i)) = stack.last_mut() {
            let node = fst.node(addr);
            if *i < node.len() {
                let child = node.transition_addr(*i);
                *i += 1;
                if !done.contains_key(&child) {
                    stack.push((child, 0));
                }
                continue;
            }
            stack.pop();
            let mut best = if node.is_final() {
                Some((node.final_output().value(), STOP))
            } else {
                None
            };
            for (k, t) in node.transitions().enumerate() {
                let Some(&(b, _)) = done.get(&t.addr) else {
                    continue;
                };
                let v = t.out.value() + b;
                if best.is_none_or(|(x, _)| v > x) {
                    best = Some((v, k as u16));
                }
            }
            done.insert(addr, best.unwrap_or((0, STOP)));
        }
        let mut all: Vec<(CompiledAddr, (u64, u16))> = done.into_iter().collect();
        all.sort_unstable_by_key(|e| e.0);
        Completions {
            addrs: all.iter().map(|e| e.0).collect(),
            best: all.iter().map(|e| e.1 .0).collect(),
            next: all.iter().map(|e| e.1 .1).collect(),
        }
    }

    /// The number of states.
    pub fn len(&self) -> usize {
        self.addrs.len()
    }

    /// Whether the automaton has no states, which never happens for a built one.
    pub fn is_empty(&self) -> bool {
        self.addrs.is_empty()
    }

    /// The units that complete the best word from state `addr`, and the sum of
    /// outputs along them, including the final output.
    pub fn best<D: AsRef<[u8]>>(
        &self,
        fst: &Fst<D>,
        mut addr: CompiledAddr,
    ) -> Option<(Vec<u8>, u64)> {
        let i = self.addrs.binary_search(&addr).ok()?;
        let total = self.best[i];
        let mut units = Vec::new();
        loop {
            let i = self.addrs.binary_search(&addr).ok()?;
            let node = fst.node(addr);
            match self.next[i] {
                STOP => return node.is_final().then_some((units, total)),
                k => {
                    let t = node.transition(k as usize);
                    units.push(t.inp);
                    addr = t.addr;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fst::MapBuilder;

    #[test]
    fn follows_the_commonest_word() {
        let mut b = MapBuilder::memory();
        for (k, v) in [(&b"ab"[..], 3u64), (b"abc", 9), (b"abd", 4), (b"b", 1)] {
            b.insert(k, v).unwrap();
        }
        let fst = Fst::new(b.into_inner().unwrap()).unwrap();
        let c = Completions::new(&fst);
        let root = fst.root();
        let a = root.transition(root.find_input(b'a').unwrap());
        let (rest, total) = c.best(&fst, a.addr).unwrap();
        assert_eq!(rest, b"bc");
        assert_eq!(a.out.value() + total, 9);
        let (rest, total) = c.best(&fst, root.addr()).unwrap();
        assert_eq!((rest.as_slice(), total), (&b"abc"[..], 9));
    }
}
