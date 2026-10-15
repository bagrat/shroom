//! The runner's stash of pasted secrets (law 13). A paste arrives once, over the
//! `paste_secret` command; its text moves in here and the core only ever sees a handle
//! and the paste's shape. Text is never logged and never echoed. The stash's own copy is
//! overwritten with zeros when it leaves — consumed (saved to `token.json`), superseded
//! by a newer paste the core went on with, or found worthless. That zeroing covers this
//! copy only: the IPC buffer the paste arrived in, the Cloudflare client's copy and the
//! serialized `token.json` buffer are ordinary memory, freed without zeroing.

use std::collections::BTreeMap;
use std::time::Instant;

use shroom_core::event::{SecretField, Shape};
use shroom_core::world::SecretId;

/// Secret text that zeroes its bytes when dropped and never prints itself — plus, once
/// validated, whether it offered a choice of accounts (`token.json` keeps that).
pub struct Secret(String, Option<bool>);

impl Secret {
    pub fn text(&self) -> &str {
        &self.0
    }

    /// Whether the validated paste reaches more than one usable account.
    pub fn several(&self) -> bool {
        self.1.unwrap_or(false)
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        // SAFETY: zero bytes are valid UTF-8; the String is never read after this.
        let bytes = unsafe { self.0.as_mut_vec() };
        for b in bytes.iter_mut() {
            unsafe { std::ptr::write_volatile(b, 0) };
        }
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<secret>")
    }
}

/// At most this many pastes are held at once (a burst of pastes the core never went on
/// with can't pile up).
const CAP: usize = 4;

/// What a held join link is (Region 6): one this Mac was handed — pasted, opened by the
/// deep link, the installer's file, a `join.json` replayed — or one it made (Invite, Add
/// another Mac), shown once under its row for Copy link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    Invite,
    Born,
}

#[derive(Default)]
pub struct Stash {
    next: u32,
    held: BTreeMap<SecretId, Secret>,
    /// Join links, kept apart from tokens: a burst of token pastes never evicts the
    /// invite this Mac holds, nor the link Copy link copies — with when each was last
    /// handed in (a `ForgetInvite` forgets only what came before it was asked).
    links: BTreeMap<SecretId, (LinkKind, Secret, Instant)>,
}

impl Stash {
    /// Takes a paste: classifies its shape and keeps its text only when something will
    /// use it, pasted where a paste lands now (`lands`: the Scene shows that paste field,
    /// so the core takes it) — a token, or a join link (kept normalized, `https://<host>/
    /// join/<code>`, for the member lane's peek and join). Earlier pastes stay until the
    /// core moves on from them (`superseded_by`, `forget_tokens`): a paste the core drops
    /// never evicts the one it is validating or saving.
    pub fn put(&mut self, field: SecretField, text: String, lands: bool) -> (SecretId, Shape) {
        let text = Secret(text, None);
        let shape = classify(field, text.text());
        self.next += 1;
        let id = SecretId(self.next);
        if lands && shape == Shape::Token {
            while self.held.len() >= CAP {
                self.held.pop_first();
            }
            self.held.insert(id, text);
        } else if lands && shape == Shape::JoinLink {
            self.insert_link(id, LinkKind::Invite, normalized(text.text()));
        }
        (id, shape)
    }

    /// Keeps a join link that arrived by another door (the deep link, the installer's
    /// file, `join.json` at launch) or that this Mac made (Invite): the core gets the handle.
    /// A link already held as an invite keeps its handle.
    pub fn keep_link(&mut self, kind: LinkKind, text: String) -> SecretId {
        let text = normalized(&text);
        if let Some((id, (_, _, at))) = self.links.iter_mut().find(|(_, (k, s, _))| *k == kind && s.text() == text) {
            // Handed in again: it's as new as this arrival.
            *at = Instant::now();
            return *id;
        }
        self.next += 1;
        let id = SecretId(self.next);
        self.insert_link(id, kind, text);
        id
    }

    fn insert_link(&mut self, id: SecretId, kind: LinkKind, text: String) {
        while self.links.values().filter(|(k, _, _)| *k == kind).count() >= CAP {
            let Some(oldest) = self.links.iter().find(|(_, (k, _, _))| *k == kind).map(|(id, _)| *id) else { break };
            self.links.remove(&oldest);
        }
        self.links.insert(id, (kind, Secret(text, None), Instant::now()));
    }

    /// A held join link's text (the member lane and Copy link only — never a log).
    pub fn link(&self, id: SecretId) -> Option<&str> {
        self.links.get(&id).map(|(_, s, _)| s.text())
    }

    /// Whether this link is held as an invite (the installer's file seen again).
    pub fn holds_invite(&self, text: &str) -> bool {
        let text = normalized(text);
        self.links.values().any(|(k, s, _)| *k == LinkKind::Invite && s.text() == text)
    }

