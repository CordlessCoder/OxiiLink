use rand::{rngs, Rng};
type Random = rngs::ThreadRng;

pub struct Id(Vec<u8>);

impl Id {
    pub fn new(length: usize) -> Id {
        const BASE62: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

        let mut buf = Vec::with_capacity(length);
        let mut rand_thread = Random::default();
        for _ in 1..=length {
            buf.push(BASE62[rand_thread.gen::<usize>() % BASE62.len()])
        }
        Id(buf)
    }
    pub fn into_inner(self) -> Vec<u8> {
        self.0
    }
    pub fn possible_ids(length: usize) -> usize {
        62usize.pow(length as u32)
    }
}
