//! The space, through its own page under this Mac's device key — the door every Mac but
//! the token Mac's Library already uses (`app/src-tauri/src/runtime/library.rs`,
//! WORKER-INTERFACE §4.10, §6.8). The list is `POST /library`; a rename or delete runs
//! under a `manage` credential minted for that one video (15 minutes, memory only), and a
//! delete is sealed with `POST /gone` (DECISIONS B125) — the same three steps the Library's
//! Delete… takes.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};
use shroom_cloud::device::DeviceKey;
use shroom_cloud::member::{Answer, CleanupItem, LibraryRequest, MintNext, Purpose, Scope, Space};
use shroom_core::world::{Role, SpaceIdentity};
use shroom_storage::s3::{Bucket, Client};
use shroom_storage::sigv4::Credentials;
use shroom_upload::library::{self, Row};
use shroom_upload::meta::State;

use crate::out::Fail;
use crate::say;

pub struct Door {
    pub identity: SpaceIdentity,
    key: DeviceKey,
    page: Space,
    /// A debug build's local stand-in for the space (`worker/dev/serve.mjs`): its pages at
    /// `<base>/<host>/…`, its bucket at `<base>/_s3/<host>/…`. `None`: the real space.
    base: Option<String>,
}

/// The page's suggestions: since when plays are counted, and every video it looked at.
pub type Suggestions = (Option<String>, Vec<CleanupItem>);

/// One video, as the CLI shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct Video {
    pub row: Row,
    pub link: String,
}

impl Video {
    pub fn json(&self) -> Value {
        json!({
            "id": self.row.id,
            "title": self.row.title,
            "state": match self.row.state { State::Live => "live", State::Uploading => "uploading", State::Deleted => "deleted" },
            "created_at": self.row.created_at,
            "duration_sec": self.row.duration_sec,
            "owner": self.row.owner,
            "link": self.link,
        })
    }
}

impl Door {
    pub fn open(identity: SpaceIdentity, key: DeviceKey, base: Option<String>) -> Door {
        let page = match &base {
            Some(b) => Space::at(&identity.host, &format!("{}/{}", b.trim_end_matches('/'), identity.host)),
            None => Space::new(&identity.host),
        };
        Door { identity, key, page, base }
    }

    pub fn link(&self, id: &str) -> String {
        format!("https://{}/{id}", self.identity.host)
    }

    /// Every video this Mac may see — a member's own, everyone's for the owner's Macs —
    /// newest first. `cached` (the Library's) saves reading records that haven't changed.
    pub fn list(&self, cached: &BTreeMap<String, (String, Row)>) -> Result<Vec<Video>, Fail> {
        const SLICE: usize = 1000;
        let scope = if self.identity.role == Role::Member { Scope::Mine } else { Scope::Everyone };
        let ids: Vec<String> = cached.keys().cloned().collect();
        let mut rows: BTreeMap<String, Row> = BTreeMap::new();
        let mut seen = BTreeSet::new();
        let mut after: Option<String> = None;
        for _ in 0..10_000 {
            let start = after.as_ref().map_or(0, |a| ids.partition_point(|x| x <= a));
            let slice = &ids[start..(start + SLICE).min(ids.len())];
            let known: BTreeMap<String, String> = slice.iter().map(|id| (id.clone(), cached[id].0.clone())).collect();
            let known_through = (start + slice.len() < ids.len()).then(|| slice.last().cloned()).flatten();
            let p = answer(self.page.library(&self.key, &LibraryRequest { scope, known, known_through, after: after.clone() }))?;
            for item in p.items {
                seen.insert(item.id.clone());
                match item.meta.and_then(shroom_upload::meta::listed) {
                    Some(m) => {
                        rows.insert(item.id.clone(), Row {
                            id: item.id,
                            title: m.title,
                            state: m.state,
                            created_at: m.created_at,
                            duration_sec: m.duration_sec,
                            owner: Some(item.owner),
                            poster: m.media.and_then(|x| x.poster),
                        });
                    }
                    // Unchanged since the cache: the cached row, with the ledger's owner.
                    None => {
                        if let Some((_, r)) = cached.get(&item.id) {
                            rows.insert(item.id.clone(), Row { owner: Some(item.owner), ..r.clone() });
                        }
                    }
                }
            }
            match p.next {
                Some(n) if after.as_deref() != Some(n.as_str()) => after = Some(n),
                _ => break,
            }
        }
        let mut videos: Vec<Video> =
            rows.into_values().filter(|r| seen.contains(&r.id) && r.state != State::Deleted).map(|row| Video { link: self.link(&row.id), row }).collect();
        videos.sort_by(|a, b| b.row.created_at.cmp(&a.row.created_at));
        Ok(videos)
    }

