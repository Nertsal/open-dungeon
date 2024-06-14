use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Id(u64);

#[derive(Debug, Clone)]
pub struct IdGenerator {
    next: Id,
}

impl Default for IdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl IdGenerator {
    pub fn new() -> Self {
        Self { next: Id(0) }
    }

    pub fn gen(&mut self) -> Id {
        let id = self.next;
        self.next.0 += 1;
        id
    }
}
