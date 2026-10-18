//! Lining the retake (words after the cue) up against what was said before the cue.

use crate::text::{is_filler, likeness};

const MISMATCH: f64 = -0.6;
const GAP: f64 = -0.8;

/// Where the retake restarts: `pre[r..]` is the flubbed take, `post[s..]` the retake.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Restart {
    pub r: usize,
    pub s: usize,
    /// Words of the flubbed take the retake says again.
    pub matched: usize,
    pub score: f64,
}

/// Best alignment of `post[s..]` against `pre[r..]`, both starts pinned and both ends
/// free: (score, words matched).
fn pinned(pre: &[String], post: &[String]) -> (f64, usize) {
    let (n, m) = (pre.len(), post.len());
    // dp[i][j]: best score aligning pre[..i] with post[..j], and its matches.
    let mut dp = vec![vec![(f64::NEG_INFINITY, 0usize); m + 1]; n + 1];
    dp[0][0] = (0.0, 0);
    let mut best = (f64::NEG_INFINITY, 0);
    for i in 0..=n {
        for j in 0..=m {
            let (here, hits) = dp[i][j];
            if here == f64::NEG_INFINITY {
                continue;
            }
            if (i, j) != (0, 0) && here > best.0 {
                best = (here, hits);
            }
            if i < n && j < m {
                let like = likeness(&pre[i], &post[j]);
                let step = if like > 0.0 {
                    (here + like, hits + 1)
                } else {
                    (here + MISMATCH, hits)
                };
                if (i, j) == (0, 0) && like == 0.0 {
                    continue; // the retake opens with the flubbed take's first word
                }
                if step.0 > dp[i + 1][j + 1].0 {
                    dp[i + 1][j + 1] = step;
                }
            }
            if (i, j) == (0, 0) {
                continue;
            }
            if i < n && here + GAP > dp[i + 1][j].0 {
                dp[i + 1][j] = (here + GAP, hits);
            }
            if j < m && here + GAP > dp[i][j + 1].0 {
                dp[i][j + 1] = (here + GAP, hits);
            }
        }
    }
    best
}

/// The restart that best explains the retake. `pre` is what came before the cue (oldest
/// first), `post` what came after it. Among equal scores the latest restart wins: a
/// speaker retakes the line they just fluffed, not one from a minute ago.
pub fn find_restart(pre: &[String], post: &[String]) -> Option<Restart> {
    let leading_fillers = post.iter().take_while(|w| is_filler(w)).count().min(3);
    let mut best: Option<Restart> = None;
    for s in 0..=leading_fillers.min(post.len().saturating_sub(1)) {
        for r in 0..pre.len() {
            if is_filler(&pre[r]) {
                continue;
            }
            let (score, matched) = pinned(&pre[r..], &post[s..]);
            if matched == 0 {
                continue;
            }
            let better = match best {
                None => true,
                Some(b) => score > b.score + 1e-9 || ((score - b.score).abs() <= 1e-9 && r > b.r),
            };
            if better {
                best = Some(Restart {
                    r,
                    s,
                    matched,
                    score,
                });
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn the_example() {
        let r = find_restart(&w("hello that is shroom"), &w("hello this is shroom")).unwrap();
        assert_eq!((r.r, r.s, r.matched), (0, 0, 3));
    }

    #[test]
    fn restart_mid_paragraph() {
        let pre = w("you click the icon in the menu bar then you pick the the window um");
        let post = w("so then you pick the window you want");
        let r = find_restart(&pre, &post).unwrap();
        assert_eq!((pre[r.r].as_str(), r.s), ("then", 1));
    }

    #[test]
    fn latest_of_equals() {
        let pre = w("open settings open settings and click");
        let r = find_restart(&pre, &w("open settings and choose")).unwrap();
        assert_eq!(r.r, 2);
    }
}
