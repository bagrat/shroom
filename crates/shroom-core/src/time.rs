//! Time and facts. The core never reads a clock: every input arrives in an [`Envelope`]
//! the runner stamps (tests choose the stamps).

use serde::{Deserialize, Serialize};

/// Process-monotonic milliseconds. Doesn't advance in sleep on macOS — freshness and
/// deadlines only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
pub struct Mono(pub u64);

impl Mono {
    pub fn plus(self, ms: u64) -> Mono {
        Mono(self.0 + ms)
    }
    pub fn since(self, earlier: Mono) -> u64 {
        self.0.saturating_sub(earlier.0)
    }
}

/// Unix milliseconds — every expiry a person can see (the 1 h deferred page, an invite).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
pub struct Wall(pub i64);

/// Something a probe, the OS, the recorder or the cloud reported — or not yet.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Fact<T> {
    /// Not probed in this process.
    #[default]
    Unknown,
    Known { v: T, at: Mono },
}

impl<T> Fact<T> {
    pub fn known(v: T, at: Mono) -> Fact<T> {
        Fact::Known { v, at }
    }
    pub fn get(&self) -> Option<&T> {
        match self {
            Fact::Known { v, .. } => Some(v),
            Fact::Unknown => None,
        }
    }
    pub fn at(&self) -> Option<Mono> {
        match self {
            Fact::Known { at, .. } => Some(*at),
            Fact::Unknown => None,
        }
    }
    pub fn is(&self, want: &T) -> bool
    where
        T: PartialEq,
    {
        self.get() == Some(want)
    }
}

/// One input with its stamps.
#[derive(Debug, Clone)]
pub struct Envelope {
    pub mono: Mono,
    pub wall: Wall,
    pub event: crate::event::Event,
}
