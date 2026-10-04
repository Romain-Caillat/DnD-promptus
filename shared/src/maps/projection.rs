//! The one place a map is cut down for a viewer (`MEMORY.md` §3).
//!
//! Players never receive a GM layer, nor anything placed on one (hidden
//! objects, secret doors, ambushers' starts), nor GM notes, the checks
//! that find objects, or the backdrop's generation prompt. A secret
//! door's cell is a wall in the grid, so removing the door leaves no gap.
//! Fog of war (unrevealed cells) is a session state and is cut later,
//! by `maps/reveal-fog-and-hidden`, on top of this.

use super::model::{Map, Viewer};

impl Map {
    /// The map as `viewer` may receive it.
    pub fn project(&self, viewer: Viewer) -> Map {
        if viewer == Viewer::Gm {
            return self.clone();
        }
        let shown = |layer: &str| self.visibility(layer).shown_to(viewer);
        let mut map = self.clone();
        map.gm_notes = None;
        map.layers.retain(|l| l.visibility.shown_to(viewer));
        map.doors.retain(|d| shown(&d.layer));
        map.props.retain(|p| shown(&p.layer));
        map.objects.retain(|o| shown(&o.layer));
        for o in &mut map.objects {
            o.check = None;
            o.notes = None;
        }
        map.lights.retain(|l| shown(&l.layer));
        map.exits.retain(|e| shown(&e.layer));
        map.labels.retain(|l| shown(&l.layer));
        map.starts.retain(|s| shown(&s.layer));
        if let Some(backdrop) = &mut map.backdrop {
            backdrop.prompt = None;
        }
        map
    }
}