    /// A `manage` credential's bucket client for one video (§6.8).
    fn manage(&self, id: &str) -> Result<Client, Fail> {
        let m = match MintNext::of(self.page.mint(&self.key, id, Purpose::Manage)) {
            MintNext::Minted(m) => m,
            MintNext::Revoked => return Err(Fail::Refused(say::not_in_the_space())),
            MintNext::NotYours => return Err(Fail::Refused(say::someone_elses())),
            MintNext::Deleted | MintNext::Absent => return Err(Fail::NotFound(say::already_gone())),
            MintNext::Backoff => return Err(Fail::Offline(say::busy_or_unreachable())),
            _ => return Err(Fail::Other(say::unexpected_answer())),
        };
        let endpoint = match &self.base {
            Some(b) => format!("{}/_s3/{}", b.trim_end_matches('/'), self.identity.host),
            None => m.endpoint.clone(),
        };
        Ok(Client::new(Bucket {
            endpoint,
            region: "auto".into(),
            name: m.bucket.clone(),
            credentials: Credentials { access_key_id: m.access_key_id.clone(), secret_access_key: m.secret_access_key.clone(), session_token: Some(m.session_token.clone()) },
        }))
    }

    /// Renames a video — live on its page at the next load.
    pub fn rename(&self, id: &str, title: &str) -> Result<String, Fail> {
        let s3 = self.manage(id)?;
        let meta = library::rename(&s3, id, title).map_err(upload_fail)?;
        Ok(meta.title)
    }

    /// Deletes one video for good: the record becomes a tombstone (its link says
    /// “removed” from then on), then its bytes go, then the deletion is sealed (B125).
    /// Running it again after an interruption finishes the job.
    pub fn delete(&self, id: &str) -> Result<(), Fail> {
        let s3 = self.manage(id)?;
        library::delete(&s3, id).map_err(upload_fail)?;
        // A page from before B125 has no `/gone`: the delete stands without the seal, as
        // the Library's does.
        let _ = self.page.gone(&self.key, id);
        Ok(())
    }

    /// A member invite for `name` (owner-class, §4.12): the code is made here and only
    /// its hash is sent; the link is `https://<host>/join/<code>`, good for 7 days, once.
    pub fn invite(&self, name: &str) -> Result<(shroom_cloud::member::InviteRow, String), Fail> {
        let code = shroom_cloud::device::new_join_code();
        let invite = shroom_cloud::member::NewInvite {
            id: shroom_cloud::device::new_invite_id(),
            code_hash: shroom_cloud::device::hash(&code),
            kind: "member".into(),
            label: Some(name.to_string()),
            inviter: self.identity.owner_name.chars().take(40).collect(),
        };
        let row = answer(self.page.invite(&self.key, &invite))?;
        Ok((row, format!("https://{}/join/{code}", self.identity.host)))
    }

