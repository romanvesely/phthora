//! Undo/redo for the effect stack, based on snapshots.

use crate::chain::EffectInstance;

const MAX_STEPS: usize = 50;

/// Snapshot history. The caller offers the current stack whenever an edit may be complete
/// (no drag in progress); a snapshot is only recorded when the stack actually changed.
#[derive(Default)]
pub struct History {
    past: Vec<Vec<EffectInstance>>,
    future: Vec<Vec<EffectInstance>>,
    current: Option<(String, Vec<EffectInstance>)>,
}

/// Content that counts as an edit: everything in the preset except UI state (expanded).
fn fingerprint(effects: &[EffectInstance]) -> String {
    let mut p = crate::preset::to_preset(effects);
    for e in &mut p.effects {
        e.expanded = false;
    }
    let uids: Vec<u64> = effects.iter().map(|e| e.uid).collect();
    format!("{uids:?}{}", serde_json::to_string(&p).unwrap_or_default())
}

impl History {
    /// Record `effects` if they differ from the last recorded state.
    pub fn observe(&mut self, effects: &[EffectInstance]) {
        let fp = fingerprint(effects);
        match &self.current {
            Some((cur, _)) if *cur == fp => {}
            Some(_) => {
                let (_, prev) = self.current.replace((fp, effects.to_vec())).unwrap();
                self.past.push(prev);
                if self.past.len() > MAX_STEPS {
                    self.past.remove(0);
                }
                self.future.clear();
            }
            None => self.current = Some((fp, effects.to_vec())),
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    /// Previous state; keeps the expanded flags of the current stack where effects match.
    pub fn undo(&mut self, effects: &[EffectInstance]) -> Option<Vec<EffectInstance>> {
        self.observe(effects);
        let prev = self.past.pop()?;
        let (_, cur) = self.current.replace((fingerprint(&prev), prev.clone())).unwrap();
        self.future.push(cur);
        Some(keep_expanded(prev, effects))
    }

    pub fn redo(&mut self, effects: &[EffectInstance]) -> Option<Vec<EffectInstance>> {
        self.observe(effects);
        let next = self.future.pop()?;
        let (_, cur) = self.current.replace((fingerprint(&next), next.clone())).unwrap();
        self.past.push(cur);
        Some(keep_expanded(next, effects))
    }
}

fn keep_expanded(mut v: Vec<EffectInstance>, now: &[EffectInstance]) -> Vec<EffectInstance> {
    for e in &mut v {
        if let Some(n) = now.iter().find(|n| n.uid == e.uid) {
            e.expanded = n.expanded;
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
        h.observe(&c.effects);
        let uid = c.add("pixelate").unwrap();
        h.observe(&c.effects);
        c.get_mut(uid).unwrap().set("amount", Value::Float(0.9));
        h.observe(&c.effects);
        // expanding is not an edit
        c.get_mut(uid).unwrap().expanded = false;
        h.observe(&c.effects);
        assert!(h.can_undo());
        c.effects = h.undo(&c.effects).unwrap();
        assert_eq!(c.effects.len(), 1);
        assert_ne!(c.effects[0].f("amount"), 0.9);
        assert!(!c.effects[0].expanded, "UI state is kept");
        c.effects = h.undo(&c.effects).unwrap();
        assert!(c.effects.is_empty());
        assert!(h.undo(&c.effects).is_none());
        c.effects = h.redo(&c.effects).unwrap();
        c.effects = h.redo(&c.effects).unwrap();
        assert_eq!(c.effects[0].f("amount"), 0.9);
        // a new edit drops the redo branch
        c.effects = h.undo(&c.effects).unwrap();
        c.add("bulge");
        h.observe(&c.effects);
        assert!(!h.can_redo());
    }

    #[test]
    fn bounded() {
        let mut h = History::default();
        let mut c = Chain::default();
        h.observe(&c.effects);
        for _ in 0..80 {
            c.add("pixelate");
            h.observe(&c.effects);
        }
        let mut n = 0;
        while let Some(v) = h.undo(&c.effects) {
            c.effects = v;
            n += 1;
        }
        assert_eq!(n, MAX_STEPS);
    }
}
