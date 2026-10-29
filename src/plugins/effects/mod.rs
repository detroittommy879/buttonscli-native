#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod analog_static;
pub mod row_banding;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod shader_background;
pub mod simple_noise;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod shader_expression;

/// Drops the least recently used entry once `map` holds `limit` entries.
///
/// Stamps must come from one counter shared by every window. egui's pass
/// number is per viewport, so a busy main window made detached Settings
/// entries look stale: its previews evicted each other mid-frame and drew the
/// preset pipeline instead of their own code.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn make_room<K: Clone + Eq + std::hash::Hash, V>(
    map: &mut std::collections::HashMap<K, V>,
    limit: usize,
    last_used: impl Fn(&V) -> u64,
) {
    while map.len() >= limit.max(1) {
        let Some(oldest) = map
            .iter()
            .min_by_key(|(_, value)| last_used(value))
            .map(|(key, _)| key.clone())
        else {
            return;
        };
        map.remove(&oldest);
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use std::collections::HashMap;

    /// Two windows drawing through one cache: the main window keeps many
    /// terminals busy, the Settings window draws a handful of previews.
    #[test]
    fn shared_stamps_keep_every_entry_drawn_this_frame() {
        const LIMIT: usize = 8;
        let mut cache: HashMap<String, u64> = HashMap::new();
        let mut uses = 0_u64;
        let mut draw = |cache: &mut HashMap<String, u64>, key: String| {
            uses += 1;
            if !cache.contains_key(&key) {
                super::make_room(cache, LIMIT, |stamp| *stamp);
            }
            cache.insert(key, uses);
        };
        for round in 0..20 {
            for terminal in 0..6 {
                draw(&mut cache, format!("main-{round}-{terminal}"));
            }
            let previews: Vec<_> = (0..5).map(|n| format!("settings-{round}-{n}")).collect();
            for preview in &previews {
                draw(&mut cache, preview.clone());
            }
            for preview in &previews {
                assert!(
                    cache.contains_key(preview),
                    "{preview} survives its own frame"
                );
            }
            assert!(cache.len() <= LIMIT);
        }
    }
}