    /// The invite is behind us (`MemberCmd::ForgetInvite`, asked at `asked`): every link
    /// this Mac was handed before then goes — one handed in since is the next invite, and
    /// stays; links it made stay for their Copy link until the window closes. How many went.
    pub fn forget_invites_before(&mut self, asked: Instant) -> usize {
        let before = self.links.len();
        self.links.retain(|_, (k, _, at)| *k != LinkKind::Invite || *at > asked);
        before - self.links.len()
    }

    pub fn get(&self, id: SecretId) -> Option<&Secret> {
        self.held.get(&id)
    }

    /// Notes what validation found: whether the paste offered a choice of accounts.
    pub fn validated(&mut self, id: SecretId, several: bool) {
        if let Some(s) = self.held.get_mut(&id) {
            s.1 = Some(several);
        }
    }

    /// The core went on with `id`: every older paste is behind it (the core holds one
    /// paste at a time, the newest it took).
    pub fn superseded_by(&mut self, id: SecretId) {
        self.held.retain(|k, _| *k >= id);
    }

    /// Forgets one paste (its bytes are zeroed as it drops).
    pub fn forget(&mut self, id: SecretId) {
        self.held.remove(&id);
    }

    /// Forgets every pasted token (the token is saved; no token pasted before it is
    /// wanted). Join links stay: an invite this Mac holds isn't behind it.
    pub fn forget_tokens(&mut self) {
        self.held.clear();
    }

    /// Forgets everything — tokens and join links (Disconnect, Leave, a switch).
    pub fn forget_all(&mut self) {
        self.held.clear();
        self.links.clear();
    }

    #[cfg_attr(not(feature = "dev-harness"), allow(dead_code))]
    pub fn is_empty(&self) -> bool {
        self.held.is_empty() && self.links.is_empty()
    }

    /// No pasted token is held (join links aside).
    #[cfg(test)]
    pub fn no_tokens(&self) -> bool {
        self.held.is_empty()
    }
}

/// A join link in its one stored form, `https://<host>/join/<code>`: the three forms
/// (bare, `https://`, `shroom://`, §6.2) normalize to it; anything the parser can't read
/// is kept trimmed as it came (the lane answers it as a link that doesn't work).
pub fn normalized(text: &str) -> String {
    match shroom_cloud::member::parse_link(text) {
        Some((host, code)) => format!("https://{host}/join/{code}"),
        None => text.trim().to_string(),
    }
}

/// What a paste is, before any network. The token field takes tokens; the invite door
/// takes a shroom join link, and a Cloudflare token there is the wizard's paste (N30).
pub fn classify(field: SecretField, text: &str) -> Shape {
    use shroom_cloud::sweep::{Shape as Cf, shape};
    let token = !matches!(shape(text), Cf::Garbled);
    match field {
        SecretField::Token if token => Shape::Token,
        SecretField::Token => Shape::Garbled,
        SecretField::Invite if join_link(text) => Shape::JoinLink,
        SecretField::Invite if token => Shape::Token,
        SecretField::Invite => Shape::Other,
    }
}