    /// A new link for a pending invite (§4.12, renew): the same invite, a new code — the
    /// old link stops working.
    pub fn renew(&self, id: &str) -> Result<(shroom_cloud::member::InviteRow, String), Fail> {
        let code = shroom_cloud::device::new_join_code();
        let row = answer(self.page.renew(&self.key, id, &shroom_cloud::device::hash(&code)))?;
        Ok((row, format!("https://{}/join/{code}", self.identity.host)))
    }

    /// Cancels a pending invite (§4.13): its link stops working.
    pub fn cancel_invite(&self, id: &str) -> Result<(), Fail> {
        answer(self.page.cancel_invite(&self.key, id))
    }

    /// Removes a member and every Mac of theirs (§4.14). Videos they made stay.
    pub fn remove_member(&self, id: &str) -> Result<(), Fail> {
        answer(self.page.remove_member(&self.key, id))
    }

    /// The page's own suggestions of what to delete, every page of them (`POST cleanup`,
    /// paged by `after`) — only this Mac's person's videos for a member, or with `mine`;
    /// `None` when the page doesn't make them (404: an older page).
    pub fn suggestions(&self, mine: bool) -> Result<Option<Suggestions>, Fail> {
        let scope = if mine || self.identity.role == Role::Member { Scope::Mine } else { Scope::Everyone };
        let mut items = Vec::new();
        let mut since = None;
        let mut after: Option<String> = None;
        for _ in 0..10_000 {
            let page = match self.page.cleanup(&self.key, scope, after.as_deref()) {
                Answer::Refused(r) if r.status == 404 => return Ok(None),
                Answer::NotShroom { status: 404 } => return Ok(None),
                a => answer(a)?,
            };
            since = page.counting_since;
            items.extend(page.items);
            match page.next {
                Some(n) if after.as_deref() != Some(n.as_str()) => after = Some(n),
                _ => break,
            }
        }
        Ok(Some((since, items)))
    }

    /// The space's people (owner-class only, §4.11).
    pub fn members(&self) -> Result<shroom_cloud::member::Members, Fail> {
        answer(self.page.members(&self.key))
    }
}

fn answer<T>(a: Answer<T>) -> Result<T, Fail> {
    match a {
        Answer::Ok(t) => Ok(t),
        Answer::Unanswered(_) => Err(Fail::Offline(say::unreachable())),
        Answer::Refused(r) if r.status == 503 || r.is_transient() => Err(Fail::Offline(say::busy_space())),
        Answer::Refused(r) if r.status == 403 && r.is_revoked() => Err(Fail::Refused(say::not_in_the_space())),
        Answer::Refused(r) if r.status == 401 => Err(Fail::NotSetUp(say::unknown_mac())),
        Answer::Refused(r) if r.status == 403 => Err(Fail::Refused(say::owner_only())),
        Answer::Refused(r) if r.status == 404 => Err(Fail::NotFound(say::no_such_invite_or_member())),
        Answer::Refused(r) if r.class == shroom_cloud::member::Class::Full => Err(Fail::Refused(say::too_many_invites())),
        Answer::Refused(r) if r.status == 409 => Err(Fail::Refused(say::cant_change_now())),
        Answer::Refused(r) if r.status == 400 => Err(Fail::Usage(say::name_too_long())),
        Answer::Refused(_) => Err(Fail::Other(say::space_said_no())),
        Answer::NotShroom { .. } => Err(Fail::Offline(say::not_answering_as_shroom())),
    }
}

fn upload_fail(e: shroom_upload::upload::UploadError) -> Fail {
    use shroom_storage::s3::S3Error;
    use shroom_upload::upload::UploadError as U;
    match e {
        U::S3(S3Error::Transport(_)) => Fail::Offline(say::unreachable()),
        U::Tombstoned => Fail::NotFound(say::already_gone()),
        U::S3(S3Error::Status { status: 401 | 403, .. }) => Fail::Refused(say::space_said_no()),
        // Never the storage's own words (law 15): the step that stopped, and that a re-run
        // finishes it.
        _ => Fail::Other(say::didnt_finish()),
    }
}
