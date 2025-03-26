use std::{collections::HashMap, convert::TryInto, iter};

use reader_writer::{CStrConversionExtension, FourCC, Reader};
use resource_info_table::resource_info;
use structs::{res_id, ResId, SclyPropertyData};

use crate::{
    door_meta::DoorType,
    mlvl_wrapper,
    patch_config::{
        ActorKeyFrameConfig, ActorRotateConfig, BlockConfig, BombSlotConfig, CameraConfig,
        CameraFilterKeyframeConfig, CameraHintTriggerConfig, CameraWaypointConfig,
        ControllerActionConfig, CounterConfig, DamageType, FogConfig, GenericTexture,
        HudmemoConfig, LockOnPoint, NewCameraHintConfig, PlatformConfig, PlatformType,
        PlayerActorConfig, PlayerHintConfig, RelayConfig, SpawnPointConfig, SpecialFunctionConfig,
        StreamedAudioConfig, SwitchConfig, TimerConfig, TriggerConfig, WaterConfig, WaypointConfig,
        WorldLightFaderConfig,
    },
    patcher::PatcherState,
    patches::{string_to_cstr, WaterType},
    pickup_meta::PickupType,
};

macro_rules! add_edit_obj_helper {
    ($area:expr, $id:expr, $requested_layer_id:expr, $object_type:ident, $new_property_data:ident, $update_property_data:ident) => {
        let area = $area;
        let id = $id;
        let requested_layer_id = $requested_layer_id;
        let mrea_id = area.mlvl_area.mrea.to_u32().clone();

        // add more layers as needed
        if let Some(requested_layer_id) = requested_layer_id {
            while area.layer_flags.layer_count <= requested_layer_id {
                area.add_layer(b"New Layer\0".as_cstr());
            }
        }

        if let Some(id) = id {
            let scly = area.mrea().scly_section_mut();

            // try to find existing object
            let info = {
                let mut info = None;

                let layer_count = scly.layers.as_mut_vec().len();
                for _layer_id in 0..layer_count {
                    let layer = scly.layers
                        .iter()
                        .nth(_layer_id)
                        .unwrap();

                    let obj = layer.objects
                        .iter()
                        .find(|obj| obj.instance_id & 0x00FFFFFF == id & 0x00FFFFFF);

                    if let Some(obj) = obj {
                        if obj.property_data.object_type() != structs::$object_type::OBJECT_TYPE {
                            panic!("Failed to edit existing object 0x{:X} in room 0x{:X}: Unexpected object type 0x{:X} (expected 0x{:X})", id, mrea_id, obj.property_data.object_type(), structs::$object_type::OBJECT_TYPE);
                        }

                        info = Some((_layer_id as u32, obj.instance_id));
                        break;
                    }
                }

                info
            };

            if let Some(info) = info {
                let (layer_id, _) = info;

                // move and update
                if requested_layer_id.is_some() && requested_layer_id.unwrap() != layer_id {
                    let requested_layer_id = requested_layer_id.unwrap();

                    // clone existing object
                    let mut obj = scly.layers
                        .as_mut_vec()[layer_id as usize]
                        .objects
                        .as_mut_vec()
                        .iter_mut()
                        .find(|obj| obj.instance_id & 0x00FFFFFF == id & 0x00FFFFFF)
                        .unwrap()
                        .clone();

                    // modify it
                    $update_property_data!(obj);

                    // remove original
                    scly.layers
                        .as_mut_vec()[layer_id as usize]
                        .objects
                        .as_mut_vec()
                        .retain(|obj| obj.instance_id & 0x00FFFFFF != id & 0x00FFFFFF);

                    // re-add to target layer
                    scly.layers
                        .as_mut_vec()[requested_layer_id as usize]
                        .objects
                        .as_mut_vec()
                        .push(obj);

                    return Ok(());
                }

                // get mutable reference to existing object
                let obj = scly.layers
                    .as_mut_vec()[layer_id as usize]
                    .objects
                    .as_mut_vec()
                    .iter_mut()
                    .find(|obj| obj.instance_id & 0x00FFFFFF == id & 0x00FFFFFF)
                    .unwrap();

                // update it
                $update_property_data!(obj);

                return Ok(());
            }
        }

        // add new object
        let id = id.unwrap_or(area.new_object_id_from_layer_id(0));
        let scly = area.mrea().scly_section_mut();
        let layers = &mut scly.layers.as_mut_vec();
        let objects = layers[requested_layer_id.unwrap_or(0) as usize].objects.as_mut_vec();
        let property_data = $new_property_data!();
        let property_data: structs::SclyProperty = property_data.into();

        assert!(property_data.object_type() == structs::$object_type::OBJECT_TYPE);

        objects.push(
            structs::SclyObject {
                instance_id: id,
                property_data,
                connections: vec![].into(),
            }
        );

        return Ok(());
    };
}

pub fn patch_add_streamed_audio(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: StreamedAudioConfig,
) -> Result<(), String> {
    macro_rules! new {
        () => {
            structs::StreamedAudio {
                name: b"mystreamedaudio\0".as_cstr(),
                active: config.active.unwrap_or(true) as u8,
                audio_file_name: string_to_cstr(config.audio_file_name),
                no_stop_on_deactivate: config.no_stop_on_deactivate.unwrap_or(true) as u8,
                fade_in_time: config.fade_in_time.unwrap_or(0.1),
                fade_out_time: config.fade_out_time.unwrap_or(1.5),
                volume: config.volume.unwrap_or(100),
                oneshot: config.oneshot.unwrap_or(0),
                is_music: config.is_music as u8,
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_streamed_audio_mut().unwrap();

            property_data.audio_file_name = string_to_cstr(config.audio_file_name);
            property_data.is_music = config.is_music as u8;

            if let Some(active) = config.active {
                property_data.active = active as u8
            }
            if let Some(no_stop_on_deactivate) = config.no_stop_on_deactivate {
                property_data.no_stop_on_deactivate = no_stop_on_deactivate as u8
            }
            if let Some(fade_in_time) = config.fade_in_time {
                property_data.fade_in_time = fade_in_time
            }
            if let Some(fade_out_time) = config.fade_out_time {
                property_data.fade_out_time = fade_out_time
            }
            if let Some(volume) = config.volume {
                property_data.volume = volume
            }
            if let Some(oneshot) = config.oneshot {
                property_data.oneshot = oneshot
            }
        };
    }

    add_edit_obj_helper!(area, config.id, config.layer, StreamedAudio, new, update);
}

pub fn patch_add_liquid<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    config: &WaterConfig,
    resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
) -> Result<(), String> {
    let water_type = WaterType::from_str(config.liquid_type.as_str());

    /* add dependencies to area */
    {
        let deps = water_type.dependencies();
        let deps_iter = deps.iter().map(|&(file_id, fourcc)| structs::Dependency {
            asset_id: file_id,
            asset_type: fourcc,
        });

        area.add_dependencies(resources, 0, deps_iter);
    }

    let mut water_obj = water_type.to_obj();
    {
        let water = water_obj.property_data.as_water_mut().unwrap();
        water.position[0] = config.position[0];
        water.position[1] = config.position[1];
        water.position[2] = config.position[2];
        water.scale[0] = config.scale[0];
        water.scale[1] = config.scale[1];
        water.scale[2] = config.scale[2];
    }

    {
        let id = config.id;
        let requested_layer_id = config.layer;
        let mrea_id = area.mlvl_area.mrea.to_u32();

        // add more layers as needed
        if let Some(requested_layer_id) = requested_layer_id {
            while area.layer_flags.layer_count <= requested_layer_id {
                area.add_layer(b"New Layer\0".as_cstr());
            }
        }

        if let Some(id) = id {
            let scly = area.mrea().scly_section_mut();

            // try to find existing object
            let info = {
                let mut info = None;

                let layer_count = scly.layers.as_mut_vec().len();
                for _layer_id in 0..layer_count {
                    let layer = scly.layers.iter().nth(_layer_id).unwrap();

                    let obj = layer
                        .objects
                        .iter()
                        .find(|obj| obj.instance_id & 0x00FFFFFF == id & 0x00FFFFFF);

                    if let Some(obj) = obj {
                        if obj.property_data.object_type() != structs::Water::OBJECT_TYPE {
                            panic!("Failed to edit existing object 0x{:X} in room 0x{:X}: Unexpected object type 0x{:X} (expected 0x{:X})", id, mrea_id, obj.property_data.object_type(), structs::Water::OBJECT_TYPE);
                        }

                        info = Some((_layer_id as u32, obj.instance_id));
                        break;
                    }
                }

                info
            };

            if let Some(info) = info {
                let (layer_id, _) = info;

                // move and update
                if requested_layer_id.is_some() && requested_layer_id.unwrap() != layer_id {
                    let requested_layer_id = requested_layer_id.unwrap();

                    // clone existing object
                    let mut obj = scly.layers.as_mut_vec()[layer_id as usize]
                        .objects
                        .as_mut_vec()
                        .iter_mut()
                        .find(|obj| obj.instance_id & 0x00FFFFFF == id & 0x00FFFFFF)
                        .unwrap()
                        .clone();

                    // modify it
                    water_obj.property_data.as_water_mut().unwrap().active = config
                        .active
                        .unwrap_or(obj.property_data.as_water().unwrap().active != 0)
                        as u8;
                    obj.property_data = water_obj.property_data;

                    // remove original
                    scly.layers.as_mut_vec()[layer_id as usize]
                        .objects
                        .as_mut_vec()
                        .retain(|obj| obj.instance_id & 0x00FFFFFF != id & 0x00FFFFFF);

                    // re-add to target layer
                    scly.layers.as_mut_vec()[requested_layer_id as usize]
                        .objects
                        .as_mut_vec()
                        .push(obj);

                    return Ok(());
                }

                // get mutable reference to existing object
                let obj = scly.layers.as_mut_vec()[layer_id as usize]
                    .objects
                    .as_mut_vec()
                    .iter_mut()
                    .find(|obj| obj.instance_id & 0x00FFFFFF == id & 0x00FFFFFF)
                    .unwrap();

                // update it
                water_obj.property_data.as_water_mut().unwrap().active = config
                    .active
                    .unwrap_or(obj.property_data.as_water().unwrap().active != 0)
                    as u8;
                obj.property_data = water_obj.property_data;

                return Ok(());
            }
        }

        // add new object
        let id = id.unwrap_or(area.new_object_id_from_layer_id(0));
        let scly = area.mrea().scly_section_mut();
        let layers = &mut scly.layers.as_mut_vec();
        let objects = layers[requested_layer_id.unwrap_or(0) as usize]
            .objects
            .as_mut_vec();
        water_obj.property_data.as_water_mut().unwrap().active =
            config.active.unwrap_or(true) as u8;
        let property_data: structs::SclyProperty = water_obj.property_data;

        assert!(property_data.object_type() == structs::Water::OBJECT_TYPE);

        objects.push(structs::SclyObject {
            instance_id: id,
            property_data,
            connections: vec![].into(),
        });

        Ok(())
    }
}