/// `https://<host>/join/<code>` — the one shape every invite link has (N29) — or either of
/// its other two forms, bare `<host>/join/<code>` and `shroom://join/<host>/<code>` (§6.2).
pub fn join_link(text: &str) -> bool {
    if shroom_cloud::member::parse_link(text).is_some() {
        return true;
    }
    let t = text.trim();
    let Some(rest) = t.strip_prefix("https://") else { return false };
    let Some((host, path)) = rest.split_once('/') else { return false };
    let Some(code) = path.strip_prefix("join/") else { return false };
    let code = code.strip_suffix('/').unwrap_or(code);
    host.contains('.')
        && !host.starts_with('.')
        && host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
        && !code.is_empty()
        && code.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: &str = "cfat_0123456789abcdefghijklmnopqrstuvwxyzABCD";

    #[test]
    fn shapes_by_field() {
        assert_eq!(classify(SecretField::Token, TOKEN), Shape::Token);
        assert_eq!(classify(SecretField::Token, "  abc "), Shape::Garbled);
        assert_eq!(classify(SecretField::Token, "https://v.x.workers.dev/join/abc"), Shape::Garbled);
        assert_eq!(classify(SecretField::Invite, "https://v.myshroom.workers.dev/join/Ab3_x-9"), Shape::JoinLink);
        assert_eq!(classify(SecretField::Invite, " https://v.myshroom.workers.dev/join/Ab3/\n"), Shape::JoinLink);
        assert_eq!(classify(SecretField::Invite, TOKEN), Shape::Token);
        for miss in ["", "hello", "http://v.x.workers.dev/join/abc", "https://v.x.workers.dev/abc", "https://v.x.workers.dev/join/", "https:///join/abc"] {
            assert_eq!(classify(SecretField::Invite, miss), Shape::Other, "{miss:?}");
        }
    }

    #[test]
    fn useless_or_unlanded_text_is_never_kept() {
        let mut s = Stash::default();
        let (a, shape) = s.put(SecretField::Token, "nope".into(), true);
        assert_eq!(shape, Shape::Garbled);
        assert!(s.get(a).is_none() && s.is_empty());
        // A paste where no paste field shows: the core drops it, so the text isn't kept.
        let (b, shape) = s.put(SecretField::Token, TOKEN.into(), false);
        assert_eq!(shape, Shape::Token);
        assert!(s.get(b).is_none() && s.is_empty());
        // A join link where no paste field shows: classified, never kept.
        let (c, shape) = s.put(SecretField::Invite, "https://v.myshroom.workers.dev/join/AbC".into(), false);
        assert_eq!(shape, Shape::JoinLink);
        assert!(s.get(c).is_none() && s.link(c).is_none() && s.is_empty());
    }

    #[test]
    fn a_join_link_is_kept_apart_normalized_and_forgotten_with_its_invite() {
        let mut s = Stash::default();
        // Region 6: a pasted link lands in the invite door — kept, in its one stored form.
        let (a, shape) = s.put(SecretField::Invite, " shroom://join/V.MyShroom.workers.dev/AbCdEfGhIjKlMnOp\n".into(), true);
        assert_eq!(shape, Shape::JoinLink);
        assert_eq!(s.link(a), Some("https://v.myshroom.workers.dev/join/AbCdEfGhIjKlMnOp"));
        assert!(s.get(a).is_none(), "a link is never a token");
        assert_eq!(classify(SecretField::Invite, "v.myshroom.workers.dev/join/AbCdEfGhIjKlMnOp"), Shape::JoinLink, "the bare form too");
        // The same invite arriving again keeps its handle; a born link is its own.
        assert_eq!(s.keep_link(LinkKind::Invite, "https://v.myshroom.workers.dev/join/AbCdEfGhIjKlMnOp".into()), a);
        let born = s.keep_link(LinkKind::Born, "https://v.myshroom.workers.dev/join/ZyXwVuTsRqPoNmLk".into());
        assert!(s.holds_invite("v.myshroom.workers.dev/join/AbCdEfGhIjKlMnOp"));
        // A burst of token pastes never evicts a link; saving the token keeps them.
        for _ in 0..10 {
            s.put(SecretField::Token, TOKEN.into(), true);
        }
        s.forget_tokens();
        assert!(s.no_tokens() && s.link(a).is_some() && s.link(born).is_some());
        // The invite is behind us: the handed link goes, the born one stays for Copy link.
        assert_eq!(s.forget_invites_before(Instant::now()), 1);
        assert!(s.link(a).is_none() && s.link(born).is_some());
        s.forget_all();
        assert!(s.is_empty());
    }

    #[test]
    fn forgetting_an_invite_spares_one_handed_in_after_it_was_asked() {
        let mut s = Stash::default();
        let x = s.keep_link(LinkKind::Invite, "https://v.alpha.workers.dev/join/AbCdEfGhIjKlMnOp".into());
        std::thread::sleep(std::time::Duration::from_millis(2));
        let asked = Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(2));
        // Y is pasted while X's ForgetInvite waits behind a slow peek on the lane.
        let (y, _) = s.put(SecretField::Invite, "https://v.beta.workers.dev/join/ZyXwVuTsRqPoNmLk".into(), true);
        assert_eq!(s.forget_invites_before(asked), 1);
        assert!(s.link(x).is_none() && s.link(y).is_some(), "Y is the next invite, not the one closed");
        // X handed in again after the ask is as new as its arrival.
        let x2 = s.keep_link(LinkKind::Invite, "https://v.alpha.workers.dev/join/AbCdEfGhIjKlMnOp".into());
        assert_eq!(s.forget_invites_before(asked), 0);
        assert!(s.link(x2).is_some());
    }

    #[test]
    fn a_held_paste_survives_until_the_core_moves_on() {
        let mut s = Stash::default();
        let (a, _) = s.put(SecretField::Token, TOKEN.into(), true);
        s.validated(a, true);
        // A later paste (one the core may yet drop) doesn't evict the one being saved.
        let (b, _) = s.put(SecretField::Token, TOKEN.into(), true);
        assert!(s.get(a).is_some_and(Secret::several) && s.get(b).is_some_and(|x| !x.several()));
        // The core validates the newer one: the older is behind it.
        s.superseded_by(b);
        assert!(s.get(a).is_none() && s.get(b).is_some());
        s.forget_all();
        assert!(s.is_empty());
        // Never more than CAP at once.
        for _ in 0..10 {
            s.put(SecretField::Token, TOKEN.into(), true);
        }
        assert_eq!(s.held.len(), CAP);
    }

    #[test]
    fn a_secret_never_prints() {
        let s = Secret(TOKEN.into(), None);
        assert_eq!(format!("{s:?}"), "<secret>");
    }
}
