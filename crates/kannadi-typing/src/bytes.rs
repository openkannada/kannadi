//! Little-endian reading and writing for the model file.

#[derive(Default)]
pub(crate) struct Writer(Vec<u8>);

impl Writer {
    pub fn u32(&mut self, x: u32) {
        self.0.extend_from_slice(&x.to_le_bytes());
    }
    pub fn f32(&mut self, x: f32) {
        self.0.extend_from_slice(&x.to_le_bytes());
    }
    pub fn bytes(&mut self, b: &[u8]) {
        self.u32(b.len() as u32);
        self.0.extend_from_slice(b);
    }
    pub fn u16s(&mut self, v: &[u16]) {
        self.u32(v.len() as u32);
        v.iter()
            .for_each(|x| self.0.extend_from_slice(&x.to_le_bytes()));
    }
    pub fn u32s(&mut self, v: &[u32]) {
        self.u32(v.len() as u32);
        v.iter().for_each(|&x| self.u32(x));
    }
    pub fn finish(self) -> Vec<u8> {
        self.0
    }
}

pub(crate) struct Reader<'a> {
    rest: &'a [u8],
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Reader<'a> {
        Reader { rest: bytes }
    }
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        if n > self.rest.len() {
            return None;
        }
        let (head, tail) = self.rest.split_at(n);
        self.rest = tail;
        Some(head)
    }
    pub fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    pub fn f32(&mut self) -> Option<f32> {
        Some(f32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    pub fn bytes(&mut self) -> Option<&'a [u8]> {
        let n = self.u32()? as usize;
        self.take(n)
    }
    pub fn u16s(&mut self) -> Option<Vec<u16>> {
        let n = self.u32()? as usize;
        let b = self.take(n.checked_mul(2)?)?;
        Some(
            b.as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_le_bytes(*c))
                .collect(),
        )
    }
    pub fn u32s(&mut self) -> Option<Vec<u32>> {
        let n = self.u32()? as usize;
        let b = self.take(n.checked_mul(4)?)?;
        Some(
            b.as_chunks::<4>()
                .0
                .iter()
                .map(|c| u32::from_le_bytes(*c))
                .collect(),
        )
    }
    pub fn is_empty(&self) -> bool {
        self.rest.is_empty()
    }
}