pub fn patch_add_actor_key_frame(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: ActorKeyFrameConfig,
) -> Result<(), String> {
    macro_rules! new {
        () => {
            structs::ActorKeyFrame {
                name: b"my keyframe\0".as_cstr(),
                active: config.active.unwrap_or(true) as u8,
                animation_id: config.animation_id,
                looping: config.looping as u8,
                lifetime: config.lifetime,
                fade_out: config.fade_out,
                total_playback: config.total_playback,
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_actor_key_frame_mut().unwrap();

            if let Some(active) = config.active {
                property_data.active = active as u8
            }

            property_data.animation_id = config.animation_id;
            property_data.looping = config.looping as u8;
            property_data.lifetime = config.lifetime;
            property_data.fade_out = config.fade_out;
            property_data.total_playback = config.total_playback;
        };
    }

    add_edit_obj_helper!(
        area,
        Some(config.id),
        config.layer,
        ActorKeyFrame,
        new,
        update
    );
}

pub fn patch_add_timer(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: TimerConfig,
) -> Result<(), String> {
    macro_rules! new {
        () => {
            structs::Timer {
                name: b"my timer\0".as_cstr(),
                start_time: config.time,
                max_random_add: config.max_random_add.unwrap_or(0.0),
                looping: config.looping.unwrap_or(false) as u8,
                start_immediately: config.start_immediately.unwrap_or(false) as u8,
                active: config.active.unwrap_or(true) as u8,
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_timer_mut().unwrap();

            property_data.start_time = config.time;

            if let Some(active) = config.active {
                property_data.active = active as u8
            }
            if let Some(max_random_add) = config.max_random_add {
                property_data.max_random_add = max_random_add
            }
            if let Some(looping) = config.looping {
                property_data.looping = looping as u8
            }
            if let Some(start_immediately) = config.start_immediately {
                property_data.start_immediately = start_immediately as u8
            }
        };
    }

    add_edit_obj_helper!(area, Some(config.id), config.layer, Timer, new, update);
}

pub fn patch_add_relay(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: RelayConfig,
) -> Result<(), String> {
    macro_rules! new {
        () => {
            structs::Relay {
                name: b"my relay\0".as_cstr(),
                active: config.active.unwrap_or(true) as u8,
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_relay_mut().unwrap();
            if let Some(active) = config.active {
                property_data.active = active as u8
            }
        };
    }

    add_edit_obj_helper!(area, Some(config.id), config.layer, Relay, new, update);
}

pub fn patch_add_spawn_point(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: SpawnPointConfig,
) -> Result<(), String> {
    let spawn_point = {
        let mut spawn_point = structs::SpawnPoint {
            name: b"my spawnpoint\0".as_cstr(),
            position: config.position.into(),
            rotation: config.rotation.unwrap_or([0.0, 0.0, 0.0]).into(),
            power: 0,
            ice: 0,
            wave: 0,
            plasma: 0,
            missiles: 0,
            scan_visor: 0,
            bombs: 0,
            power_bombs: 0,
            flamethrower: 0,
            thermal_visor: 0,
            charge: 0,
            super_missile: 0,
            grapple: 0,
            xray: 0,
            ice_spreader: 0,
            space_jump: 0,
            morph_ball: 0,
            combat_visor: 0,
            boost_ball: 0,
            spider_ball: 0,
            power_suit: 0,
            gravity_suit: 0,
            varia_suit: 0,
            phazon_suit: 0,
            energy_tanks: 0,
            unknown_item_1: 0,
            health_refill: 0,
            unknown_item_2: 0,
            wavebuster: 0,
            default_spawn: config.default_spawn.unwrap_or(false) as u8,
            active: config.active.unwrap_or(true) as u8,
            morphed: config.morphed.unwrap_or(false) as u8,
        };

        if let Some(items) = config.items.as_ref() {
            items.update_spawn_point(&mut spawn_point);
        }

        spawn_point
    };

    macro_rules! new {
        () => {
            spawn_point.clone()
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_spawn_point_mut().unwrap();

            property_data.position = config.position.into();

            if let Some(items) = config.items.as_ref() {
                items.update_spawn_point(property_data);
            }

            if let Some(active) = config.active {
                property_data.active = active as u8
            }
            if let Some(default_spawn) = config.default_spawn {
                property_data.default_spawn = default_spawn as u8
            }
            if let Some(morphed) = config.morphed {
                property_data.morphed = morphed as u8
            }
            if let Some(rotation) = config.rotation {
                property_data.rotation = rotation.into()
            }
        };
    }

    add_edit_obj_helper!(area, Some(config.id), config.layer, SpawnPoint, new, update);
}

pub fn patch_add_trigger(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: TriggerConfig,
) -> Result<(), String> {
    macro_rules! new {
        () => {
            structs::Trigger {
                name: b"my trigger\0".as_cstr(),
                position: config.position.unwrap_or([0.0, 0.0, 0.0]).into(),
                scale: config.scale.unwrap_or([5.0, 5.0, 5.0]).into(),
                damage_info: structs::scly_structs::DamageInfo {
                    weapon_type: config.damage_type.unwrap_or(DamageType::Power) as u32,
                    damage: config.damage_amount.unwrap_or(0.0),
                    radius: 0.0,
                    knockback_power: 0.0,
                },
                force: config.force.unwrap_or([0.0, 0.0, 0.0]).into(),
                flags: config.flags.unwrap_or(1),
                active: config.active.unwrap_or(true) as u8,
                deactivate_on_enter: config.deactivate_on_enter.unwrap_or(false) as u8,
                deactivate_on_exit: config.deactivate_on_exit.unwrap_or(false) as u8,
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_trigger_mut().unwrap();

            if let Some(active) = config.active {
                property_data.active = active as u8
            }
            if let Some(position) = config.position {
                property_data.position = position.into()
            }
            if let Some(scale) = config.scale {
                property_data.scale = scale.into()
            }
            if let Some(damage_type) = config.damage_type {
                property_data.damage_info.weapon_type = damage_type as u32
            }
            if let Some(damage_type) = config.damage_type {
                property_data.damage_info.weapon_type = damage_type as u32
            }
            if let Some(damage_amount) = config.damage_amount {
                property_data.damage_info.damage = damage_amount
            }
            if let Some(force) = config.force {
                property_data.force = force.into()
            }
            if let Some(flags) = config.flags {
                property_data.flags = flags
            }
            if let Some(deactivate_on_enter) = config.deactivate_on_enter {
                property_data.deactivate_on_enter = deactivate_on_enter as u8
            }
            if let Some(deactivate_on_exit) = config.deactivate_on_exit {
                property_data.deactivate_on_exit = deactivate_on_exit as u8
            }
        };
    }

    add_edit_obj_helper!(area, config.id, config.layer, Trigger, new, update);
}

pub fn patch_add_special_fn(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: SpecialFunctionConfig,
) -> Result<(), String> {
    let default_unknown0 = "".to_string();
    let unknown0 = config.unknown1.as_ref().unwrap_or(&default_unknown0);
    let unknown0 = string_to_cstr(unknown0.clone());
    let pickup_type = match config.item_id.as_ref() {
        Some(item_id) => PickupType::from_str(item_id),
        None => PickupType::PowerBeam,
    };
    let item_id = pickup_type as u32;

    macro_rules! new {
        () => {
            structs::SpecialFunction {
                name: b"myspecialfun\0".as_cstr(),
                position: config.position.unwrap_or_default().into(),
                rotation: config.rotation.unwrap_or_default().into(),
                type_: config.type_ as u32,
                unknown0,
                unknown1: config.unknown2.unwrap_or_default(),
                unknown2: config.unknown3.unwrap_or_default(),
                unknown3: config.unknown4.unwrap_or_default(),
                layer_change_room_id: config.layer_change_room_id.unwrap_or(0xFFFFFFFF),
                layer_change_layer_id: config.layer_change_layer_id.unwrap_or(0xFFFFFFFF),
                item_id,
                unknown4: config.active.unwrap_or(true) as u8, // active
                unknown5: config.unknown6.unwrap_or_default(),
                unknown6: config.spinner1.unwrap_or(0xFFFFFFFF),
                unknown7: config.spinner2.unwrap_or(0xFFFFFFFF),
                unknown8: config.spinner3.unwrap_or(0xFFFFFFFF),
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_special_function_mut().unwrap();

            property_data.type_ = config.type_ as u32;

            if let Some(position) = config.position {
                property_data.position = position.into()
            }
            if let Some(rotation) = config.rotation {
                property_data.rotation = rotation.into()
            }
            if let Some(_) = config.unknown1.as_ref() {
                property_data.unknown0 = unknown0
            }
            if let Some(unknown2) = config.unknown2 {
                property_data.unknown1 = unknown2
            }
            if let Some(unknown3) = config.unknown3 {
                property_data.unknown2 = unknown3
            }
            if let Some(layer_change_room_id) = config.layer_change_room_id {
                property_data.layer_change_room_id = layer_change_room_id
            }
            if let Some(layer_change_layer_id) = config.layer_change_layer_id {
                property_data.layer_change_layer_id = layer_change_layer_id
            }
            if let Some(item_id) = config.item_id {
                property_data.item_id = PickupType::from_str(&item_id) as u32;
            };
            if let Some(active) = config.active {
                property_data.unknown4 = active as u8
            }
            if let Some(unknown6) = config.unknown6 {
                property_data.unknown5 = unknown6
            }
            if let Some(spinner1) = config.spinner1 {
                property_data.unknown6 = spinner1
            }
            if let Some(spinner2) = config.spinner2 {
                property_data.unknown7 = spinner2
            }
            if let Some(spinner3) = config.spinner3 {
                property_data.unknown8 = spinner3
            }
        };
    }

    add_edit_obj_helper!(area, config.id, config.layer, SpecialFunction, new, update);
}

pub fn patch_add_hudmemo<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    config: HudmemoConfig,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    strg_id: Option<ResId<res_id::STRG>>,
) -> Result<(), String> {
    let memo_type = match config.modal.unwrap_or(false) {
        false => 0,
        true => 1,
    };

    macro_rules! new {
        () => {
            structs::HudMemo {
                name: b"my hudmemo\0".as_cstr(),
                first_message_timer: config.message_time.unwrap_or(4.0),
                unknown: 1,
                memo_type,
                strg: strg_id.unwrap_or(ResId::invalid()),
                active: config.active.unwrap_or(true) as u8,
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_hud_memo_mut().unwrap();

            if config.modal.is_some() {
                property_data.memo_type = memo_type;
            }

            if let Some(strg_id) = strg_id {
                property_data.strg = strg_id
            }
            if let Some(message_time) = config.message_time {
                property_data.first_message_timer = message_time
            }
            if let Some(active) = config.active {
                property_data.active = active as u8
            }
        };
    }

    if let Some(strg_id) = strg_id {
        let strg_dep: structs::Dependency = strg_id.into();
        area.add_dependencies(game_resources, 0, iter::once(strg_dep));
    }

    add_edit_obj_helper!(area, Some(config.id), config.layer, HudMemo, new, update);
}

pub fn patch_add_actor_rotate_fn(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: ActorRotateConfig,
) -> Result<(), String> {
    macro_rules! new {
        () => {
            structs::ActorRotate {
                name: b"my actor rotate\0".as_cstr(),
                rotation: config.rotation.into(),
                time_scale: config.time_scale,
                update_actors: config.update_actors as u8,
                update_on_creation: config.update_on_creation as u8,
                update_active: config.update_active as u8,
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_actor_rotate_mut().unwrap();

            property_data.rotation = config.rotation.into();
            property_data.time_scale = config.time_scale;
            property_data.update_actors = config.update_actors as u8;
            property_data.update_on_creation = config.update_on_creation as u8;
            property_data.update_active = config.update_active as u8;
        };
    }

    add_edit_obj_helper!(area, config.id, config.layer, ActorRotate, new, update);
}

pub fn patch_add_waypoint(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: WaypointConfig,
) -> Result<(), String> {
    macro_rules! new {
        () => {
            structs::Waypoint {
                name: b"my waypoint\0".as_cstr(),
                position: config.position.unwrap_or([0.0, 0.0, 0.0]).into(),
                rotation: config.rotation.unwrap_or([0.0, 0.0, 0.0]).into(),
                active: config.active.unwrap_or(true) as u8,
                speed: config.speed.unwrap_or(1.0),
                pause: config.pause.unwrap_or(0.0),
                pattern_translate: config.pattern_translate.unwrap_or(0),
                pattern_orient: config.pattern_orient.unwrap_or(0),
                pattern_fit: config.pattern_fit.unwrap_or(0),
                behaviour: config.behaviour.unwrap_or(0),
                behaviour_orient: config.behaviour_orient.unwrap_or(0),
                behaviour_modifiers: config.behaviour_modifiers.unwrap_or(0),
                animation: config.animation.unwrap_or(0),
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_waypoint_mut().unwrap();
            if let Some(position) = config.position {
                property_data.position = position.into()
            }
            if let Some(rotation) = config.rotation {
                property_data.rotation = rotation.into()
            }
            if let Some(active) = config.active {
                property_data.active = active as u8
            }
            if let Some(speed) = config.speed {
                property_data.speed = speed
            }
            if let Some(pause) = config.pause {
                property_data.pause = pause
            }
            if let Some(pattern_translate) = config.pattern_translate {
                property_data.pattern_translate = pattern_translate
            }
            if let Some(pattern_orient) = config.pattern_orient {
                property_data.pattern_orient = pattern_orient
            }
            if let Some(pattern_fit) = config.pattern_fit {
                property_data.pattern_fit = pattern_fit
            }
            if let Some(behaviour) = config.behaviour {
                property_data.behaviour = behaviour
            }
            if let Some(behaviour_orient) = config.behaviour_orient {
                property_data.behaviour_orient = behaviour_orient
            }
            if let Some(behaviour_modifiers) = config.behaviour_modifiers {
                property_data.behaviour_modifiers = behaviour_modifiers
            }
            if let Some(animation) = config.animation {
                property_data.animation = animation
            }
        };
    }

    add_edit_obj_helper!(area, Some(config.id), config.layer, Waypoint, new, update);
}

pub fn patch_add_counter(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: CounterConfig,
) -> Result<(), String> {
    macro_rules! new {
        () => {
            structs::Counter {
                name: b"my counter\0".as_cstr(),
                start_value: config.start_value.unwrap_or(0),
                max_value: config.max_value.unwrap_or(1),
                auto_reset: config.auto_reset.unwrap_or(false) as u8,
                active: config.active.unwrap_or(true) as u8,
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_counter_mut().unwrap();
            if let Some(start_value) = config.start_value {
                property_data.start_value = start_value
            }
            if let Some(max_value) = config.max_value {
                property_data.max_value = max_value
            }
            if let Some(auto_reset) = config.auto_reset {
                property_data.auto_reset = auto_reset as u8
            }
            if let Some(active) = config.active {
                property_data.active = active as u8
            }
        };
    }

    add_edit_obj_helper!(area, Some(config.id), config.layer, Counter, new, update);
}

pub fn patch_add_switch(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: SwitchConfig,
) -> Result<(), String> {
    macro_rules! new {
        () => {
            structs::Switch {
                name: b"my switch\0".as_cstr(),
                active: config.active.unwrap_or(true) as u8,
                open: config.open.unwrap_or(false) as u8,
                auto_close: config.auto_close.unwrap_or(false) as u8,
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_switch_mut().unwrap();
            if let Some(active) = config.active {
                property_data.active = active as u8
            }
            if let Some(open) = config.open {
                property_data.open = open as u8
            }
            if let Some(auto_close) = config.auto_close {
                property_data.auto_close = auto_close as u8
            }
        };
    }

    add_edit_obj_helper!(area, Some(config.id), config.layer, Switch, new, update);
}

pub fn patch_add_player_hint(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: PlayerHintConfig,
) -> Result<(), String> {
    macro_rules! new {
        () => {
            structs::PlayerHint {
                name: b"my playerhint\0".as_cstr(),

                position: [0.0, 0.0, 0.0].into(),
                rotation: [0.0, 0.0, 0.0].into(),

                active: config.active.unwrap_or(true) as u8,

                data: structs::PlayerHintStruct {
                    unknown1: config.unknown1.unwrap_or(false) as u8,
                    unknown2: config.unknown2.unwrap_or(false) as u8,
                    extend_target_distance: config.extend_target_distance.unwrap_or(false) as u8,
                    unknown4: config.unknown4.unwrap_or(false) as u8,
                    unknown5: config.unknown5.unwrap_or(false) as u8,
                    disable_unmorph: config.disable_unmorph.unwrap_or(false) as u8,
                    disable_morph: config.disable_morph.unwrap_or(false) as u8,
                    disable_controls: config.disable_controls.unwrap_or(false) as u8,
                    disable_boost: config.disable_boost.unwrap_or(false) as u8,
                    activate_visor_combat: config.activate_visor_combat.unwrap_or(false) as u8,
                    activate_visor_scan: config.activate_visor_scan.unwrap_or(false) as u8,
                    activate_visor_thermal: config.activate_visor_thermal.unwrap_or(false) as u8,
                    activate_visor_xray: config.activate_visor_xray.unwrap_or(false) as u8,
                    unknown6: config.unknown6.unwrap_or(false) as u8,
                    face_object_on_unmorph: config.face_object_on_unmorph.unwrap_or(false) as u8,
                }
                .into(),

                priority: config.priority.unwrap_or(10),
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_player_hint_mut().unwrap();
            if let Some(active) = config.active {
                property_data.active = active as u8
            }
            if let Some(priority) = config.priority {
                property_data.priority = priority
            }
            if let Some(unknown1) = config.unknown1 {
                property_data.data.unknown1 = unknown1 as u8
            }
            if let Some(unknown2) = config.unknown2 {
                property_data.data.unknown2 = unknown2 as u8
            }
            if let Some(extend_target_distance) = config.extend_target_distance {
                property_data.data.extend_target_distance = extend_target_distance as u8
            }
            if let Some(unknown4) = config.unknown4 {
                property_data.data.unknown4 = unknown4 as u8
            }
            if let Some(unknown5) = config.unknown5 {
                property_data.data.unknown5 = unknown5 as u8
            }
            if let Some(disable_unmorph) = config.disable_unmorph {
                property_data.data.disable_unmorph = disable_unmorph as u8
            }
            if let Some(disable_morph) = config.disable_morph {
                property_data.data.disable_morph = disable_morph as u8
            }
            if let Some(disable_controls) = config.disable_controls {
                property_data.data.disable_controls = disable_controls as u8
            }
            if let Some(disable_boost) = config.disable_boost {
                property_data.data.disable_boost = disable_boost as u8
            }
            if let Some(activate_visor_combat) = config.activate_visor_combat {
                property_data.data.activate_visor_combat = activate_visor_combat as u8
            }
            if let Some(activate_visor_scan) = config.activate_visor_scan {
                property_data.data.activate_visor_scan = activate_visor_scan as u8
            }
            if let Some(activate_visor_thermal) = config.activate_visor_thermal {
                property_data.data.activate_visor_thermal = activate_visor_thermal as u8
            }
            if let Some(activate_visor_xray) = config.activate_visor_xray {
                property_data.data.activate_visor_xray = activate_visor_xray as u8
            }
            if let Some(unknown6) = config.unknown6 {
                property_data.data.unknown6 = unknown6 as u8
            }
            if let Some(face_object_on_unmorph) = config.face_object_on_unmorph {
                property_data.data.face_object_on_unmorph = face_object_on_unmorph as u8
            }
        };
    }

    add_edit_obj_helper!(area, Some(config.id), config.layer, PlayerHint, new, update);
}

pub fn patch_add_distance_fogs(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: FogConfig,
) -> Result<(), String> {
    macro_rules! new {
        () => {
            structs::DistanceFog {
                name: b"my fog\0".as_cstr(),
                mode: config.mode.unwrap_or(1),
                color: config.color.unwrap_or([0.8, 0.8, 0.9, 0.0]).into(),
                range: config.range.unwrap_or([30.0, 40.0]).into(),
                color_delta: config.color_delta.unwrap_or(0.0),
                range_delta: config.range_delta.unwrap_or([0.0, 0.0]).into(),
                explicit: config.explicit.unwrap_or(true) as u8,
                active: config.active.unwrap_or(true) as u8,
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_distance_fog_mut().unwrap();
            if let Some(mode) = config.mode {
                property_data.mode = mode
            }
            if let Some(color) = config.color {
                property_data.color = color.into()
            }
            if let Some(range) = config.range {
                property_data.range = range.into()
            }
            if let Some(color_delta) = config.color_delta {
                property_data.color_delta = color_delta
            }
            if let Some(range_delta) = config.range_delta {
                property_data.range_delta = range_delta.into()
            }
            if let Some(explicit) = config.explicit {
                property_data.explicit = explicit as u8
            }
            if let Some(active) = config.active {
                property_data.active = active as u8
            }
        };
    }

    add_edit_obj_helper!(area, config.id, config.layer, DistanceFog, new, update);
}

use nalgebra::{Matrix3, Vector3};

enum Rotation {
    Pitch(f32),
    Roll(f32),
    Yaw(f32),
}

use Rotation::*;

fn rotation_matrix(rotations: [Rotation; 3]) -> Matrix3<f32> {
    let mut matrix = Matrix3::identity();

    for rotation in rotations {
        matrix *= match rotation {
            Pitch(angle) => {
                let rad = angle.to_radians();
                Matrix3::new(
                    1.0,
                    0.0,
                    0.0,
                    0.0,
                    rad.cos(),
                    -rad.sin(),
                    0.0,
                    rad.sin(),
                    rad.cos(),
                )
            }
            Roll(angle) => {
                let rad = angle.to_radians();
                Matrix3::new(
                    rad.cos(),
                    0.0,
                    rad.sin(),
                    0.0,
                    1.0,
                    0.0,
                    -rad.sin(),
                    0.0,
                    rad.cos(),
                )
            }
            Yaw(angle) => {
                let rad = angle.to_radians();
                Matrix3::new(
                    rad.cos(),
                    -rad.sin(),
                    0.0,
                    rad.sin(),
                    rad.cos(),
                    0.0,
                    0.0,
                    0.0,
                    1.0,
                )
            }
        };
    }

    matrix
}

fn apply_rotation(matrix: &Matrix3<f32>, vector: Vector3<f32>) -> Vector3<f32> {
    matrix * vector
}

pub fn relative_offset(position: [f32; 3], rotation: [f32; 3], offset: [f32; 3]) -> [f32; 3] {
    let rotations = [Yaw(rotation[2]), Roll(rotation[1]), Pitch(rotation[0])];
    let rotation_matrix = rotation_matrix(rotations);
    let position = Vector3::from_column_slice(&position);
    let offset = Vector3::from_column_slice(&offset);

    let rotated_offset = apply_rotation(&rotation_matrix, offset);
    let adjusted_position = position + rotated_offset;

    adjusted_position.into()
}

pub fn patch_add_bomb_slot<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    config: BombSlotConfig,
) -> Result<(), String> {
    let layer = match config.layer {
        Some(layer) => {
            while area.layer_flags.layer_count <= layer {
                area.add_layer(b"New Layer\0".as_cstr());
            }
            layer
        }
        None => 0,
    } as usize;

    let deps = [
        (0x3852C9CF, b"CMDL"),
        (0x5B4D184E, b"TXTR"),
        (0x89CC3758, b"DCLN"),
        // glow actor
        (0xA88267E6, b"CMDL"),
        (0xD64787E8, b"TXTR"),
    ];
    let deps_iter = deps.iter().map(|&(file_id, fourcc)| structs::Dependency {
        asset_id: file_id,
        asset_type: FourCC::from_bytes(fourcc),
    });
    area.add_dependencies(game_resources, layer, deps_iter);

    let bomb_slot_id = config
        .platform_id
        .unwrap_or(area.new_object_id_from_layer_id(layer));
    let glow_ring_id = config
        .actor_id
        .unwrap_or(area.new_object_id_from_layer_id(layer));
    let ball_trigger_id = config
        .ball_trigger_id
        .unwrap_or(area.new_object_id_from_layer_id(layer));
    let player_hint_id = area.new_object_id_from_layer_id(layer);
    let streamed_audio_id = area.new_object_id_from_layer_id(layer);
    let timer_id = area.new_object_id_from_layer_id(layer);
    let damageable_trigger_id = config.damageable_trigger_id;

    let offset = [0.0, -1.05, 0.0];
    let ball_trigger_position = relative_offset(config.position, config.rotation, offset);
    let ball_release_delay_s = config.release_ball_delay_s.unwrap_or(2.0);
    let active = config.active.unwrap_or(true) as u8;

    let scly = area.mrea().scly_section_mut();
    let objects = scly.layers.as_mut_vec()[layer].objects.as_mut_vec();

    objects.extend_from_slice(&[
        // Energy core used as reference
        structs::SclyObject {
            instance_id: bomb_slot_id,
            property_data: structs::Platform {
                name: b"bombslotplatform\0".as_cstr(),

                position: config.position.into(),
                rotation: config.rotation.into(),
                scale: [1.034, 1.0, 1.034].into(),
                extent: [0.0, 0.0, 0.0].into(),
                scan_offset: [0.0, 0.0, 0.0].into(),

                cmdl: ResId::<res_id::CMDL>::new(0x3852C9CF),

                ancs: structs::scly_structs::AncsProp {
                    file_id: ResId::invalid(),
                    node_index: 0,
                    default_animation: 0xFFFFFFFF,
                },
                actor_params: structs::scly_structs::ActorParameters {
                    light_params: structs::scly_structs::LightParameters {
                        unknown0: 1,
                        unknown1: 1.0,
                        shadow_tessellation: 0,
                        unknown2: 1.0,
                        unknown3: 20.0,
                        color: [1.0, 1.0, 1.0, 1.0].into(),
                        unknown4: 1,
                        world_lighting: 3,
                        light_recalculation: 1,
                        unknown5: [0.0, 0.0, 0.0].into(),
                        unknown6: 4,
                        unknown7: 4,
                        unknown8: 0,
                        light_layer_id: 0,
                    },
                    scan_params: structs::scly_structs::ScannableParameters {
                        scan: ResId::invalid(), // None
                    },
                    xray_cmdl: ResId::invalid(),    // None
                    xray_cskr: ResId::invalid(),    // None
                    thermal_cmdl: ResId::invalid(), // None
                    thermal_cskr: ResId::invalid(), // None

                    unknown0: 1,
                    unknown1: 1.0,
                    unknown2: 1.0,

                    visor_params: structs::scly_structs::VisorParameters {
                        unknown0: 0,
                        target_passthrough: 0,
                        visor_mask: 15, // Combat|Scan|Thermal|XRay
                    },
                    enable_thermal_heat: 0,
                    unknown3: 0,
                    unknown4: 0,
                    unknown5: 1.0,
                },

                speed: 1.0,
                active: 1,

                dcln: ResId::<res_id::DCLN>::new(0x89CC3758),

                health_info: structs::scly_structs::HealthInfo {
                    health: 1.0,
                    knockback_resistance: 1.0,
                },
                damage_vulnerability: DoorType::Disabled.vulnerability(),

                detect_collision: 0,
                unknown4: 1.0,
                unknown5: 0,
                unknown6: 200,
                unknown7: 20,
            }
            .into(),
            connections: vec![].into(),
        },
        structs::SclyObject {
            instance_id: glow_ring_id,
            property_data: structs::Actor {
                name: b"myactor\0".as_cstr(),
                position: relative_offset(config.position, config.rotation, [0.0125, 0.0, 0.0])
                    .into(),
                rotation: config.rotation.into(),
                scale: [1.034, 1.0, 1.034].into(),
                hitbox: [0.0, 0.0, 0.0].into(),
                scan_offset: [0.0, 0.0, 0.0].into(),
                unknown1: 1.0,
                unknown2: 0.0,
                health_info: structs::scly_structs::HealthInfo {
                    health: 5.0,
                    knockback_resistance: 1.0,
                },
                damage_vulnerability: DoorType::Disabled.vulnerability(),
                cmdl: ResId::<res_id::CMDL>::new(0xA88267E6),
                ancs: structs::scly_structs::AncsProp {
                    file_id: ResId::invalid(), // None
                    node_index: 0,
                    default_animation: 0xFFFFFFFF, // -1
                },
                actor_params: structs::scly_structs::ActorParameters {
                    light_params: structs::scly_structs::LightParameters {
                        unknown0: 1,
                        unknown1: 1.0,
                        shadow_tessellation: 0,
                        unknown2: 1.0,
                        unknown3: 20.0,
                        color: [1.0, 1.0, 1.0, 1.0].into(),
                        unknown4: 1,
                        world_lighting: 3,
                        light_recalculation: 1,
                        unknown5: [0.0, 0.0, 0.0].into(),
                        unknown6: 4,
                        unknown7: 4,
                        unknown8: 0,
                        light_layer_id: 0,
                    },
                    scan_params: structs::scly_structs::ScannableParameters {
                        scan: ResId::invalid(), // None
                    },
                    xray_cmdl: ResId::invalid(),    // None
                    xray_cskr: ResId::invalid(),    // None
                    thermal_cmdl: ResId::invalid(), // None
                    thermal_cskr: ResId::invalid(), // None

                    unknown0: 1,
                    unknown1: 1.0,
                    unknown2: 1.0,

                    visor_params: structs::scly_structs::VisorParameters {
                        unknown0: 0,
                        target_passthrough: 0,
                        visor_mask: 15, // Combat|Scan|Thermal|XRay
                    },
                    enable_thermal_heat: 1,
                    unknown3: 0,
                    unknown4: 0,
                    unknown5: 1.0,
                },
                looping: 1,
                snow: 1,
                solid: 0,
                camera_passthrough: 0,
                active,
                unknown8: 0,
                unknown9: 1.0,
                unknown10: 0,
                unknown11: 0,
                unknown12: 0,
                unknown13: 0,
            }
            .into(),
            connections: vec![].into(),
        },
        structs::SclyObject {
            instance_id: ball_trigger_id,
            property_data: structs::BallTrigger {
                name: b"myballtrigger\0".as_cstr(),
                position: ball_trigger_position.into(),
                scale: [1.0, 1.0, 1.0].into(),
                active,
                force: 40.0,
                min_angle: 180.0,
                max_distance: 1.5,
                force_angle: [1.0, 1.0, 1.0].into(),
                stop_player: 1,
            }
            .into(),
            connections: vec![
                structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::ACTIVATE,
                    target_object_id: damageable_trigger_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::EXITED,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: damageable_trigger_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::INACTIVE,
                    message: structs::ConnectionMsg::DECREMENT,
                    target_object_id: player_hint_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::INCREMENT,
                    target_object_id: player_hint_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::EXITED,
                    message: structs::ConnectionMsg::DECREMENT,
                    target_object_id: player_hint_id,
                },
            ]
            .into(),
        },
        structs::SclyObject {
            instance_id: player_hint_id,
            property_data: structs::PlayerHint {
                name: b"disableboost\0".as_cstr(),
                position: [0.0, 0.0, 0.0].into(),
                rotation: [0.0, 0.0, 0.0].into(),
                active: 1,
                data: structs::PlayerHintStruct {
                    unknown1: 1,
                    unknown2: 0,
                    extend_target_distance: 0,
                    unknown4: 0,
                    unknown5: 0,
                    disable_unmorph: 1,
                    disable_morph: 0,
                    disable_controls: 0,
                    disable_boost: 1,
                    activate_visor_combat: 0,
                    activate_visor_scan: 0,
                    activate_visor_thermal: 0,
                    activate_visor_xray: 0,
                    unknown6: 0,
                    face_object_on_unmorph: 0,
                },
                priority: 10,
            }
            .into(),
            connections: vec![].into(),
        },
        structs::SclyObject {
            instance_id: streamed_audio_id,
            property_data: structs::StreamedAudio {
                name: b"mystreamedaudio\0".as_cstr(),
                active: 1,
                audio_file_name: b"/audio/evt_x_event_00.dsp\0".as_cstr(),
                no_stop_on_deactivate: 0,
                fade_in_time: 0.0,
                fade_out_time: 0.0,
                volume: 92,
                oneshot: 1,
                is_music: 1,
            }
            .into(),
            connections: vec![].into(),
        },
        structs::SclyObject {
            instance_id: damageable_trigger_id,
            property_data: structs::DamageableTrigger {
                name: b"my dtrigger\0".as_cstr(),
                position: ball_trigger_position.into(),
                scale: [0.1, 0.1, 0.1].into(),
                health_info: structs::scly_structs::HealthInfo {
                    health: 1.0,
                    knockback_resistance: 1.0,
                },
                damage_vulnerability: DoorType::Bomb.vulnerability(),
                unknown0: 0,
                pattern_txtr0: ResId::invalid(),
                pattern_txtr1: ResId::invalid(),
                color_txtr: ResId::invalid(),
                lock_on: 0,
                active: 0,
                visor_params: structs::scly_structs::VisorParameters {
                    unknown0: 0,
                    target_passthrough: 0,
                    visor_mask: 15, // Combat|Scan|Thermal|XRay
                },
            }
            .into(),
            connections: vec![
                structs::Connection {
                    state: structs::ConnectionState::DEAD,
                    message: structs::ConnectionMsg::DECREMENT,
                    target_object_id: glow_ring_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::DEAD,
                    message: structs::ConnectionMsg::RESET_AND_START,
                    target_object_id: timer_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::DEAD,
                    message: structs::ConnectionMsg::PLAY,
                    target_object_id: streamed_audio_id,
                },
            ]
            .into(),
        },
        structs::SclyObject {
            instance_id: timer_id,
            property_data: structs::Timer {
                name: b"timer fade in\0".as_cstr(),
                start_time: ball_release_delay_s,
                max_random_add: 0.0,
                looping: 0,
                start_immediately: 0,
                active: 1,
            }
            .into(),
            connections: vec![structs::Connection {
                state: structs::ConnectionState::ZERO,
                message: structs::ConnectionMsg::DEACTIVATE,
                target_object_id: ball_trigger_id,
            }]
            .into(),
        },
    ]);

    if let Some(activate_slot_id) = config.activate_slot_id {
        objects.push(structs::SclyObject {
            instance_id: activate_slot_id,
            property_data: structs::Relay {
                name: b"muh relay\0".as_cstr(),
                active: 1,
            }
            .into(),
            connections: vec![
                structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::ACTIVATE,
                    target_object_id: ball_trigger_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::INCREMENT,
                    target_object_id: glow_ring_id,
                },
            ]
            .into(),
        });
    }

    if let Some(deactivate_slot_id) = config.deactivate_slot_id {
        objects.push(structs::SclyObject {
            instance_id: deactivate_slot_id,
            property_data: structs::Relay {
                name: b"muh relay\0".as_cstr(),
                active: 1,
            }
            .into(),
            connections: vec![
                structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: ball_trigger_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: damageable_trigger_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::DECREMENT,
                    target_object_id: glow_ring_id,
                },
            ]
            .into(),
        });
    }

    Ok(())
}

fn player_actor_data<'r>() -> structs::PlayerActor<'r> {
    let bytes: &'static [u8] = &[
        0x00, 0x00, 0x00, 0x13, 0x50, 0x6C, 0x61, 0x79, 0x65, 0x72, 0x41, 0x63, 0x74, 0x6F, 0x72,
        0x20, 0x2D, 0x20, 0x4C, 0x65, 0x61, 0x76, 0x69, 0x6E, 0x67, 0x2D, 0x63, 0x6F, 0x6D, 0x70,
        0x6F, 0x6E, 0x65, 0x6E, 0x74, 0x00, 0x43, 0x33, 0xE1, 0x87, 0xC4, 0x54, 0x93, 0xA5, 0x42,
        0x83, 0x6B, 0x69, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x40, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3F, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x02, 0x40, 0xA0, 0x00, 0x00, 0x3F, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x12,
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
        0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
        0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00,
        0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
        0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x05, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00,
        0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0x77, 0x28, 0x9A, 0x4A,
        0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x0E, 0x00, 0x00, 0x00,
        0x0E, 0x01, 0x3F, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3F, 0x80, 0x00, 0x00, 0x41,
        0xA0, 0x00, 0x00, 0x3F, 0x80, 0x00, 0x00, 0x3F, 0x80, 0x00, 0x00, 0x3F, 0x80, 0x00, 0x00,
        0x3F, 0x80, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00,
        0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0xFF, 0xFF, 0xFF,
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        0xFF, 0xFF, 0x01, 0x3F, 0x80, 0x00, 0x00, 0x3F, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x0F, 0x00, 0x00, 0x00, 0x3F, 0x80, 0x00, 0x00, 0x01, 0x01,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    Reader::new(bytes).read(())
}

pub fn patch_add_player_actor<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    config: PlayerActorConfig,
) -> Result<(), String> {
    let deps = [(0x836c33b3, b"ANCS")];
    let deps_iter = deps.iter().map(|&(file_id, fourcc)| structs::Dependency {
        asset_id: file_id,
        asset_type: FourCC::from_bytes(fourcc),
    });
    area.add_dependencies(game_resources, 0, deps_iter);

    let mut property_data = player_actor_data();
    property_data.active = config.active.unwrap_or(true) as u8;
    property_data.position = config.position.unwrap_or([0.0, 0.0, 0.0]).into();
    property_data.rotation = config.rotation.unwrap_or([0.0, 0.0, 0.0]).into();

    macro_rules! new {
        () => {
            property_data
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_player_actor_mut().unwrap();
            if let Some(active) = config.active {
                property_data.active = active as u8
            }
            if let Some(position) = config.position {
                property_data.position = position.into()
            }
            if let Some(rotation) = config.rotation {
                property_data.rotation = rotation.into()
            }
        };
    }

    add_edit_obj_helper!(area, config.id, config.layer, PlayerActor, new, update);
}

pub fn patch_add_world_light_fader(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: WorldLightFaderConfig,
) -> Result<(), String> {
    macro_rules! new {
        () => {
            structs::WorldLightFader {
                name: b"my world light fader\0".as_cstr(),
                active: config.active.unwrap_or(true) as u8,
                faded_light_level: config.faded_light_level.unwrap_or(0.2),
                fade_speed: config.fade_speed.unwrap_or(0.25),
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_world_light_fader_mut().unwrap();
            if let Some(active) = config.active {
                property_data.active = active as u8
            }
            if let Some(faded_light_level) = config.faded_light_level {
                property_data.faded_light_level = faded_light_level
            }
            if let Some(fade_speed) = config.fade_speed {
                property_data.fade_speed = fade_speed
            }
        };
    }

    add_edit_obj_helper!(
        area,
        Some(config.id),
        config.layer,
        WorldLightFader,
        new,
        update
    );
}

pub fn patch_add_controller_action(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: ControllerActionConfig,
) -> Result<(), String> {
    macro_rules! new {
        () => {
            structs::ControllerAction {
                name: b"my ctrlaction\0".as_cstr(),
                active: config.active.unwrap_or(true) as u8,
                action: config.action as u32,
                one_shot: config.one_shot.unwrap_or(false) as u8,
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_controller_action_mut().unwrap();

            property_data.action = config.action as u32;

            if let Some(active) = config.active {
                property_data.active = active as u8
            }
            if let Some(one_shot) = config.one_shot {
                property_data.one_shot = one_shot as u8
            }
        };
    }

    add_edit_obj_helper!(
        area,
        Some(config.id),
        config.layer,
        ControllerAction,
        new,
        update
    );
}

pub fn patch_add_camera(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: CameraConfig,
) -> Result<(), String> {
    macro_rules! new {
        () => {
            structs::Camera {
                name: b"my camera\0".as_cstr(),
                position: config.position.unwrap_or([0.0, 0.0, 0.0]).into(),
                rotation: config.rotation.unwrap_or([0.0, 0.0, 0.0]).into(),
                active: config.active.unwrap_or(false) as u8,
                shot_duration: config.shot_duration.unwrap_or(10.0) as f32,
                look_at_player: config.look_at_player.unwrap_or(false) as u8,
                out_of_player_eye: config.out_of_player_eye.unwrap_or(false) as u8,
                into_player_eye: config.into_player_eye.unwrap_or(false) as u8,
                draw_player: config.draw_player.unwrap_or(false) as u8,
                disable_input: config.disable_input.unwrap_or(true) as u8,
                unknown: config.unknown.unwrap_or(false) as u8,
                finish_cine_skip: config.finish_cine_skip.unwrap_or(false) as u8,
                field_of_view: config.field_of_view.unwrap_or(70.0) as f32,
                check_failsafe: config.check_failsafe.unwrap_or(true) as u8,
                disable_out_of_into: config.disable_out_of_into.unwrap_or(false) as u8,
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_camera_mut().unwrap();

            if let Some(position) = config.position {
                property_data.position = position.into()
            }
            if let Some(rotation) = config.rotation {
                property_data.rotation = rotation.into()
            }
            if let Some(active) = config.active {
                property_data.active = active as u8
            }
            if let Some(shot_duration) = config.shot_duration {
                property_data.shot_duration = shot_duration as f32
            }
            if let Some(look_at_player) = config.look_at_player {
                property_data.look_at_player = look_at_player as u8
            }
            if let Some(out_of_player_eye) = config.out_of_player_eye {
                property_data.out_of_player_eye = out_of_player_eye as u8
            }
            if let Some(into_player_eye) = config.into_player_eye {
                property_data.into_player_eye = into_player_eye as u8
            }
            if let Some(draw_player) = config.draw_player {
                property_data.draw_player = draw_player as u8
            }
            if let Some(disable_input) = config.disable_input {
                property_data.disable_input = disable_input as u8
            }
            if let Some(unknown) = config.unknown {
                property_data.unknown = unknown as u8
            }
            if let Some(finish_cine_skip) = config.finish_cine_skip {
                property_data.finish_cine_skip = finish_cine_skip as u8
            }
            if let Some(field_of_view) = config.field_of_view {
                property_data.field_of_view = field_of_view as f32
            }
            if let Some(check_failsafe) = config.check_failsafe {
                property_data.check_failsafe = check_failsafe as u8
            }
            if let Some(disable_out_of_into) = config.disable_out_of_into {
                property_data.disable_out_of_into = disable_out_of_into as u8
            }
        };
    }

    add_edit_obj_helper!(area, Some(config.id), config.layer, Camera, new, update);
}

pub fn patch_add_camera_waypoint(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: CameraWaypointConfig,
) -> Result<(), String> {
    macro_rules! new {
        () => {
            structs::CameraWaypoint {
                name: b"my camera waypoint\0".as_cstr(),
                position: config.position.unwrap_or([0.0, 0.0, 0.0]).into(),
                rotation: config.rotation.unwrap_or([0.0, 0.0, 0.0]).into(),
                active: config.active.unwrap_or(true) as u8,
                fov: config.fov.unwrap_or(70.0) as f32,
                unknown: config.unknown.unwrap_or(0) as u32,
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_camera_waypoint_mut().unwrap();

            if let Some(position) = config.position {
                property_data.position = position.into()
            }
            if let Some(rotation) = config.rotation {
                property_data.rotation = rotation.into()
            }
            if let Some(active) = config.active {
                property_data.active = active as u8
            }
            if let Some(fov) = config.fov {
                property_data.fov = fov as f32
            }
            if let Some(unknown) = config.unknown {
                property_data.unknown = unknown as u32
            }
        };
    }

    add_edit_obj_helper!(
        area,
        Some(config.id),
        config.layer,
        CameraWaypoint,
        new,
        update
    );
}

pub fn patch_add_camera_filter_keyframe(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: CameraFilterKeyframeConfig,
) -> Result<(), String> {
    macro_rules! new {
        () => {
            structs::CameraFilterKeyframe {
                name: b"my filter\0".as_cstr(),
                active: config.active.unwrap_or(true) as u8,
                filter_type: config.filter_type as u32,
                filter_shape: config.filter_shape as u32,
                filter_index: config.filter_index.unwrap_or(0) as u32,
                filter_group: config.filter_group.unwrap_or(0) as u32,
                color: config.color.unwrap_or([0.0, 0.0, 0.0, 1.0]).into(),
                fade_in_time: config.fade_in_time.unwrap_or(0.0) as f32,
                fade_out_time: config.fade_out_time.unwrap_or(0.0) as f32,
                overlay_texture: config.overlay_texture.unwrap_or(0xFFFFFFFF) as u32,
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_camera_filter_keyframe_mut().unwrap();

            property_data.filter_type = config.filter_type as u32;
            property_data.filter_shape = config.filter_shape as u32;

            if let Some(active) = config.active {
                property_data.active = active as u8
            }
            if let Some(filter_index) = config.filter_index {
                property_data.filter_index = filter_index as u32
            }
            if let Some(filter_group) = config.filter_group {
                property_data.filter_group = filter_group as u32
            }
            if let Some(color) = config.color {
                property_data.color = color.into()
            }
            if let Some(fade_in_time) = config.fade_in_time {
                property_data.fade_in_time = fade_in_time as f32
            }
            if let Some(fade_out_time) = config.fade_out_time {
                property_data.fade_out_time = fade_out_time as f32
            }
            if let Some(overlay_texture) = config.overlay_texture {
                property_data.overlay_texture = overlay_texture as u32
            }
        };
    }

    add_edit_obj_helper!(
        area,
        Some(config.id),
        config.layer,
        CameraFilterKeyframe,
        new,
        update
    );
}

#[allow(clippy::too_many_arguments)]
pub fn patch_add_new_camera_hint(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: NewCameraHintConfig,
) -> Result<(), String> {
    macro_rules! new {
        () => {
            structs::CameraHint {
                name: b"my camerahint\0".as_cstr(),
                position: config.position.unwrap_or([0.0, 0.0, 0.0]).into(),
                rotation: config.rotation.unwrap_or([0.0, 0.0, 0.0]).into(),
                active: config.active.unwrap_or(true) as u8,
                priority: config.priority.unwrap_or(10) as u32,
                behavior: config.behaviour as u32,

                camera_hint_params: structs::scly_props::structs::CameraHintParameters {
                    calculate_cam_pos: config.calculate_cam_pos.unwrap_or(false) as u8,
                    chase_allowed: config.chase_allowed.unwrap_or(false) as u8,
                    boost_allowed: config.boost_allowed.unwrap_or(false) as u8,
                    obscure_avoidance: config.obscure_avoidance.unwrap_or(false) as u8,
                    volume_collider: config.volume_collider.unwrap_or(false) as u8,
                    apply_immediately: config.apply_immediately.unwrap_or(false) as u8,
                    look_at_ball: config.look_at_ball.unwrap_or(false) as u8,
                    hint_distance_selection: config.hint_distance_selection.unwrap_or(false) as u8,
                    hint_distance_self_pos: config.hint_distance_self_pos.unwrap_or(false) as u8,
                    control_interpolation: config.control_interpolation.unwrap_or(false) as u8,
                    sinusoidal_interpolation: config.sinusoidal_interpolation.unwrap_or(false)
                        as u8,
                    sinusoidal_interpolation_hintless: config
                        .sinusoidal_interpolation_hintless
                        .unwrap_or(false) as u8,
                    clamp_velocity: config.clamp_velocity.unwrap_or(false) as u8,
                    skip_cinematic: config.skip_cinematic.unwrap_or(false) as u8,
                    no_elevation_interp: config.no_elevation_interp.unwrap_or(false) as u8,
                    direct_elevation: config.direct_elevation.unwrap_or(false) as u8,
                    override_look_dir: config.override_look_dir.unwrap_or(false) as u8,
                    no_elevation_vel_clamp: config.no_elevation_vel_clamp.unwrap_or(false) as u8,
                    calculate_transform_from_prev_cam: config
                        .calculate_transform_from_prev_cam
                        .unwrap_or(false) as u8,
                    no_spline: config.no_spline.unwrap_or(false) as u8,
                    unknown21: config.unknown1.unwrap_or(false) as u8,
                    unknown22: config.unknown2.unwrap_or(false) as u8,
                }
                .into(),

                min_dist: structs::scly_props::structs::BoolFloat {
                    override_flags: config.override_min_dist.unwrap_or(false) as u8,
                    value: config.min_dist.unwrap_or(8.0) as f32,
                }
                .into(),
                max_dist: structs::scly_props::structs::BoolFloat {
                    override_flags: config.override_max_dist.unwrap_or(false) as u8,
                    value: config.max_dist.unwrap_or(8.0) as f32,
                }
                .into(),
                backwards_dist: structs::scly_props::structs::BoolFloat {
                    override_flags: config.override_backwards_dist.unwrap_or(false) as u8,
                    value: config.backwards_dist.unwrap_or(8.0) as f32,
                }
                .into(),

                look_at_offset: structs::scly_props::structs::BoolVec3 {
                    override_flags: config.override_look_at_offset.unwrap_or(false) as u8,
                    value: config.look_at_offset.unwrap_or([0.0, 0.0, 0.0]).into(),
                }
                .into(),
                chase_look_at_offset: structs::scly_props::structs::BoolVec3 {
                    override_flags: config.override_chase_look_at_offset.unwrap_or(false) as u8,
                    value: config
                        .chase_look_at_offset
                        .unwrap_or([0.0, 0.0, 0.0])
                        .into(),
                }
                .into(),

                ball_to_cam: config.ball_to_cam.unwrap_or([0.0, 0.0, 0.0]).into(),

                fov: structs::scly_props::structs::BoolFloat {
                    override_flags: config.override_fov.unwrap_or(false) as u8,
                    value: config.fov.unwrap_or(55.0) as f32,
                }
                .into(),

                attitude_range: structs::scly_props::structs::BoolFloat {
                    override_flags: config.override_attitude_range.unwrap_or(false) as u8,
                    value: config.attitude_range.unwrap_or(90.0) as f32,
                }
                .into(),

                azimuth_range: structs::scly_props::structs::BoolFloat {
                    override_flags: config.override_azimuth_range.unwrap_or(false) as u8,
                    value: config.azimuth_range.unwrap_or(90.0) as f32,
                }
                .into(),

                angle_per_second: structs::scly_props::structs::BoolFloat {
                    override_flags: config.override_angle_per_second.unwrap_or(false) as u8,
                    value: config.angle_per_second.unwrap_or(120.0) as f32,
                }
                .into(),

                clamp_vel_range: config.clamp_vel_range.unwrap_or(10.0) as f32,
                clamp_rot_range: config.clamp_rot_range.unwrap_or(120.0) as f32,

                elevation: structs::scly_props::structs::BoolFloat {
                    override_flags: config.override_elevation.unwrap_or(false) as u8,
                    value: config.elevation.unwrap_or(2.7) as f32,
                }
                .into(),

                interpolate_time: config.interpolate_time.unwrap_or(1.5) as f32,
                clamp_vel_time: config.clamp_vel_time.unwrap_or(2.0) as f32,
                control_interp_dur: config.control_interp_dur.unwrap_or(1.0) as f32,
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_camera_hint_mut().unwrap();

            property_data.behavior = config.behaviour as u32;

            if let Some(position) = config.position {
                property_data.position = position.into()
            }
            if let Some(rotation) = config.rotation {
                property_data.rotation = rotation.into()
            }
            if let Some(active) = config.active {
                property_data.active = active as u8
            }
            if let Some(priority) = config.priority {
                property_data.priority = priority as u32
            }
            if let Some(calculate_cam_pos) = config.calculate_cam_pos {
                property_data.camera_hint_params.calculate_cam_pos = calculate_cam_pos as u8
            }
            if let Some(chase_allowed) = config.chase_allowed {
                property_data.camera_hint_params.chase_allowed = chase_allowed as u8
            }
            if let Some(boost_allowed) = config.boost_allowed {
                property_data.camera_hint_params.boost_allowed = boost_allowed as u8
            }
            if let Some(obscure_avoidance) = config.obscure_avoidance {
                property_data.camera_hint_params.obscure_avoidance = obscure_avoidance as u8
            }
            if let Some(volume_collider) = config.volume_collider {
                property_data.camera_hint_params.volume_collider = volume_collider as u8
            }
            if let Some(apply_immediately) = config.apply_immediately {
                property_data.camera_hint_params.apply_immediately = apply_immediately as u8
            }
            if let Some(look_at_ball) = config.look_at_ball {
                property_data.camera_hint_params.look_at_ball = look_at_ball as u8
            }
            if let Some(hint_distance_selection) = config.hint_distance_selection {
                property_data.camera_hint_params.hint_distance_selection =
                    hint_distance_selection as u8
            }
            if let Some(hint_distance_self_pos) = config.hint_distance_self_pos {
                property_data.camera_hint_params.hint_distance_self_pos =
                    hint_distance_self_pos as u8
            }
            if let Some(control_interpolation) = config.control_interpolation {
                property_data.camera_hint_params.control_interpolation = control_interpolation as u8
            }
            if let Some(sinusoidal_interpolation) = config.sinusoidal_interpolation {
                property_data.camera_hint_params.sinusoidal_interpolation =
                    sinusoidal_interpolation as u8
            }
            if let Some(sinusoidal_interpolation_hintless) =
                config.sinusoidal_interpolation_hintless
            {
                property_data
                    .camera_hint_params
                    .sinusoidal_interpolation_hintless = sinusoidal_interpolation_hintless as u8
            }
            if let Some(clamp_velocity) = config.clamp_velocity {
                property_data.camera_hint_params.clamp_velocity = clamp_velocity as u8
            }
            if let Some(skip_cinematic) = config.skip_cinematic {
                property_data.camera_hint_params.skip_cinematic = skip_cinematic as u8
            }
            if let Some(no_elevation_interp) = config.no_elevation_interp {
                property_data.camera_hint_params.no_elevation_interp = no_elevation_interp as u8
            }
            if let Some(direct_elevation) = config.direct_elevation {
                property_data.camera_hint_params.direct_elevation = direct_elevation as u8
            }
            if let Some(override_look_dir) = config.override_look_dir {
                property_data.camera_hint_params.override_look_dir = override_look_dir as u8
            }
            if let Some(no_elevation_vel_clamp) = config.no_elevation_vel_clamp {
                property_data.camera_hint_params.no_elevation_vel_clamp =
                    no_elevation_vel_clamp as u8
            }
            if let Some(calculate_transform_from_prev_cam) =
                config.calculate_transform_from_prev_cam
            {
                property_data
                    .camera_hint_params
                    .calculate_transform_from_prev_cam = calculate_transform_from_prev_cam as u8
            }
            if let Some(no_spline) = config.no_spline {
                property_data.camera_hint_params.no_spline = no_spline as u8
            }
            if let Some(unknown1) = config.unknown1 {
                property_data.camera_hint_params.unknown21 = unknown1 as u8
            }
            if let Some(unknown2) = config.unknown2 {
                property_data.camera_hint_params.unknown22 = unknown2 as u8
            }
            if let Some(override_min_dist) = config.override_min_dist {
                property_data.min_dist.override_flags = override_min_dist as u8
            }
            if let Some(min_dist) = config.min_dist {
                property_data.min_dist.value = min_dist as f32
            }
            if let Some(override_max_dist) = config.override_max_dist {
                property_data.max_dist.override_flags = override_max_dist as u8
            }
            if let Some(max_dist) = config.max_dist {
                property_data.max_dist.value = max_dist as f32
            }
            if let Some(override_backwards_dist) = config.override_backwards_dist {
                property_data.backwards_dist.override_flags = override_backwards_dist as u8
            }
            if let Some(backwards_dist) = config.backwards_dist {
                property_data.backwards_dist.value = backwards_dist as f32
            }
            if let Some(override_look_at_offset) = config.override_look_at_offset {
                property_data.look_at_offset.override_flags = override_look_at_offset as u8
            }
            if let Some(look_at_offset) = config.look_at_offset {
                property_data.look_at_offset.value = look_at_offset.into()
            }
            if let Some(override_chase_look_at_offset) = config.override_chase_look_at_offset {
                property_data.chase_look_at_offset.override_flags =
                    override_chase_look_at_offset as u8
            }
            if let Some(chase_look_at_offset) = config.chase_look_at_offset {
                property_data.chase_look_at_offset.value = chase_look_at_offset.into()
            }
            if let Some(ball_to_cam) = config.ball_to_cam {
                property_data.ball_to_cam = ball_to_cam.into()
            }
            if let Some(override_fov) = config.override_fov {
                property_data.fov.override_flags = override_fov as u8
            }
            if let Some(fov) = config.fov {
                property_data.fov.value = fov as f32
            }
            if let Some(override_attitude_range) = config.override_attitude_range {
                property_data.attitude_range.override_flags = override_attitude_range as u8
            }
            if let Some(attitude_range) = config.attitude_range {
                property_data.attitude_range.value = attitude_range as f32
            }
            if let Some(override_azimuth_range) = config.override_azimuth_range {
                property_data.azimuth_range.override_flags = override_azimuth_range as u8
            }
            if let Some(azimuth_range) = config.azimuth_range {
                property_data.azimuth_range.value = azimuth_range as f32
            }
            if let Some(override_angle_per_second) = config.override_angle_per_second {
                property_data.angle_per_second.override_flags = override_angle_per_second as u8
            }
            if let Some(angle_per_second) = config.angle_per_second {
                property_data.angle_per_second.value = angle_per_second as f32
            }
            if let Some(clamp_vel_range) = config.clamp_vel_range {
                property_data.clamp_vel_range = clamp_vel_range as f32
            }
            if let Some(clamp_rot_range) = config.clamp_rot_range {
                property_data.clamp_rot_range = clamp_rot_range as f32
            }
            if let Some(override_elevation) = config.override_elevation {
                property_data.elevation.override_flags = override_elevation as u8
            }
            if let Some(elevation) = config.elevation {
                property_data.elevation.value = elevation as f32
            }
            if let Some(interpolate_time) = config.interpolate_time {
                property_data.interpolate_time = interpolate_time as f32
            }
            if let Some(clamp_vel_time) = config.clamp_vel_time {
                property_data.clamp_vel_time = clamp_vel_time as f32
            }
            if let Some(control_interp_dur) = config.control_interp_dur {
                property_data.control_interp_dur = control_interp_dur as f32
            }
        };
    }

    add_edit_obj_helper!(area, Some(config.id), config.layer, CameraHint, new, update);
}

pub fn patch_add_camera_hint_trigger(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: CameraHintTriggerConfig,
) -> Result<(), String> {
    macro_rules! new {
        () => {
            structs::CameraHintTrigger {
                name: b"my camerahinttrigger\0".as_cstr(),
                position: config.position.unwrap_or([0.0, 0.0, 0.0]).into(),
                rotation: config.rotation.unwrap_or([0.0, 0.0, 0.0]).into(),
                scale: config.scale.unwrap_or([5.0, 5.0, 5.0]).into(),
                active: config.active.unwrap_or(true) as u8,
                deactivate_on_enter: config.deactivate_on_enter.unwrap_or(false) as u8,
                deactivate_on_exit: config.deactivate_on_exit.unwrap_or(false) as u8,
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_camera_hint_trigger_mut().unwrap();

            if let Some(position) = config.position {
                property_data.position = position.into()
            }
            if let Some(rotation) = config.rotation {
                property_data.rotation = rotation.into()
            }
            if let Some(scale) = config.scale {
                property_data.scale = scale.into()
            }
            if let Some(active) = config.active {
                property_data.active = active as u8
            }
            if let Some(deactivate_on_enter) = config.deactivate_on_enter {
                property_data.deactivate_on_enter = deactivate_on_enter as u8
            }
            if let Some(deactivate_on_exit) = config.deactivate_on_exit {
                property_data.deactivate_on_exit = deactivate_on_exit as u8
            }
        };
    }

    add_edit_obj_helper!(
        area,
        config.id,
        config.layer,
        CameraHintTrigger,
        new,
        update
    );
}

pub fn patch_add_platform<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    config: PlatformConfig,
) -> Result<(), String> {
    let platform_type = {
        match config.platform_type {
            Some(platform_type) => platform_type,
            None => {
                if config.alt_platform.unwrap_or(false) {
                    PlatformType::Snow
                } else {
                    PlatformType::Metal
                }
            }
        }
    };

    let ids = match platform_type {
        PlatformType::BombBox => {
            let mut ids = vec![];
            let layer = config.layer.unwrap_or(0) as usize;
            for _ in 0..8 {
                ids.push(area.new_object_id_from_layer_id(layer));
            }
            Some(ids)
        }
        _ => None,
    };

    let undamaged_block_id = match config.id {
        Some(id) => id,
        None => area.new_object_id_from_layer_id(config.layer.unwrap_or(0) as usize),
    };

    let vulnerability = match platform_type {
        PlatformType::BombBox => DoorType::Bomb.vulnerability(),
        _ => DoorType::Disabled.vulnerability(),
    };

    let connections = match platform_type {
        PlatformType::BombBox => {
            let ids = ids.as_ref().unwrap();

            let relay_block_switch_id = ids[3];
            let relay_kill_block_id = ids[4];
            let sound_id = ids[5];

            vec![
                structs::Connection {
                    state: structs::ConnectionState::DEAD,
                    message: structs::ConnectionMsg::ACTIVATE,
                    target_object_id: sound_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::DEAD,
                    message: structs::ConnectionMsg::SET_TO_ZERO,
                    target_object_id: relay_block_switch_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::DEAD,
                    message: structs::ConnectionMsg::SET_TO_ZERO,
                    target_object_id: relay_kill_block_id,
                },
            ]
        }
        _ => vec![],
    };

    let (deps, cmdl, dcln) = {
        match platform_type {
            PlatformType::Snow => (
                vec![
                    (0xDCDFD386, b"CMDL"),
                    (0x6D412D11, b"DCLN"),
                    (0xEED972E7, b"TXTR"),
                    (0xF1478D6A, b"TXTR"),
                    (0xF89D34EF, b"TXTR"),
                ],
                ResId::<res_id::CMDL>::new(0xDCDFD386),
                ResId::<res_id::DCLN>::new(0x6D412D11),
            ),
            PlatformType::Metal => (
                vec![
                    (0x48DF38A3, b"CMDL"),
                    (0xB2D50628, b"DCLN"),
                    (0x19C17D5C, b"TXTR"),
                    (0x0259F5F6, b"TXTR"),
                    (0x71190250, b"TXTR"),
                    (0xD0BA0FA8, b"TXTR"),
                    (0xF1478D6A, b"TXTR"),
                ],
                ResId::<res_id::CMDL>::new(0x48DF38A3),
                ResId::<res_id::DCLN>::new(0xB2D50628),
            ),
            PlatformType::BombBox => {
                (
                    vec![
                        (0x09D55763, b"CMDL"),
                        (0x133336F4, b"CMDL"),
                        (0x00F75174, b"TXTR"),
                        (0x123A70A6, b"TXTR"),
                        (0xB3A153C0, b"TXTR"),
                        (0x57fe7e67, b"AGSC"), // Misc.AGSC
                    ],
                    ResId::<res_id::CMDL>::new(0x09D55763),
                    ResId::invalid(),
                )
            }
            PlatformType::Block => (
                vec![
                    (0x27D0663B, b"CMDL"),
                    (0x964E98AC, b"DCLN"),
                    (0x19AD934F, b"TXTR"),
                    (0xFF6F41A6, b"TXTR"),
                ],
                ResId::<res_id::CMDL>::new(0x27D0663B),
                ResId::<res_id::DCLN>::new(0x964E98AC),
            ),
            PlatformType::HalfBlock => (
                vec![
                    (0x27D0663B, b"CMDL"),
                    (0x910FF59C, b"DCLN"),
                    (0x19AD934F, b"TXTR"),
                    (0xFF6F41A6, b"TXTR"),
                ],
                ResId::<res_id::CMDL>::new(0x27D0663B),
                ResId::<res_id::DCLN>::new(0x910FF59C),
            ),
            PlatformType::LongBlock => (
                vec![
                    (0x27D0663B, b"CMDL"),
                    (0xA87758DC, b"DCLN"),
                    (0x19AD934F, b"TXTR"),
                    (0xFF6F41A6, b"TXTR"),
                ],
                ResId::<res_id::CMDL>::new(0x27D0663B),
                ResId::<res_id::DCLN>::new(0xA87758DC),
            ),
            PlatformType::Empty => {
                (
                    vec![
                        // Magma Pool Jump Blocker (invis)
                        (0x3801DE98, b"CMDL"),
                        (0xB3048E27, b"TXTR"),
                        // Empty DCLN
                        (0xF4BEE243, b"DCLN"),
                    ],
                    ResId::<res_id::CMDL>::new(0x3801DE98),
                    ResId::<res_id::DCLN>::new(0xF4BEE243),
                )
            }
        }
    };

    let scale = match platform_type {
        PlatformType::HalfBlock => [1.0, 1.0, 0.5],
        PlatformType::LongBlock => [2.0, 1.0, 0.5],
        _ => [1.0, 1.0, 1.0],
    };

    let deps_iter = deps.iter().map(|&(file_id, fourcc)| structs::Dependency {
        asset_id: file_id,
        asset_type: FourCC::from_bytes(fourcc),
    });
    area.add_dependencies(game_resources, 0, deps_iter);

    macro_rules! new {
        () => {
            structs::Platform {
                name: b"myplatform\0".as_cstr(),

                position: config.position.into(),
                rotation: config.rotation.unwrap_or([0.0, 0.0, 0.0]).into(),
                scale: scale.into(),
                extent: [0.0, 0.0, 0.0].into(),
                scan_offset: [0.0, 0.0, 0.0].into(),

                cmdl,
                ancs: structs::scly_structs::AncsProp {
                    file_id: ResId::invalid(),
                    node_index: 0,
                    default_animation: 0xFFFFFFFF,
                },
                actor_params: structs::scly_structs::ActorParameters {
                    light_params: structs::scly_structs::LightParameters {
                        unknown0: 1,
                        unknown1: 1.0,
                        shadow_tessellation: 0,
                        unknown2: 1.0,
                        unknown3: 20.0,
                        color: [1.0, 1.0, 1.0, 1.0].into(),
                        unknown4: 1,
                        world_lighting: 1,
                        light_recalculation: 1,
                        unknown5: [0.0, 0.0, 0.0].into(),
                        unknown6: 4,
                        unknown7: 4,
                        unknown8: 0,
                        light_layer_id: 0,
                    },
                    scan_params: structs::scly_structs::ScannableParameters {
                        scan: ResId::invalid(), // None
                    },
                    xray_cmdl: ResId::invalid(),    // None
                    xray_cskr: ResId::invalid(),    // None
                    thermal_cmdl: ResId::invalid(), // None
                    thermal_cskr: ResId::invalid(), // None

                    unknown0: 1,
                    unknown1: 1.0,
                    unknown2: 1.0,

                    visor_params: structs::scly_structs::VisorParameters {
                        unknown0: 0,
                        target_passthrough: 0,
                        visor_mask: 15, // Combat|Scan|Thermal|XRay
                    },
                    enable_thermal_heat: 1,
                    unknown3: 0,
                    unknown4: 0,
                    unknown5: 1.0,
                },

                speed: 5.0,
                active: config.active.unwrap_or(true) as u8,

                dcln,

                health_info: structs::scly_structs::HealthInfo {
                    health: 1.0,
                    knockback_resistance: 1.0,
                },
                damage_vulnerability: vulnerability.clone(),

                detect_collision: 0,
                unknown4: 1.0,
                unknown5: 0,
                unknown6: 200,
                unknown7: 20,
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_platform_mut().unwrap();

            if config.platform_type.is_some() {
                property_data.cmdl = cmdl;
                property_data.dcln = dcln;
            }

            property_data.position = config.position.into();

            if let Some(rotation) = config.rotation {
                property_data.rotation = rotation.into();
            }

            if let Some(active) = config.active {
                property_data.active = active as u8;
            }
        };
    }

    if platform_type == PlatformType::BombBox {
        let layer_id = config.layer.unwrap_or(0) as usize;
        while area.layer_flags.layer_count <= layer_id as u32 {
            area.add_layer(b"New Layer\0".as_cstr());
        }

        let scly = area.mrea().scly_section_mut();
        let objects = scly.layers.as_mut_vec()[layer_id].objects.as_mut_vec();

        let ids = ids.unwrap();

        let damaged_block_id = ids[0];
        let timer_fade_in_id = ids[1];
        let timer_restore_block_id = ids[2];
        let relay_block_switch_id = ids[3];
        let relay_kill_block_id = ids[4];
        let sound_id = ids[5];
        let relay_restore_block_id = ids[6];
        let trigger_id = ids[7];

        objects.extend_from_slice(&[
            structs::SclyObject {
                instance_id: damaged_block_id,
                property_data: structs::Platform {
                    name: b"myplatform\0".as_cstr(),

                    position: config.position.into(),
                    rotation: config.rotation.unwrap_or([0.0, 0.0, 0.0]).into(),
                    scale: [1.0, 1.0, 1.0].into(),
                    extent: [0.0, 0.0, 0.0].into(),
                    scan_offset: [0.0, 0.0, 0.0].into(),

                    cmdl: ResId::<res_id::CMDL>::new(0x133336F4),

                    ancs: structs::scly_structs::AncsProp {
                        file_id: ResId::invalid(),
                        node_index: 0,
                        default_animation: 0xFFFFFFFF,
                    },
                    actor_params: structs::scly_structs::ActorParameters {
                        light_params: structs::scly_structs::LightParameters {
                            unknown0: 1,
                            unknown1: 1.0,
                            shadow_tessellation: 0,
                            unknown2: 1.0,
                            unknown3: 20.0,
                            color: [1.0, 1.0, 1.0, 1.0].into(),
                            unknown4: 1,
                            world_lighting: 1,
                            light_recalculation: 1,
                            unknown5: [0.0, 0.0, 0.0].into(),
                            unknown6: 4,
                            unknown7: 4,
                            unknown8: 0,
                            light_layer_id: 0,
                        },
                        scan_params: structs::scly_structs::ScannableParameters {
                            scan: ResId::invalid(), // None
                        },
                        xray_cmdl: ResId::invalid(),    // None
                        xray_cskr: ResId::invalid(),    // None
                        thermal_cmdl: ResId::invalid(), // None
                        thermal_cskr: ResId::invalid(), // None

                        unknown0: 1,
                        unknown1: 1.0,
                        unknown2: 1.0,

                        visor_params: structs::scly_structs::VisorParameters {
                            unknown0: 0,
                            target_passthrough: 0,
                            visor_mask: 15, // Combat|Scan|Thermal|XRay
                        },
                        enable_thermal_heat: 1,
                        unknown3: 0,
                        unknown4: 0,
                        unknown5: 1.0,
                    },

                    speed: 5.0,
                    active: 0,

                    dcln,

                    health_info: structs::scly_structs::HealthInfo {
                        health: 1.0,
                        knockback_resistance: 1.0,
                    },
                    damage_vulnerability: vulnerability.clone(),

                    detect_collision: 0,
                    unknown4: 1.0,
                    unknown5: 0,
                    unknown6: 200,
                    unknown7: 20,
                }
                .into(),
                connections: vec![
                    structs::Connection {
                        state: structs::ConnectionState::DEAD,
                        message: structs::ConnectionMsg::PLAY,
                        target_object_id: sound_id,
                    },
                    structs::Connection {
                        state: structs::ConnectionState::DEAD,
                        message: structs::ConnectionMsg::SET_TO_ZERO,
                        target_object_id: relay_kill_block_id,
                    },
                    structs::Connection {
                        state: structs::ConnectionState::INACTIVE,
                        message: structs::ConnectionMsg::DEACTIVATE,
                        target_object_id: relay_kill_block_id,
                    },
                ]
                .into(),
            },
            structs::SclyObject {
                instance_id: timer_fade_in_id,
                property_data: structs::Timer {
                    name: b"timer fade in\0".as_cstr(),
                    start_time: 4.0,
                    max_random_add: 0.0,
                    looping: 1,
                    start_immediately: 0,
                    active: 1,
                }
                .into(),
                connections: vec![
                    structs::Connection {
                        state: structs::ConnectionState::ZERO,
                        message: structs::ConnectionMsg::RESET,
                        target_object_id: damaged_block_id,
                    },
                    structs::Connection {
                        state: structs::ConnectionState::ZERO,
                        message: structs::ConnectionMsg::RESET,
                        target_object_id: undamaged_block_id,
                    },
                    structs::Connection {
                        state: structs::ConnectionState::ZERO,
                        message: structs::ConnectionMsg::ACTIVATE,
                        target_object_id: timer_restore_block_id,
                    },
                ]
                .into(),
            },
            structs::SclyObject {
                instance_id: timer_restore_block_id,
                property_data: structs::Timer {
                    name: b"my timer\0".as_cstr(),
                    start_time: 0.02,
                    max_random_add: 0.0,
                    looping: 1,
                    start_immediately: 1,
                    active: 0,
                }
                .into(),
                connections: vec![structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::SET_TO_ZERO,
                    target_object_id: relay_restore_block_id,
                }]
                .into(),
            },
            structs::SclyObject {
                instance_id: relay_block_switch_id,
                property_data: structs::Relay {
                    name: b"relay block switch relay\0".as_cstr(),
                    active: 1,
                }
                .into(),
                connections: vec![
                    structs::Connection {
                        state: structs::ConnectionState::ZERO,
                        message: structs::ConnectionMsg::DEACTIVATE,
                        target_object_id: undamaged_block_id,
                    },
                    structs::Connection {
                        state: structs::ConnectionState::ZERO,
                        message: structs::ConnectionMsg::ACTIVATE,
                        target_object_id: damaged_block_id,
                    },
                    structs::Connection {
                        state: structs::ConnectionState::ZERO,
                        message: structs::ConnectionMsg::ACTIVATE,
                        target_object_id: relay_kill_block_id,
                    },
                ]
                .into(),
            },
            structs::SclyObject {
                instance_id: relay_kill_block_id,
                property_data: structs::Relay {
                    name: b"relay kill block\0".as_cstr(),
                    active: 0,
                }
                .into(),
                connections: vec![
                    structs::Connection {
                        state: structs::ConnectionState::ZERO,
                        message: structs::ConnectionMsg::DEACTIVATE,
                        target_object_id: damaged_block_id,
                    },
                    structs::Connection {
                        state: structs::ConnectionState::ZERO,
                        message: structs::ConnectionMsg::DEACTIVATE,
                        target_object_id: undamaged_block_id,
                    },
                    structs::Connection {
                        state: structs::ConnectionState::ZERO,
                        message: structs::ConnectionMsg::RESET_AND_START,
                        target_object_id: timer_fade_in_id,
                    },
                ]
                .into(),
            },
            structs::SclyObject {
                instance_id: relay_restore_block_id,
                property_data: structs::Relay {
                    name: b"relay restore block\0".as_cstr(),
                    active: 1,
                }
                .into(),
                connections: vec![
                    structs::Connection {
                        state: structs::ConnectionState::ZERO,
                        message: structs::ConnectionMsg::DEACTIVATE,
                        target_object_id: timer_restore_block_id,
                    },
                    structs::Connection {
                        state: structs::ConnectionState::ZERO,
                        message: structs::ConnectionMsg::INCREMENT,
                        target_object_id: undamaged_block_id,
                    },
                ]
                .into(),
            },
            structs::SclyObject {
                instance_id: sound_id,
                property_data: structs::Sound {
                    name: b"mysound\0".as_cstr(),
                    position: config.position.into(),
                    rotation: [0.0, 0.0, 0.0].into(),
                    sound_id: 3510,
                    active: 1,
                    max_dist: 150.0,
                    dist_comp: 0.2,
                    start_delay: 0.0,
                    min_volume: 20,
                    volume: 127,
                    priority: 127,
                    pan: 64,
                    loops: 0,
                    non_emitter: 0,
                    auto_start: 0,
                    occlusion_test: 0,
                    acoustics: 1,
                    world_sfx: 0,
                    allow_duplicates: 0,
                    pitch: 0,
                }
                .into(),
                connections: vec![].into(),
            },
            structs::SclyObject {
                instance_id: trigger_id,
                property_data: structs::Trigger {
                    name: b"camerahinttrigger\0".as_cstr(),
                    position: config.position.into(),
                    scale: [1.899002, 1.898987, 1.299].into(),
                    damage_info: structs::scly_structs::DamageInfo {
                        weapon_type: 0,
                        damage: 0.0,
                        radius: 0.0,
                        knockback_power: 0.0,
                    },
                    force: [0.0, 0.0, 0.0].into(),
                    flags: 1,
                    active: 1,
                    deactivate_on_enter: 0,
                    deactivate_on_exit: 0,
                }
                .into(),
                connections: vec![
                    structs::Connection {
                        state: structs::ConnectionState::ENTERED,
                        message: structs::ConnectionMsg::DEACTIVATE,
                        target_object_id: relay_restore_block_id,
                    },
                    structs::Connection {
                        state: structs::ConnectionState::EXITED,
                        message: structs::ConnectionMsg::ACTIVATE,
                        target_object_id: relay_restore_block_id,
                    },
                ]
                .into(),
            },
        ]);
    }

    let id = config.id;
    let requested_layer_id = config.layer;
    let mrea_id = area.mlvl_area.mrea.to_u32();

    // add more layers as needed
    if let Some(requested_layer_id) = requested_layer_id {
        while area.layer_flags.layer_count <= requested_layer_id {
            area.add_layer(b"New Layer\0".as_cstr());
        }
    }

    if let Some(id) = id {
        let scly = area.mrea().scly_section_mut();

        // try to find existing object
        let info = {
            let mut info = None;

            let layer_count = scly.layers.as_mut_vec().len();
            for _layer_id in 0..layer_count {
                let layer = scly.layers.iter().nth(_layer_id).unwrap();

                let obj = layer
                    .objects
                    .iter()
                    .find(|obj| obj.instance_id & 0x00FFFFFF == id & 0x00FFFFFF);

                if let Some(obj) = obj {
                    if obj.property_data.object_type() != structs::Platform::OBJECT_TYPE {
                        panic!("Failed to edit existing object 0x{:X} in room 0x{:X}: Unexpected object type 0x{:X} (expected 0x{:X})", id, mrea_id, obj.property_data.object_type(), structs::Platform::OBJECT_TYPE);
                    }

                    info = Some((_layer_id as u32, obj.instance_id));
                    break;
                }
            }

            info
        };

        if let Some(info) = info {
            let (layer_id, _) = info;

            // move and update
            if requested_layer_id.is_some() && requested_layer_id.unwrap() != layer_id {
                let requested_layer_id = requested_layer_id.unwrap();

                // clone existing object
                let mut obj = scly.layers.as_mut_vec()[layer_id as usize]
                    .objects
                    .as_mut_vec()
                    .iter_mut()
                    .find(|obj| obj.instance_id & 0x00FFFFFF == id & 0x00FFFFFF)
                    .unwrap()
                    .clone();

                // modify it
                update!(obj);

                // remove original
                scly.layers.as_mut_vec()[layer_id as usize]
                    .objects
                    .as_mut_vec()
                    .retain(|obj| obj.instance_id & 0x00FFFFFF != id & 0x00FFFFFF);

                // re-add to target layer
                scly.layers.as_mut_vec()[requested_layer_id as usize]
                    .objects
                    .as_mut_vec()
                    .push(obj);

                return Ok(());
            }

            // get mutable reference to existing object
            let obj = scly.layers.as_mut_vec()[layer_id as usize]
                .objects
                .as_mut_vec()
                .iter_mut()
                .find(|obj| obj.instance_id & 0x00FFFFFF == id & 0x00FFFFFF)
                .unwrap();

            // update it
            update!(obj);

            return Ok(());
        }
    }

    // add new object
    let id = id.unwrap_or(undamaged_block_id);

    let scly = area.mrea().scly_section_mut();
    let layers = &mut scly.layers.as_mut_vec();
    let objects = layers[requested_layer_id.unwrap_or(0) as usize]
        .objects
        .as_mut_vec();
    let property_data = new!();
    let property_data: structs::SclyProperty = property_data.into();

    assert!(property_data.object_type() == structs::Platform::OBJECT_TYPE);

    objects.push(structs::SclyObject {
        instance_id: id,
        property_data,
        connections: connections.into(),
    });

    Ok(())
}

pub fn patch_add_block<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    config: BlockConfig,
    old_scale: bool,
) -> Result<(), String> {
    let texture = config.texture.unwrap_or(GenericTexture::Grass);

    let deps = [
        (texture.cmdl().to_u32(), b"CMDL"),
        (texture.txtr().to_u32(), b"TXTR"),
    ];
    let deps_iter = deps.iter().map(|&(file_id, fourcc)| structs::Dependency {
        asset_id: file_id,
        asset_type: FourCC::from_bytes(fourcc),
    });
    area.add_dependencies(game_resources, 0, deps_iter);

    add_block(
        area,
        config.id,
        config.position,
        config.scale.unwrap_or([1.0, 1.0, 1.0]),
        texture,
        1,
        config.layer,
        config.active.unwrap_or(true),
        old_scale,
    );

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn add_block(
    area: &mut mlvl_wrapper::MlvlArea,
    id: Option<u32>,
    position: [f32; 3],
    scale: [f32; 3],
    texture: GenericTexture,
    is_tangible: u8,
    layer: Option<u32>,
    active: bool,
    old_scale: bool,
) {
    let layer_id = layer.unwrap_or(0);

    let scale = match old_scale {
        true => scale,
        false => [scale[0] * 0.587, scale[1] * 0.587, scale[2] * 0.587],
    };

    let actor_id = match id {
        Some(id) => id,
        None => area.new_object_id_from_layer_id(layer_id as usize),
    };

    while area.layer_flags.layer_count <= layer_id {
        area.add_layer(b"New Layer\0".as_cstr());
    }

    let scly = area.mrea().scly_section_mut();
    let objects = &mut scly.layers.as_mut_vec()[layer_id as usize]
        .objects
        .as_mut_vec();

    objects.push(structs::SclyObject {
        instance_id: actor_id,
        property_data: structs::Actor {
            name: b"myactor\0".as_cstr(),
            position: position.into(),
            rotation: [0.0, 0.0, 0.0].into(),
            scale: scale.into(),
            hitbox: [0.0, 0.0, 0.0].into(),
            scan_offset: [0.0, 0.0, 0.0].into(),
            unknown1: 1.0,
            unknown2: 0.0,
            health_info: structs::scly_structs::HealthInfo {
                health: 5.0,
                knockback_resistance: 1.0,
            },
            damage_vulnerability: DoorType::Disabled.vulnerability(),
            cmdl: texture.cmdl(),
            ancs: structs::scly_structs::AncsProp {
                file_id: ResId::invalid(), // None
                node_index: 0,
                default_animation: 0xFFFFFFFF, // -1
            },
            actor_params: structs::scly_structs::ActorParameters {
                light_params: structs::scly_structs::LightParameters {
                    unknown0: 1,
                    unknown1: 1.0,
                    shadow_tessellation: 0,
                    unknown2: 1.0,
                    unknown3: 20.0,
                    color: [1.0, 1.0, 1.0, 1.0].into(),
                    unknown4: 1,
                    world_lighting: 1,
                    light_recalculation: 1,
                    unknown5: [0.0, 0.0, 0.0].into(),
                    unknown6: 4,
                    unknown7: 4,
                    unknown8: 0,
                    light_layer_id: 0,
                },
                scan_params: structs::scly_structs::ScannableParameters {
                    scan: ResId::invalid(), // None
                },
                xray_cmdl: ResId::invalid(),    // None
                xray_cskr: ResId::invalid(),    // None
                thermal_cmdl: ResId::invalid(), // None
                thermal_cskr: ResId::invalid(), // None

                unknown0: 1,
                unknown1: 1.0,
                unknown2: 1.0,

                visor_params: structs::scly_structs::VisorParameters {
                    unknown0: 0,
                    target_passthrough: 0,
                    visor_mask: 15, // Combat|Scan|Thermal|XRay
                },
                enable_thermal_heat: 1,
                unknown3: 0,
                unknown4: 0,
                unknown5: 1.0,
            },
            looping: 1,
            snow: 1,
            solid: is_tangible,
            camera_passthrough: 0,
            active: active as u8,
            unknown8: 0,
            unknown9: 1.0,
            unknown10: 1,
            unknown11: 0,
            unknown12: 0,
            unknown13: 0,
        }
        .into(),
        connections: vec![].into(),
    });
}

pub fn patch_lock_on_point<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    config: LockOnPoint,
) -> Result<(), String> {
    let deps = [
        (0xBFE4DAA0, b"CMDL"),
        (0x57C7107D, b"TXTR"),
        (0xE580D665, b"TXTR"),
    ];
    let deps_iter = deps.iter().map(|&(file_id, fourcc)| structs::Dependency {
        asset_id: file_id,
        asset_type: FourCC::from_bytes(fourcc),
    });
    area.add_dependencies(game_resources, 0, deps_iter);

    let is_grapple = config.is_grapple.unwrap_or(false);
    let no_lock = config.no_lock.unwrap_or(false);
    let position = config.position;
    let layer = config.layer.unwrap_or(0) as usize;

    if is_grapple {
        let deps = [
            (0x3abe45a6, b"SCAN"),
            (0x191a6881, b"STRG"),
            (0x748c37a5, b"SCAN"),
            (0x50ac3b9a, b"STRG"),
            (0xA482DBD1, b"TXTR"),
            (0xC9A36445, b"TXTR"),
            (0x2702E5E0, b"TXTR"),
            (0x34E79314, b"TXTR"),
            (0x46434ED3, b"TXTR"),
            (0x4F944876, b"TXTR"),
        ];
        let deps_iter = deps.iter().map(|&(file_id, fourcc)| structs::Dependency {
            asset_id: file_id,
            asset_type: FourCC::from_bytes(fourcc),
        });
        area.add_dependencies(game_resources, 0, deps_iter);
    }

    let actor_id = config
        .id1
        .unwrap_or(area.new_object_id_from_layer_id(layer));
    let mut grapple_point_id = 0;
    let mut special_function_id = 0;
    let mut timer_id = 0;
    let mut poi_pre_id = 0;
    let mut poi_post_id = 0;
    let mut damageable_trigger_id = 0;
    let mut add_scan_point = false;

    if is_grapple {
        grapple_point_id = config
            .id2
            .unwrap_or(area.new_object_id_from_layer_id(layer));
        add_scan_point = true; // We don't actually need the scan points, just their assets. Could save on objects by making this false via config
        if add_scan_point {
            special_function_id = area.new_object_id_from_layer_id(layer);
            timer_id = area.new_object_id_from_layer_id(layer);
            poi_pre_id = area.new_object_id_from_layer_id(layer);
            poi_post_id = area.new_object_id_from_layer_id(layer);
        }
    } else if !no_lock {
        damageable_trigger_id = config
            .id2
            .unwrap_or(area.new_object_id_from_layer_id(layer));
    }

    let layers = area.mrea().scly_section_mut().layers.as_mut_vec();
    layers[layer]
        .objects
        .as_mut_vec()
        .push(structs::SclyObject {
            instance_id: actor_id,
            property_data: structs::Actor {
                name: b"myactor\0".as_cstr(),
                position: position.into(),
                rotation: [0.0, 0.0, 0.0].into(),
                scale: [8.0, 8.0, 8.0].into(),
                hitbox: [0.0, 0.0, 0.0].into(),
                scan_offset: [0.0, 0.0, 0.0].into(),
                unknown1: 1.0,
                unknown2: 0.0,
                health_info: structs::scly_structs::HealthInfo {
                    health: 5.0,
                    knockback_resistance: 1.0,
                },
                damage_vulnerability: DoorType::Disabled.vulnerability(),
                cmdl: ResId::<res_id::CMDL>::new(0xBFE4DAA0),
                ancs: structs::scly_structs::AncsProp {
                    file_id: ResId::invalid(),
                    node_index: 0,
                    default_animation: 0xFFFFFFFF,
                },
                actor_params: structs::scly_structs::ActorParameters {
                    light_params: structs::scly_structs::LightParameters {
                        unknown0: 1,
                        unknown1: 1.0,
                        shadow_tessellation: 0,
                        unknown2: 1.0,
                        unknown3: 20.0,
                        color: [1.0, 1.0, 1.0, 1.0].into(),
                        unknown4: 1,
                        world_lighting: 1,
                        light_recalculation: 1,
                        unknown5: [0.0, 0.0, 0.0].into(),
                        unknown6: 4,
                        unknown7: 4,
                        unknown8: 0,
                        light_layer_id: 0,
                    },
                    scan_params: structs::scly_structs::ScannableParameters {
                        scan: ResId::invalid(), // None
                    },
                    xray_cmdl: ResId::invalid(),    // None
                    xray_cskr: ResId::invalid(),    // None
                    thermal_cmdl: ResId::invalid(), // None
                    thermal_cskr: ResId::invalid(), // None

                    unknown0: 1,
                    unknown1: 1.0,
                    unknown2: 1.0,

                    visor_params: structs::scly_structs::VisorParameters {
                        unknown0: 0,
                        target_passthrough: 1,
                        visor_mask: 15, // Combat|Scan|Thermal|XRay
                    },
                    enable_thermal_heat: 1,
                    unknown3: 0,
                    unknown4: 0,
                    unknown5: 1.0,
                },
                looping: 1,
                snow: 1,
                solid: 0,
                camera_passthrough: 1,
                active: config.active1.unwrap_or(true) as u8,
                unknown8: 0,
                unknown9: 1.0,
                unknown10: 1,
                unknown11: 0,
                unknown12: 0,
                unknown13: 0,
            }
            .into(),
            connections: vec![].into(),
        });

    if is_grapple {
        layers[layer]
            .objects
            .as_mut_vec()
            .push(structs::SclyObject {
                instance_id: grapple_point_id,
                property_data: structs::GrapplePoint {
                    name: b"my grapple point\0".as_cstr(),
                    position: [position[0], position[1], position[2] - 0.5].into(),
                    rotation: [0.0, -0.0, 0.0].into(),
                    active: 1,
                    grapple_params: structs::GrappleParams {
                        unknown1: 10.0,
                        unknown2: 10.0,
                        unknown3: 1.0,
                        unknown4: 1.0,
                        unknown5: 1.0,
                        unknown6: 1.0,
                        unknown7: 1.0,
                        unknown8: 45.0,
                        unknown9: 90.0,
                        unknown10: 0.0,
                        unknown11: 0.0,

                        disable_turning: 0,
                    },
                }
                .into(),
                connections: vec![].into(),
            });

        if add_scan_point {
            layers[layer]
                .objects
                .as_mut_vec()
                .push(structs::SclyObject {
                    instance_id: special_function_id,
                    connections: vec![
                        structs::Connection {
                            state: structs::ConnectionState::ZERO,
                            message: structs::ConnectionMsg::DEACTIVATE,
                            target_object_id: poi_pre_id,
                        },
                        structs::Connection {
                            state: structs::ConnectionState::ZERO,
                            message: structs::ConnectionMsg::ACTIVATE,
                            target_object_id: poi_post_id,
                        },
                    ]
                    .into(),
                    property_data: structs::SclyProperty::SpecialFunction(Box::new(
                        structs::SpecialFunction {
                            name: b"myspecialfun\0".as_cstr(),
                            position: position.into(),
                            rotation: [0.0, 0.0, 0.0].into(),
                            type_: 5, // inventory activator
                            unknown0: b"\0".as_cstr(),
                            unknown1: 0.0,
                            unknown2: 0.0,
                            unknown3: 0.0,
                            layer_change_room_id: 0xFFFFFFFF,
                            layer_change_layer_id: 0xFFFFFFFF,
                            item_id: 12, // grapple beam
                            unknown4: 1, // active
                            unknown5: 0.0,
                            unknown6: 0xFFFFFFFF,
                            unknown7: 0xFFFFFFFF,
                            unknown8: 0xFFFFFFFF,
                        },
                    )),
                });

            layers[layer]
                .objects
                .as_mut_vec()
                .push(structs::SclyObject {
                    instance_id: timer_id,
                    connections: vec![structs::Connection {
                        state: structs::ConnectionState::ZERO,
                        message: structs::ConnectionMsg::ACTION,
                        target_object_id: special_function_id,
                    }]
                    .into(),
                    property_data: structs::Timer {
                        name: b"grapple timer\0".as_cstr(),
                        start_time: 0.02,
                        max_random_add: 0.0,
                        looping: 0,
                        start_immediately: 1,
                        active: 1,
                    }
                    .into(),
                });

            layers[layer]
                .objects
                .as_mut_vec()
                .push(structs::SclyObject {
                    instance_id: poi_pre_id,
                    connections: vec![].into(),
                    property_data: structs::SclyProperty::PointOfInterest(Box::new(
                        structs::PointOfInterest {
                            name: b"mypoi\0".as_cstr(),
                            position: [position[0], position[1], position[2] - 0.5].into(),
                            rotation: [0.0, 0.0, 0.0].into(),
                            active: 1,
                            scan_param: structs::scly_structs::ScannableParameters {
                                scan: resource_info!("Grapple Point pre.SCAN").try_into().unwrap(),
                            },
                            point_size: 0.0,
                        },
                    )),
                });

            layers[layer]
                .objects
                .as_mut_vec()
                .push(structs::SclyObject {
                    instance_id: poi_post_id,
                    connections: vec![].into(),
                    property_data: structs::SclyProperty::PointOfInterest(Box::new(
                        structs::PointOfInterest {
                            name: b"mypoi\0".as_cstr(),
                            position: [position[0], position[1], position[2] - 0.5].into(),
                            rotation: [0.0, 0.0, 0.0].into(),
                            active: 0,
                            scan_param: structs::scly_structs::ScannableParameters {
                                scan: resource_info!("Grapple Point.SCAN").try_into().unwrap(),
                            },
                            point_size: 0.0,
                        },
                    )),
                });
        }
    } else if !no_lock {
        layers[layer]
            .objects
            .as_mut_vec()
            .push(structs::SclyObject {
                instance_id: damageable_trigger_id,
                property_data: structs::DamageableTrigger {
                    name: b"my dtrigger\0".as_cstr(),
                    position: position.into(),
                    scale: [0.001, 0.001, 0.001].into(),
                    health_info: structs::scly_structs::HealthInfo {
                        health: 9999999999.0,
                        knockback_resistance: 1.0,
                    },
                    damage_vulnerability: DoorType::Blue.vulnerability(),
                    unknown0: 0,
                    pattern_txtr0: ResId::invalid(),
                    pattern_txtr1: ResId::invalid(),
                    color_txtr: ResId::invalid(),
                    lock_on: 1,
                    active: config.active2.unwrap_or(true) as u8,
                    visor_params: structs::scly_structs::VisorParameters {
                        unknown0: 0,
                        target_passthrough: 0,
                        visor_mask: 15, // Combat|Scan|Thermal|XRay
                    },
                }
                .into(),
                connections: vec![].into(),
            });
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn patch_add_camera_hint(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    trigger_pos: [f32; 3],
    trigger_scale: [f32; 3],
    camera_pos: [f32; 3],
    camera_rot: [f32; 3],
    behavior: u32,
    layer: u32,
    camera_id: Option<u32>,
    trigger_id: Option<u32>,
) -> Result<(), String> {
    let layer = layer as usize;
    let camear_hint_id = camera_id.unwrap_or(area.new_object_id_from_layer_id(layer));
    let camera_hint_trigger_id = trigger_id.unwrap_or(area.new_object_id_from_layer_id(layer));

    let camera_objs = add_camera_hint(
        camear_hint_id,
        camera_hint_trigger_id,
        trigger_pos,
        trigger_scale,
        camera_pos,
        camera_rot,
        behavior,
    );

    area.mrea().scly_section_mut().layers.as_mut_vec()[layer]
        .objects
        .as_mut_vec()
        .extend_from_slice(&camera_objs);

    Ok(())
}

pub fn add_camera_hint<'r>(
    camear_hint_id: u32,
    camera_hint_trigger_id: u32,
    trigger_pos: [f32; 3],
    trigger_scale: [f32; 3],
    camera_pos: [f32; 3],
    camera_rot: [f32; 3],
    behavior: u32,
) -> Vec<structs::SclyObject<'r>> {
    let objects = vec![
        structs::SclyObject {
            instance_id: camear_hint_id,
            connections: vec![].into(),
            property_data: structs::SclyProperty::CameraHint(Box::new(structs::CameraHint {
                name: b"CameraHint\0".as_cstr(),
                position: camera_pos.into(),
                rotation: camera_rot.into(),
                active: 1,
                priority: 8,
                behavior,
                camera_hint_params: structs::scly_props::structs::CameraHintParameters {
                    calculate_cam_pos: 0,
                    chase_allowed: 0,
                    boost_allowed: 0,
                    obscure_avoidance: 0,
                    volume_collider: 0,
                    apply_immediately: 1,
                    look_at_ball: 1,
                    hint_distance_selection: 0,
                    hint_distance_self_pos: 1,
                    control_interpolation: 0,
                    sinusoidal_interpolation: 0,
                    sinusoidal_interpolation_hintless: 0,
                    clamp_velocity: 0,
                    skip_cinematic: 0,
                    no_elevation_interp: 0,
                    direct_elevation: 0,
                    override_look_dir: 1,
                    no_elevation_vel_clamp: 0,
                    calculate_transform_from_prev_cam: 1,
                    no_spline: 1,
                    unknown21: 0,
                    unknown22: 0,
                },
                min_dist: structs::scly_props::structs::BoolFloat {
                    override_flags: 0,
                    value: 8.0,
                },
                max_dist: structs::scly_props::structs::BoolFloat {
                    override_flags: 0,
                    value: 50.0,
                },
                backwards_dist: structs::scly_props::structs::BoolFloat {
                    override_flags: 0,
                    value: 8.0,
                },
                look_at_offset: structs::scly_props::structs::BoolVec3 {
                    override_flags: 0,
                    value: [0.0, 1.0, 1.0].into(),
                },
                chase_look_at_offset: structs::scly_props::structs::BoolVec3 {
                    override_flags: 0,
                    value: [0.0, 1.0, 1.0].into(),
                },
                ball_to_cam: [3.0, 3.0, 3.0].into(),
                fov: structs::scly_props::structs::BoolFloat {
                    override_flags: 0,
                    value: 55.0,
                },
                attitude_range: structs::scly_props::structs::BoolFloat {
                    override_flags: 0,
                    value: 90.0,
                },
                azimuth_range: structs::scly_props::structs::BoolFloat {
                    override_flags: 0,
                    value: 90.0,
                },
                angle_per_second: structs::scly_props::structs::BoolFloat {
                    override_flags: 0,
                    value: 120.0,
                },
                clamp_vel_range: 10.0,
                clamp_rot_range: 120.0,
                elevation: structs::scly_props::structs::BoolFloat {
                    override_flags: 0,
                    value: 2.7,
                },
                interpolate_time: 1.0,
                clamp_vel_time: 1.0,
                control_interp_dur: 1.0,
            })),
        },
        structs::SclyObject {
            instance_id: camera_hint_trigger_id,
            connections: vec![
                structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::INCREMENT,
                    target_object_id: camear_hint_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::EXITED,
                    message: structs::ConnectionMsg::DECREMENT,
                    target_object_id: camear_hint_id,
                },
            ]
            .into(),
            property_data: structs::SclyProperty::Trigger(Box::new(structs::Trigger {
                name: b"camerahinttrigger\0".as_cstr(),
                position: trigger_pos.into(),
                scale: trigger_scale.into(),
                damage_info: structs::scly_structs::DamageInfo {
                    weapon_type: 0,
                    damage: 0.0,
                    radius: 0.0,
                    knockback_power: 0.0,
                },
                force: [0.0, 0.0, 0.0].into(),
                flags: 1,
                active: 1,
                deactivate_on_enter: 0,
                deactivate_on_exit: 0,
            })),
        },
        // objects.push(
        //     structs::SclyObject {
        //         instance_id: area.new_object_id_from_layer_name("Default"),
        //         connections: vec![
        //             structs::Connection {
        //                 state: structs::ConnectionState::INSIDE,
        //                 message: structs::ConnectionMsg::INCREMENT,
        //                 target_object_id: camear_hint_id,
        //             },
        //             structs::Connection {
        //                 state: structs::ConnectionState::EXITED,
        //                 message: structs::ConnectionMsg::DECREMENT,
        //                 target_object_id: camear_hint_id,
        //             }
        //         ].into(),
        //         property_data: structs::SclyProperty::CameraHintTrigger(
        //             Box::new(structs::CameraHintTrigger {
        //                 name: b"CameraHintTrigger\0".as_cstr(),
        //                 position: spawn_point_position.into(),
        //                 rotation: spawn_point_rotation.into(),
        //                 scale: [10.0, 10.0, 10.0].into(),
        //                 active: 1,
        //                 deactivate_on_enter: 0,
        //                 deactivate_on_exit: 0,
        //             })
        //         ),
        //     }
        // );
    ];
    objects
}

pub fn patch_add_escape_sequence(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    time: f32,
    start_trigger_pos: [f32; 3],
    start_trigger_scale: [f32; 3],
    stop_trigger_pos: [f32; 3],
    stop_trigger_scale: [f32; 3],
) -> Result<(), String> {
    let start_special_function_id = area.new_object_id_from_layer_name("Default");
    let stop_special_function_id = area.new_object_id_from_layer_name("Default");
    let start_sequence_trigger_id = area.new_object_id_from_layer_name("Default");
    let stop_sequence_trigger_id = area.new_object_id_from_layer_name("Default");

    let layers = area.mrea().scly_section_mut().layers.as_mut_vec();
    let objects = layers[0].objects.as_mut_vec();

    objects.push(structs::SclyObject {
        instance_id: start_special_function_id,
        connections: vec![].into(),
        property_data: structs::SclyProperty::SpecialFunction(Box::new(structs::SpecialFunction {
            name: b"start escape sequence\0".as_cstr(),
            position: [0.0, 0.0, 0.0].into(),
            rotation: [0.0, 0.0, 0.0].into(),
            type_: 11, // escape sequence
            unknown0: b"\0".as_cstr(),
            unknown1: time,
            unknown2: 0.0,
            unknown3: 0.0,
            layer_change_room_id: 0,
            layer_change_layer_id: 0,
            item_id: 0,
            unknown4: 1, // active
            unknown5: 0.0,
            unknown6: 0xFFFFFFFF,
            unknown7: 0xFFFFFFFF,
            unknown8: 0xFFFFFFFF,
        })),
    });

    objects.push(structs::SclyObject {
        instance_id: start_sequence_trigger_id,
        property_data: structs::Trigger {
            name: b"Start Sequence Trigger\0".as_cstr(),
            position: start_trigger_pos.into(),
            scale: start_trigger_scale.into(),
            damage_info: structs::scly_structs::DamageInfo {
                weapon_type: 0,
                damage: 0.0,
                radius: 0.0,
                knockback_power: 0.0,
            },
            force: [0.0, 0.0, 0.0].into(),
            flags: 1,
            active: 1,
            deactivate_on_enter: 0,
            deactivate_on_exit: 0,
        }
        .into(),
        connections: vec![structs::Connection {
            state: structs::ConnectionState::EXITED,
            message: structs::ConnectionMsg::ACTION,
            target_object_id: start_special_function_id,
        }]
        .into(),
    });

    objects.push(structs::SclyObject {
        instance_id: stop_special_function_id,
        connections: vec![].into(),
        property_data: structs::SclyProperty::SpecialFunction(Box::new(structs::SpecialFunction {
            name: b"stop escape sequence\0".as_cstr(),
            position: [0.0, 0.0, 0.0].into(),
            rotation: [0.0, 0.0, 0.0].into(),
            type_: 11, // escape sequence
            unknown0: b"\0".as_cstr(),
            unknown1: 0.0, // Set the timer to 0.0, so it stops counting
            unknown2: 0.0,
            unknown3: 0.0,
            layer_change_room_id: 0,
            layer_change_layer_id: 0,
            item_id: 0,
            unknown4: 1, // active
            unknown5: 0.0,
            unknown6: 0xFFFFFFFF,
            unknown7: 0xFFFFFFFF,
            unknown8: 0xFFFFFFFF,
        })),
    });

    objects.push(structs::SclyObject {
        instance_id: stop_sequence_trigger_id,
        property_data: structs::Trigger {
            name: b"stop Sequence Trigger\0".as_cstr(),
            position: stop_trigger_pos.into(),
            scale: stop_trigger_scale.into(),
            damage_info: structs::scly_structs::DamageInfo {
                weapon_type: 0,
                damage: 0.0,
                radius: 0.0,
                knockback_power: 0.0,
            },
            force: [0.0, 0.0, 0.0].into(),
            flags: 1,
            active: 1,
            deactivate_on_enter: 0,
            deactivate_on_exit: 0,
        }
        .into(),
        connections: vec![structs::Connection {
            state: structs::ConnectionState::ENTERED,
            message: structs::ConnectionMsg::ACTION,
            target_object_id: stop_special_function_id,
        }]
        .into(),
    });

    Ok(())
}
