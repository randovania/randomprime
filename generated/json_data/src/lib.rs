pub const SKIPPABLE_CUTSCENES: &'static str = include_str!(concat!(
    env!("OUT_DIR"),
    "/skippable_cutscenes.jsonc.min.json"
));
pub const SKIPPABLE_CUTSCENES_PAL: &'static str = include_str!(concat!(
    env!("OUT_DIR"),
    "/skippable_cutscenes_pal.jsonc.min.json"
));
pub const SKIPPABLE_CUTSCENES_COMPETITIVE: &'static str = include_str!(concat!(
    env!("OUT_DIR"),
    "/skippable_cutscenes_competitive.jsonc.min.json"
));
pub const QOL: &'static str = include_str!(concat!(env!("OUT_DIR"), "/qol.jsonc.min.json"));
