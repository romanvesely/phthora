//! Undo/redo for the effect stack, based on snapshots.

use crate::chain::Chain;

const MAX_STEPS: usize = 50;

/// Snapshot history. The caller offers the current stack whenever an edit may be complete
/// (no drag in progress); a snapshot is only recorded when the stack actually changed.
#[derive(Default)]
pub struct History {
    past: Vec<Chain>,
    future: Vec<Chain>,
    current: Option<(String, Chain)>,
}

/// Content that counts as an edit: everything in the preset except UI state (expanded).
fn fingerprint(chain: &Chain) -> String {
    let mut p = crate::preset::to_preset(chain);
    for e in &mut p.effects {
        e.expanded = false;
    }
    for g in &mut p.groups {
        g.expanded = false;
    }
    let uids: Vec<u64> = chain.effects.iter().map(|e| e.uid).chain(chain.groups.iter().map(|g| g.uid)).collect();
    format!("{uids:?}{}", serde_json::to_string(&p).unwrap_or_default())
}

impl History {
    /// Record the stack if it differs from the last recorded state.
    pub fn observe(&mut self, chain: &Chain) {
        let fp = fingerprint(chain);
        match &self.current {
            Some((cur, _)) if *cur == fp => {}
            Some(_) => {
                let (_, prev) = self.current.replace((fp, chain.clone())).unwrap();
                self.past.push(prev);
                if self.past.len() > MAX_STEPS {
                    self.past.remove(0);
                }
                self.future.clear();
            }
            None => self.current = Some((fp, chain.clone())),
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    /// Previous state; keeps the expanded flags of the current stack where effects match.
    pub fn undo(&mut self, chain: &Chain) -> Option<Chain> {
        self.observe(chain);
        let prev = self.past.pop()?;
        let (_, cur) = self.current.replace((fingerprint(&prev), prev.clone())).unwrap();
        self.future.push(cur);
        Some(keep_expanded(prev, chain))
    }

    pub fn redo(&mut self, chain: &Chain) -> Option<Chain> {
        self.observe(chain);
        let next = self.future.pop()?;
        let (_, cur) = self.current.replace((fingerprint(&next), next.clone())).unwrap();
        self.past.push(cur);
        Some(keep_expanded(next, chain))
    }
}

fn keep_expanded(mut v: Chain, now: &Chain) -> Chain {
    for e in &mut v.effects {
        if let Some(n) = now.effects.iter().find(|n| n.uid == e.uid) {
            e.expanded = n.expanded;
        }
    }
    for g in &mut v.groups {
        if let Some(n) = now.group(g.uid) {
            g.expanded = n.expanded;
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::Chain;
    use crate::params::Value;

    #[test]
    fn undo_redo() {
        let mut h = History::default();
        let mut c = Chain::default();
        h.observe(&c);
        let uid = c.add("pixelate").unwrap();
        h.observe(&c);
        c.get_mut(uid).unwrap().set("amount", Value::Float(0.9));
        h.observe(&c);
        // expanding is not an edit
        c.get_mut(uid).unwrap().expanded = false;
        h.observe(&c);
        assert!(h.can_undo());
        c = h.undo(&c).unwrap();
        assert_eq!(c.effects.len(), 1);
        assert_ne!(c.effects[0].f("amount"), 0.9);
        assert!(!c.effects[0].expanded, "UI state is kept");
        c = h.undo(&c).unwrap();
        assert!(c.effects.is_empty());
        assert!(h.undo(&c).is_none());
        c = h.redo(&c).unwrap();
        c = h.redo(&c).unwrap();
        assert_eq!(c.effects[0].f("amount"), 0.9);
        // a new edit drops the redo branch
        c = h.undo(&c).unwrap();
        c.add("bulge");
        h.observe(&c);
        assert!(!h.can_redo());
        // grouping is an edit, collapsing a group is not
        let g = c.make_group(uid).unwrap();
        h.observe(&c);
        c.group_mut(g).unwrap().expanded = false;
        h.observe(&c);
        c = h.undo(&c).unwrap();
        assert!(c.groups.is_empty() && c.effects[0].group.is_none());
    }

    #[test]
    fn bounded() {
        let mut h = History::default();
        let mut c = Chain::default();
        h.observe(&c);
        for _ in 0..80 {
            c.add("pixelate");
            h.observe(&c);
        }
        let mut n = 0;
        while let Some(v) = h.undo(&c) {
            c = v;
            n += 1;
        }
        assert_eq!(n, MAX_STEPS);
    }
}
