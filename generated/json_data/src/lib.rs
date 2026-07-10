pub const SKIPPABLE_CUTSCENES: &str = include_str!(concat!(
    env!("OUT_DIR"),
    "/skippable_cutscenes.jsonc.min.json"
));
pub const SKIPPABLE_CUTSCENES_PAL: &str = include_str!(concat!(
    env!("OUT_DIR"),
    "/skippable_cutscenes_pal.jsonc.min.json"
));
pub const SKIPPABLE_CUTSCENES_COMPETITIVE: &str = include_str!(concat!(
    env!("OUT_DIR"),
    "/skippable_cutscenes_competitive.jsonc.min.json"
));
pub const QOL_GENERAL: &str = include_str!(concat!(env!("OUT_DIR"), "/qol-general.jsonc.min.json"));
pub const QOL_MUSIC: &str = include_str!(concat!(env!("OUT_DIR"), "/qol-music.jsonc.min.json"));
pub const QOL_TUTORIAL: &str =
    include_str!(concat!(env!("OUT_DIR"), "/qol-tutorial.jsonc.min.json"));
pub const GAME_BREAKING: &str =
    include_str!(concat!(env!("OUT_DIR"), "/game_breaking.jsonc.min.json"));
