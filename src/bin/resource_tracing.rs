//! This program traces the dependencies of each pickup in a Metroid Prime ISO.
//! The location of the ISO should be provided as a command line argument.

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    env::args,
    fs::File,
    mem, str as stdstr,
};

pub use randomprime::*;
use randomprime::{
    custom_assets::custom_asset_ids,
    pickup_meta::{pickup_model_for_pickup, PickupModel, PickupType, ScriptObjectLocation},
};
use reader_writer::{FourCC, Readable, Reader, Writable};
use resource_info_table::{resource_info, ResourceInfo};
use structs::{res_id, Ancs, Cmdl, Evnt, Pickup, ResId, Resource, Scan};

// Duplicated from pickup_meta. This version needs owned-lists instead of borrowed.
#[derive(Clone, Debug)]
pub struct PickupLocation {
    location: ScriptObjectLocation,
    hudmemo: ScriptObjectLocation,
    attainment_audio: ScriptObjectLocation,
    memory_relay: ScriptObjectLocation,
    post_pickup_relay_connections: Vec<structs::Connection>,
    position: [f32; 3],
}

#[derive(Clone, Debug)]
pub struct DoorLocation {
    door_location: Option<ScriptObjectLocation>,
    door_rotation: Option<[f32; 3]>,
    door_force_locations: Vec<ScriptObjectLocation>,
    door_shield_locations: Vec<ScriptObjectLocation>,
    dock_number: u32,
    dock_position: [f32; 3],
    dock_scale: [f32; 3],
}

struct ResourceDb<'r> {
    map: HashMap<ResourceKey, ResourceDbRecord<'r>>,
}

#[derive(Debug)]
struct ResourceDbRecord<'r> {
    data: ResourceData<'r>,
    deps: Option<HashSet<ResourceKey>>,
}

impl<'r> ResourceDb<'r> {
    fn new() -> ResourceDb<'r> {
        ResourceDb {
            map: HashMap::new(),
        }
    }

    fn add_resource(&mut self, res: Resource<'r>) {
        let key = ResourceKey::new(res.file_id, res.fourcc());
        self.map
            .entry(key)
            .or_insert_with(move || ResourceDbRecord {
                data: ResourceData::new(&res),
                deps: None,
            });
    }

    fn get_dependencies(&mut self, pickup: &Pickup) -> HashSet<ResourceKey> {
        let base_resources = [
            (ResourceKey::from(pickup.cmdl), None),
            (
                ResourceKey::from(pickup.ancs.file_id),
                Some(pickup.ancs.node_index),
            ),
            (
                ResourceKey::from(pickup.actor_params.scan_params.scan),
                None,
            ),
            (ResourceKey::from(pickup.actor_params.xray_cmdl), None),
            (ResourceKey::from(pickup.actor_params.xray_cskr), None),
            (ResourceKey::from(pickup.part), None),
        ];
        let mut result = HashSet::new();
        for r in base_resources.iter() {
            self.extend_set_with_deps(&mut result, r.0, r.1);
        }
        result
    }

    // The output has been tailored to match the observed behavior of Claris's
    // randomizer.
    // A few sections of code are commented out, indicating what appear to me to
    // be dependencies, but don't seem to match Claris's dependency lists.
    fn get_resource_deps(
        &mut self,
        key: ResourceKey,
        ancs_node: Option<u32>,
    ) -> HashSet<ResourceKey> {
        let mut deps = HashSet::with_capacity(0);

        let data = {
            let record = &self.map[&key];
            if let Some(ref deps) = record.deps {
                return deps.clone();
            };
            record.data.clone()
        };
        {
            // To avoid line-wrapping, create a "specialized" version of the method.
            let mut extend_deps = |id, b: &[u8; 4]| {
                self.extend_set_with_deps(&mut deps, ResourceKey::new(id, b.into()), None);
            };

            if key.fourcc == b"SCAN".into() {
                let scan: Scan = data.data.clone().read(());
                extend_deps(scan.frme.to_u32(), b"FRME");
                extend_deps(scan.strg.to_u32(), b"STRG");
            } else if key.fourcc == b"EVNT".into() {
                let evnt: Evnt = data.data.clone().read(());
                for effect in evnt.effect_events.iter() {
                    extend_deps(effect.effect_file_id, effect.effect_type.as_bytes());
                }
            } else if key.fourcc == b"PART".into() {
                let buf = data.decompress();
                let buf: &[u8] = &buf;
                // We're cheating here. We're going to find the sub-string ICTSCNST
                // and then using the next word as the id of a PART.
                for i in 0..(buf.len() - 8) {
                    if &buf[i..(i + 8)] == b"ICTSCNST" {
                        let id: u32 = Reader::new(&buf[(i + 8)..(i + 12)]).read(());
                        if id != 0 {
                            extend_deps(id, b"PART");
                        }
                        // TODO: IITS and IDTS too?
                    } else if &buf[i..(i + 4)] == b"TEXR" {
                        if &buf[(i + 4)..(i + 8)] == b"ATEX" {
                            let id: u32 = Reader::new(&buf[(i + 12)..(i + 16)]).read(());
                            if id != 0 {
                                extend_deps(id, b"TXTR");
                            }
                        }
                    } else if &buf[i..(i + 4)] == b"KSSM" && &buf[(i + 4)..(i + 8)] != b"NONE" {
                        let kssm: structs::Kssm = Reader::new(&buf[(i + 8)..]).read(());
                        for list in kssm.lists.iter() {
                            for item in list.items.iter() {
                                extend_deps(item.part.to_u32(), b"PART");
                            }
                        }
                    }
                }
            } else if key.fourcc == b"CMDL".into() {
                let buf = data.decompress();
                let cmdl: Cmdl = Reader::new(&buf).read(());
                for material in cmdl.material_sets.iter() {
                    for id in material.texture_ids.iter() {
                        extend_deps((*id).to_u32(), b"TXTR");
                    }
                }
            } else if key.fourcc == b"ANCS".into() {
                let buf = data.decompress();
                let ancs: Ancs = Reader::new(&buf).read(());
                if let Some(ancs_node) = ancs_node {
                    let char_info = ancs
                        .char_set
                        .char_info
                        .iter()
                        .nth(ancs_node as usize)
                        .unwrap();
                    extend_deps(char_info.cmdl.to_u32(), b"CMDL");
                    extend_deps(char_info.cskr.to_u32(), b"CSKR");
                    extend_deps(char_info.cinf.to_u32(), b"CINF");
                    // char_info.effects.map(|effects| for effect in effects.iter() {
                    //     for comp in effect.components.iter() {
                    //         extend_deps(ResourceKey::new(comp.file_id, comp.type_));
                    //     }
                    // });
                    // char_info.overlay_cmdl.map(|cmdl| extend_deps(cmdl, b"CMDL"));
                    // char_info.overlay_cskr.map(|cmdl| extend_deps(cmdl, b"CSKR"));
                    for part in char_info.particles.part_assets.iter() {
                        extend_deps(*part, b"PART");
                    }
                };
                if let Some(i) = ancs.anim_set.animation_resources {
                    for anim_resource in i.iter() {
                        extend_deps(anim_resource.anim.to_u32(), b"ANIM");
                        extend_deps(anim_resource.evnt.to_u32(), b"EVNT");
                    }
                }
            }
        }

        // We can't safely cache the result if we are using a specific ANCS node.
        // XXX This would be fine if the data structure implementing the cache was
        //     reworked.
        if ancs_node.is_none() {
            self.map.get_mut(&key).unwrap().deps = Some(deps.clone());
        }
        deps
    }

    fn extend_set_with_deps(
        &mut self,
        set: &mut HashSet<ResourceKey>,
        key: ResourceKey,
        ancs_node: Option<u32>,
    ) {
        if key.file_id != u32::MAX {
            set.insert(key);
            set.extend(self.get_resource_deps(key, ancs_node));
        };
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct ResourceKey {
    file_id: u32,
    fourcc: FourCC,
}

impl From<ResourceInfo> for ResourceKey {
    fn from(res_info: ResourceInfo) -> ResourceKey {
        ResourceKey::new(res_info.res_id, res_info.fourcc)
    }
}

impl<K: res_id::ResIdKind> From<ResId<K>> for ResourceKey {
    fn from(_res_id: ResId<K>) -> ResourceKey {
        ResourceKey::new(_res_id.to_u32(), K::FOURCC)
    }
}

impl ResourceKey {
    fn new(file_id: u32, fourcc: FourCC) -> ResourceKey {
        ResourceKey { file_id, fourcc }
    }
}

static CUT_SCENE_PICKUPS: &[(u32, u32)] = &[
    (0x3C785450, 589860),    // Morph Ball
    (0x0D72F1F7, 1377077),   // Wavebuster
    (0x11BD63B7, 1769497),   // Artifact of Lifegiver
    (0xC8309DF6, 2359772),   // Missile Launcher
    (0x9A0A03EB, 2435310),   // Varia Suit
    (0x9A0A03EB, 405090173), // Artifact of Wild
    (0x492CBF4A, 2687109),   // Charge Beam
    (0x4148F7B0, 3155850),   // Morph Ball Bomb
    (0xE1981EFC, 3735555),   // Artifact of World
    (0xAFEFE677, 3997699),   // Ice Beam
    // XXX Doesn't normally have a cutscene. Skip?
    (0x6655F51E, 524887),  // Artifact of Sun
    (0x40C548E9, 917592),  // Wave Beam
    (0xA20A7455, 1048801), // Boost Ball
    (0x70181194, 1573322), // Spider Ball
    (0x3FB4A34E, 1966838), // Super Missile
    // XXX Doesn't normally have a cutscene. Skip?
    (0xB3C33249, 2557135),  // Artifact of Elder
    (0xA49B2544, 69730588), // Thermal Visor
    (0x49175472, 3473439),  // Gravity Suit
    (0xF7C84340, 3539113),  // Artifact of Spirit
    (0xC44E7A07, 262151),   // Space Jump Boots
    (0x2398E906, 68157908), // Artifact of Truth
    (0x86EB2E02, 2752545),  // X-Ray Visor
    // XXX Doesn't normally have a cutscene. Skip?
    (0x86EB2E02, 2753076), // Artifact of Chozo
    (0xE39C342B, 589827),  // Grapple Beam
    (0x35C5D736, 786470),  // Flamethrower !!!!
    // XXX Doesn't normally have a cutscene. Skip?
    (0x8A97BB54, 852800), // Artifact of Warrior
    // XXX Doesn't normally have a cutscene. Skip?
    (0xBBFA4AB3, 2556031), // Artifact of Newborn
    (0xA4719C6A, 272508),  // Artifact of Nature
    // XXX Doesn't normally have a cutscene. Skip?
    (0x89A6CB8D, 720951),  // Artifact of Strength
    (0x901040DF, 786472),  // Ice Spreader
    (0x4CC18E5A, 1376287), // Plasma Beam
];

#[derive(Debug)]
struct PickupData {
    bytes: Vec<u8>,
    deps: HashSet<ResourceKey>,
    attainment_audio_file_name: Vec<u8>,
}

#[derive(Debug)]
struct RoomInfo {
    room_id: ResId<res_id::MREA>,
    name: String,
    name_id: ResId<res_id::STRG>,
    mapa_id: ResId<res_id::MAPA>,
    pickups: Vec<PickupLocation>,
    doors: Vec<DoorLocation>,
    objects_to_remove: HashMap<u32, Vec<u32>>,
    size_index: f32,
}

fn build_scly_db<'r>(scly: &structs::Scly<'r>) -> HashMap<u32, (usize, structs::SclyObject<'r>)> {
    let mut scly_db = HashMap::new();
    for (layer_num, scly_layer) in scly.layers.iter().enumerate() {
        for obj in scly_layer.objects.iter() {
            let obj = obj.into_owned();
            assert!(scly_db.insert(obj.instance_id, (layer_num, obj)).is_none());
        }
    }
    scly_db
}

fn find_audio_attainment<'r>(
    obj: &structs::SclyObject<'r>,
    scly_db: &HashMap<u32, (usize, structs::SclyObject<'r>)>,
) -> Option<structs::SclyObject<'r>> {
    let post_pickup_relay = search_for_scly_object(&obj.connections, scly_db, |o| {
        o.property_data
            .as_relay()
            .map(|i| i.name.to_bytes() == b"Relay Post Pickup")
            .unwrap_or(false)
    })?;

    const ATTAINMENT_AUDIO_FILES: &[&[u8]] = &[
        b"/audio/itm_x_short_02.dsp",
        b"audio/jin_artifact.dsp",
        b"audio/jin_itemattain.dsp",
    ];
    search_for_scly_object(&post_pickup_relay.connections, scly_db, |obj| {
        obj.property_data
            .as_streamed_audio()
            .map(|sa| ATTAINMENT_AUDIO_FILES.contains(&sa.audio_file_name.to_bytes()))
            .unwrap_or(false)
    })
}

fn extract_pickup_data<'r>(
    scly: &structs::Scly<'r>,
    obj: &structs::SclyObject<'r>,
    res_db: &mut ResourceDb<'r>,
) -> PickupData {
    let mut pickup = obj.property_data.as_pickup().unwrap().into_owned();

    // XXX It's important to collect the dependencies before we modify the pickup object
    let mut deps = res_db.get_dependencies(&pickup);
    patch_dependencies(pickup.kind, &mut deps);

    let scly_db = build_scly_db(scly);

    let attainment_audio_file_name = if let Some(aa) = find_audio_attainment(obj, &scly_db) {
        let streamed_audio = aa.property_data.as_streamed_audio().unwrap();
        streamed_audio
            .audio_file_name
            .to_bytes_with_nul()
            .to_owned()
    } else {
        // The Phazon Suit is weird: the audio object isn't directly connected to the
        // Pickup. So, hardcode its location.
        // assert_eq!(pickup.kind, 23);
        if pickup.kind == 23 {
            b"audio/jin_itemattain.dsp\0".to_vec()
        } else {
            b"audio/itm_x_short_02.dsp\0".to_vec()
        }
    };

    if pickup.kind == 23 {
        pickup.cmdl = custom_asset_ids::PHAZON_SUIT_CMDL;
        pickup.ancs.file_id = custom_asset_ids::PHAZON_SUIT_ANCS;
    }

    let mut bytes = vec![];
    pickup.write_to(&mut bytes).unwrap();

    PickupData {
        bytes,
        deps,
        attainment_audio_file_name,
    }
}

fn extract_pickup_location<'r>(
    mrea_id: u32,
    scly: &structs::Scly<'r>,
    obj: &structs::SclyObject<'r>,
    obj_location: ScriptObjectLocation,
) -> (PickupLocation, Vec<ScriptObjectLocation>) {
    let pickup = obj.property_data.as_pickup().unwrap();

    let scly_db = build_scly_db(scly);

    let attainment_audio_location = if let Some(aa) = find_audio_attainment(obj, &scly_db) {
        ScriptObjectLocation {
            layer: scly_db[&aa.instance_id].0 as u32,
            instance_id: aa.instance_id,
        }
    } else {
        // Phazon suit override
        if pickup.kind == 23 {
            // phazon suit
            ScriptObjectLocation {
                layer: 1,
                instance_id: 68813644,
            }
        } else {
            ScriptObjectLocation {
                layer: 0,
                instance_id: 0xFFFFFFFF,
            }
        }
    };

    let hudmemo = search_for_scly_object(&obj.connections, &scly_db, |obj| {
        obj.property_data
            .as_hud_memo()
            .map(|hm| hm.name.to_str().unwrap().contains("Pickup"))
            .unwrap_or(false)
    });
    let hudmemo_loc = if let Some(hudmemo) = hudmemo {
        ScriptObjectLocation {
            layer: scly_db[&hudmemo.instance_id].0 as u32,
            instance_id: hudmemo.instance_id,
        }
    } else if pickup.kind == 23 {
        // phazon suit
        ScriptObjectLocation {
            layer: scly_db[&68813640].0 as u32,
            instance_id: 68813640,
        }
    } else {
        ScriptObjectLocation {
            layer: 0,
            instance_id: 0xFFFFFFFF,
        }
    };

    let mut removals = Vec::new();
    if pickup.kind >= 29 && pickup.kind <= 40 {
        // If this is an artifact...
        let layer_switch_function = search_for_scly_object(&obj.connections, &scly_db, |obj| {
            obj.property_data
                .as_special_function()
                .map(|hm| {
                    hm.name.to_str().unwrap()
                        == "SpecialFunction ScriptLayerController -- Stonehenge Totem"
                })
                .unwrap_or(false)
        })
        .unwrap();
        removals.push(ScriptObjectLocation {
            layer: scly_db[&layer_switch_function.instance_id].0 as u32,
            instance_id: layer_switch_function.instance_id,
        });

        let pause_function = search_for_scly_object(&obj.connections, &scly_db, |obj| {
            obj.property_data
                .as_special_function()
                .map(|hm| hm.name.to_str().unwrap() == "SpecialFunction - Enter Logbook Screen")
                .unwrap_or(false)
        })
        .unwrap();
        removals.push(ScriptObjectLocation {
            layer: scly_db[&pause_function.instance_id].0 as u32,
            instance_id: pause_function.instance_id,
        });
    }

    // Remove the PlayerHint objects that disable control when collecting an item.
    let player_hint = search_for_scly_object(&obj.connections, &scly_db, |obj| {
        obj.property_data
            .as_player_hint()
            .map(|hm| hm.name.to_str().unwrap() == "Player Hint Disable Controls")
            .unwrap_or(false)
    });
    if let Some(player_hint) = player_hint {
        removals.push(ScriptObjectLocation {
            layer: scly_db[&player_hint.instance_id].0 as u32,
            instance_id: player_hint.instance_id,
        });
    };

    // Get the memory relay linked to the pickup
    // This memory relay is activated when getting the pickup
    // and deactivate the pickup when loading again the room
    let memory_relay = search_for_scly_object(&obj.connections, &scly_db, |obj| {
        obj.property_data
            .as_memory_relay()
            .map(|hm| hm.name.to_str().unwrap() == "Memory Relay")
            .unwrap_or(false)
    });

    let memory_relay_id = if let Some(memory_relay) = memory_relay {
        memory_relay.instance_id
    } else {
        panic!(
            "Couldn't find the memory relay for pickup {:X}",
            obj.instance_id
        )
    };

    // If this is a pickup with an associated cutscene, find the connections we want to
    // preserve and the objects we want to remove.
    let post_pickup_relay_connections = if CUT_SCENE_PICKUPS.contains(&(mrea_id, obj.instance_id)) {
        removals.push(find_cutscene_trigger_relay(
            pickup.kind,
            &obj.connections,
            &scly_db,
        ));
        build_skip_cutscene_relay_connections(pickup.kind, &obj.connections, &scly_db)
    } else {
        vec![]
    };

    let location = PickupLocation {
        location: ScriptObjectLocation {
            layer: obj_location.layer,
            instance_id: obj.instance_id,
        },
        attainment_audio: attainment_audio_location,
        hudmemo: hudmemo_loc,
        post_pickup_relay_connections,
        memory_relay: ScriptObjectLocation {
            layer: memory_relay_id >> 26,
            instance_id: memory_relay_id,
        },
        position: pickup.position.into(),
    };

    (location, removals)
}

fn search_for_scly_object<'r, F>(
    connections: &reader_writer::LazyArray<'r, structs::Connection>,
    scly_db: &HashMap<u32, (usize, structs::SclyObject<'r>)>,
    f: F,
) -> Option<structs::SclyObject<'r>>
where
    F: Fn(&structs::SclyObject<'r>) -> bool,
{
    let mut stack = Vec::new();

    // Circular references are possible, so keep track of which ones we've seen
    // already.
    let mut seen = HashSet::new();

    for c in connections {
        stack.push(c.target_object_id);
        seen.insert(c.target_object_id);
    }

    while let Some(id) = stack.pop() {
        let obj = if let Some((_, obj)) = scly_db.get(&id) {
            obj
        } else {
            continue;
        };
        if f(obj) {
            return Some(obj.clone());
        }
        for c in obj.connections.iter() {
            if !seen.contains(&c.target_object_id) {
                stack.push(c.target_object_id);
                seen.insert(c.target_object_id);
            }
        }
    }
    None
}

fn build_skip_cutscene_relay_connections<'r>(
    pickup_type: u32,
    obj_connections: &reader_writer::LazyArray<'r, structs::Connection>,
    scly_db: &HashMap<u32, (usize, structs::SclyObject<'r>)>,
) -> Vec<structs::Connection> {
    let post_pickup_relay = search_for_scly_object(obj_connections, scly_db, |o| {
        o.property_data
            .as_relay()
            .map(|i| i.name.to_bytes() == b"Relay Post Pickup")
            .unwrap_or(false)
    })
    .unwrap();

    let mut connections = vec![];
    for conn in post_pickup_relay.connections.iter() {
        let connected_object = if let Some(obj) = scly_db.get(&conn.target_object_id) {
            &obj.1
        } else {
            connections.push(conn.into_owned());
            continue;
        };
        if let Some(timer) = connected_object.property_data.as_timer() {
            let name = timer.name.to_bytes();
            if name == b"Timer Jingle" {
                connections.extend(connected_object.connections.iter().map(|i| i.into_owned()));
            } else if name == b"Timer HUD" {
                // We want to copy most of Timer HUD's connections, with a few exceptions
                for conn in connected_object.connections.iter() {
                    let obj = if let Some(obj) = scly_db.get(&conn.target_object_id) {
                        &obj.1
                    } else {
                        connections.push(conn.into_owned());
                        continue;
                    };

                    let is_log_screen_timer = obj
                        .property_data
                        .as_timer()
                        .map(|i| i.name.to_bytes() == &b"Timer - Delay Enter Logbook Screen"[..])
                        .unwrap_or(false);
                    // Skip player hints and a artifact log screen timers
                    // Note the special case for the Artifact of Truth's timer
                    if (is_log_screen_timer && obj.instance_id != 1049534)
                        || obj.property_data.as_player_hint().is_some()
                    {
                        continue;
                    }
                    connections.push(conn.into_owned());
                }
            } else {
                connections.push(conn.into_owned());
            }
        } else if connected_object.property_data.as_player_hint().is_none() {
            // Skip the Player Hint objects.
            connections.push(conn.into_owned());
        }
    }

    // Stop here if not the Varia Suit
    if pickup_type != 22 {
        return connections;
    }

    // We need a special case for the Varia Suit to unlock the doors
    let unlock_doors_relay = search_for_scly_object(obj_connections, scly_db, |o| {
        o.property_data
            .as_relay()
            .map(|i| i.name.to_bytes() == &b"!Relay Local End Suit Attainment Cinematic"[..])
            .unwrap_or(false)
    })
    .unwrap();

    for conn in unlock_doors_relay.connections.iter() {
        let connected_object = &scly_db.get(&conn.target_object_id).unwrap().1;
        if connected_object.property_data.as_dock().is_some()
            || connected_object.property_data.as_trigger().is_some()
        {
            connections.push(conn.into_owned());
        }
    }

    connections
}

fn find_cutscene_trigger_relay<'r>(
    pickup_type: u32,
    obj_connections: &reader_writer::LazyArray<'r, structs::Connection>,
    scly_db: &HashMap<u32, (usize, structs::SclyObject<'r>)>,
) -> ScriptObjectLocation {
    // We need to look for specific object names depending on the pickup type. This is mostly the
    // result of the non-cutscene artifacts, for which the relay we're looking for is simply titled
    // "Relay".
    // We need this seperate static in order to get static lifetimes. Its kinda awful.
    static NAME_CANDIDATES: &[&[u8]] = &[
        b"!Relay Start Suit Attainment Cinematic",
        b"!Relay Local Start Suit Attainment Cinematic",
        b"Relay-start of cinema",
        b"Relay",
    ];
    let name_candidates: &[&[u8]] = match pickup_type {
        21 => &NAME_CANDIDATES[0..1],
        22 => &NAME_CANDIDATES[1..2],
        29..=40 => &NAME_CANDIDATES[2..4],
        _ => &NAME_CANDIDATES[2..3],
    };
    let obj = search_for_scly_object(obj_connections, scly_db, |o| {
        o.property_data
            .as_relay()
            .map(|i| name_candidates.contains(&i.name.to_bytes()))
            .unwrap_or(false)
    })
    .unwrap();
    ScriptObjectLocation {
        layer: scly_db[&obj.instance_id].0 as u32,
        instance_id: obj.instance_id,
    }
}

// We can get pretty close to the Claris's dependecies for each pickup, but some
// of them need custom modification to match exactly.
fn patch_dependencies(pickup_kind: u32, deps: &mut HashSet<ResourceKey>) {
    // Don't ask me why; Claris seems to skip this one.
    deps.remove(&resource_info!("purple.PART").into());

    if pickup_kind == 19 {
        // Spiderball. I couldn't find any references to this outside of PAK resource
        // indexes and dependency lists.
        deps.insert(resource_info!("spiderball.CSKR").into());
    } else if pickup_kind == 23 {
        // Remove the Gravity Suit's CMDL and ANCS
        deps.remove(&resource_info!("Node1_11.CMDL").into());
        deps.remove(&resource_info!("Node1_11.ANCS").into());
        deps.remove(&ResourceKey::new(0x08C625DA, b"TXTR".into()));
        deps.remove(&ResourceKey::new(0xA95D06BC, b"TXTR".into()));

        // Add the custom CMDL and textures
        deps.insert(ResourceKey::from(custom_asset_ids::PHAZON_SUIT_CMDL));
        deps.insert(ResourceKey::from(custom_asset_ids::PHAZON_SUIT_ANCS));
        deps.insert(ResourceKey::from(custom_asset_ids::PHAZON_SUIT_TXTR1));
        deps.insert(ResourceKey::from(custom_asset_ids::PHAZON_SUIT_TXTR2));
    } else if pickup_kind == PickupType::HealthRefill.kind() {
        deps.remove(&ResourceKey::new(0xF02F1B9A, b"PART".into())); // This doesn't exist in PAL, and NTSC is fine
    }
}

fn create_nothing(pickup_table: &mut HashMap<PickupModel, PickupData>) {
    // Special case for Nothing
    let mut nothing_bytes = Vec::new();
    {
        let mut nothing_pickup: structs::Pickup =
            Reader::new(&pickup_table[&PickupModel::PhazonSuit].bytes)
                .read::<Pickup>(())
                .clone();
        nothing_pickup.name = Cow::Borrowed(c"Nothing");
        nothing_pickup.kind = PickupType::Missile.kind();
        nothing_pickup.max_increase = 0;
        nothing_pickup.curr_increase = 0;
        nothing_pickup.cmdl = custom_asset_ids::NOTHING_CMDL;
        nothing_pickup.ancs.file_id = custom_asset_ids::NOTHING_ANCS;
        nothing_pickup.part = ResId::<res_id::PART>::invalid();
        nothing_pickup.write_to(&mut nothing_bytes).unwrap();
    }
    let mut nothing_deps: HashSet<_> = pickup_table[&PickupModel::PhazonSuit]
        .deps
        .iter()
        .filter(|i| ![b"SCAN".into(), b"STRG".into(), b"CMDL".into()].contains(&i.fourcc))
        .cloned()
        .collect();
    nothing_deps.extend(&[
        ResourceKey::from(custom_asset_ids::NOTHING_CMDL),
        ResourceKey::from(custom_asset_ids::NOTHING_ANCS),
        ResourceKey::from(custom_asset_ids::NOTHING_TXTR),
    ]);
    assert!(pickup_table
        .insert(
            PickupModel::Nothing,
            PickupData {
                bytes: nothing_bytes,
                deps: nothing_deps,
                attainment_audio_file_name: b"/audio/itm_x_short_02.dsp\0".to_vec(),
            }
        )
        .is_none());
}

fn create_zoomer(pickup_table: &mut HashMap<PickupModel, PickupData>) {
    let mut bytes = Vec::new();
    {
        let mut pickup: structs::Pickup =
            Reader::new(&pickup_table[&PickupModel::PhazonSuit].bytes)
                .read::<Pickup>(())
                .clone();
        pickup.name = Cow::Borrowed(c"Zoomer");
        pickup.kind = PickupType::Missile.kind();
        pickup.max_increase = 0;
        pickup.curr_increase = 0;
        pickup.cmdl = custom_asset_ids::ZOOMER_CMDL;
        pickup.ancs.file_id = custom_asset_ids::ZOOMER_ANCS;
        pickup.part = ResId::<res_id::PART>::invalid();
        pickup.write_to(&mut bytes).unwrap();
    }
    let mut deps: HashSet<_> = pickup_table[&PickupModel::PhazonSuit]
        .deps
        .iter()
        .filter(|i| ![b"SCAN".into(), b"STRG".into(), b"CMDL".into()].contains(&i.fourcc))
        .cloned()
        .collect();
    deps.extend(&[
        ResourceKey::from(custom_asset_ids::ZOOMER_CMDL),
        ResourceKey::from(custom_asset_ids::ZOOMER_ANCS),
        ResourceKey::from(custom_asset_ids::NOTHING_TXTR),
    ]);
    assert!(pickup_table
        .insert(
            PickupModel::Zoomer,
            PickupData {
                bytes,
                deps,
                attainment_audio_file_name: b"/audio/itm_x_short_02.dsp\0".to_vec(),
            }
        )
        .is_none());
}

fn create_cog(pickup_table: &mut HashMap<PickupModel, PickupData>) {
    let mut bytes = Vec::new();
    {
        let mut pickup: structs::Pickup =
            Reader::new(&pickup_table[&PickupModel::PhazonSuit].bytes)
                .read::<Pickup>(())
                .clone();
        pickup.name = Cow::Borrowed(c"Cog");
        pickup.kind = PickupType::Missile.kind();
        pickup.max_increase = 0;
        pickup.curr_increase = 0;
        pickup.cmdl = custom_asset_ids::COG_CMDL;
        pickup.ancs.file_id = custom_asset_ids::COG_ANCS;
        pickup.part = ResId::<res_id::PART>::invalid();
        pickup.write_to(&mut bytes).unwrap();
    }
    let mut deps: HashSet<_> = pickup_table[&PickupModel::PhazonSuit]
        .deps
        .iter()
        .filter(|i| ![b"SCAN".into(), b"STRG".into(), b"CMDL".into()].contains(&i.fourcc))
        .cloned()
        .collect();
    deps.extend(&[
        ResourceKey::from(custom_asset_ids::COG_CMDL),
        ResourceKey::from(custom_asset_ids::COG_ANCS),
        ResourceKey::new(0x50DF3CAD, b"TXTR".into()),
    ]);
    assert!(pickup_table
        .insert(
            PickupModel::Cog,
            PickupData {
                bytes,
                deps,
                attainment_audio_file_name: b"/audio/itm_x_short_02.dsp\0".to_vec(),
            }
        )
        .is_none());
}

fn create_gamecube(pickup_table: &mut HashMap<PickupModel, PickupData>) {
    let mut bytes = Vec::new();
    {
        let mut pickup: structs::Pickup =
            Reader::new(&pickup_table[&PickupModel::PhazonSuit].bytes)
                .read::<Pickup>(())
                .clone();
        pickup.name = Cow::Borrowed(c"Gamecube");
        pickup.kind = PickupType::Missile.kind();
        pickup.max_increase = 0;
        pickup.curr_increase = 0;
        pickup.cmdl = custom_asset_ids::RANDOVANIA_GAMECUBE_CMDL;
        pickup.ancs.file_id = custom_asset_ids::RANDOVANIA_GAMECUBE_ANCS;
        pickup.part = ResId::<res_id::PART>::invalid();
        pickup.write_to(&mut bytes).unwrap();
    }

    let mut deps: HashSet<_> = pickup_table[&PickupModel::PhazonSuit]
        .deps
        .iter()
        .filter(|i| ![b"SCAN".into(), b"STRG".into(), b"CMDL".into()].contains(&i.fourcc))
        .cloned()
        .collect();
    deps.extend(&[
        ResourceKey::from(custom_asset_ids::RANDOVANIA_GAMECUBE_CMDL),
        ResourceKey::from(custom_asset_ids::RANDOVANIA_GAMECUBE_ANCS),
        ResourceKey::from(custom_asset_ids::RANDOVANIA_GAMECUBE0_TXTR),
        ResourceKey::from(custom_asset_ids::RANDOVANIA_GAMECUBE1_TXTR),
        ResourceKey::from(ResId::<res_id::TXTR>::new(0x6E3D2E18)),
    ]);
    assert!(pickup_table
        .insert(
            PickupModel::RandovaniaGamecube,
            PickupData {
                bytes,
                deps,
                attainment_audio_file_name: b"/audio/itm_x_short_02.dsp\0".to_vec(),
            }
        )
        .is_none());
}

fn create_shiny_missile(pickup_table: &mut HashMap<PickupModel, PickupData>) {
    let mut shiny_missile_bytes = Vec::new();
    {
        let mut shiny_missile = Reader::new(&pickup_table[&PickupModel::Missile].bytes)
            .read::<Pickup>(())
            .clone();
        shiny_missile.name = Cow::Borrowed(c"Shiny Missile");
        shiny_missile.cmdl = custom_asset_ids::SHINY_MISSILE_CMDL;
        shiny_missile.ancs.file_id = custom_asset_ids::SHINY_MISSILE_ANCS;
        shiny_missile.write_to(&mut shiny_missile_bytes).unwrap();
    }

    let mut shiny_missile_deps: HashSet<_> = pickup_table[&PickupModel::Missile]
        .deps
        .iter()
        .filter(|i| {
            ![
                b"SCAN".into(),
                b"STRG".into(),
                b"CMDL".into(),
                b"ANCS".into(),
                b"EVNT".into(),
                b"TXTR".into(),
                b"PART".into(),
                b"ANIM".into(),
            ]
            .contains(&i.fourcc)
        })
        .cloned()
        .collect();
    shiny_missile_deps.extend(&[
        ResourceKey::from(custom_asset_ids::SHINY_MISSILE_CMDL),
        ResourceKey::from(custom_asset_ids::SHINY_MISSILE_ANCS),
        ResourceKey::from(custom_asset_ids::SHINY_MISSILE_EVNT),
        ResourceKey::from(custom_asset_ids::SHINY_MISSILE_ANIM),
        ResourceKey::from(custom_asset_ids::SHINY_MISSILE_TXTR0),
        ResourceKey::from(custom_asset_ids::SHINY_MISSILE_TXTR1),
        ResourceKey::from(custom_asset_ids::SHINY_MISSILE_TXTR2),
        resource_info!("healthnew.PART").into(),
        resource_info!("AfterPick.PART").into(),
    ]);
    assert!(pickup_table
        .insert(
            PickupModel::ShinyMissile,
            PickupData {
                bytes: shiny_missile_bytes,
                deps: shiny_missile_deps,
                attainment_audio_file_name: b"/audio/jin_itemattain.dsp\0".to_vec(),
            }
        )
        .is_none());
}

fn create_thermal_visor(pickup_table: &mut HashMap<PickupModel, PickupData>) {
    let mut bytes = Vec::new();
    {
        let mut visor = Reader::new(&pickup_table[&PickupModel::Visor].bytes)
            .read::<Pickup>(())
            .clone();
        visor.name = Cow::Borrowed(c"Thermal Visor");
        visor.cmdl = custom_asset_ids::THERMAL_CMDL;
        visor.ancs.file_id = custom_asset_ids::THERMAL_ANCS;
        visor.write_to(&mut bytes).unwrap();
    }

    let mut deps: HashSet<_> = pickup_table[&PickupModel::Visor]
        .deps
        .iter()
        .filter(|i| {
            ![
                b"SCAN".into(),
                b"STRG".into(),
                b"CMDL".into(),
                b"ANCS".into(),
            ]
            .contains(&i.fourcc)
        })
        .cloned()
        .collect();
    deps.extend(&[
        ResourceKey::from(custom_asset_ids::THERMAL_CMDL),
        ResourceKey::from(ResId::<res_id::TXTR>::new(0xFC095F6C)),
        ResourceKey::from(custom_asset_ids::THERMAL_ANCS),
    ]);
    assert!(pickup_table
        .insert(
            PickupModel::ThermalVisor,
            PickupData {
                bytes,
                deps,
                attainment_audio_file_name: b"/audio/jin_itemattain.dsp\0".to_vec(),
            }
        )
        .is_none());
}

fn create_xray_visor(pickup_table: &mut HashMap<PickupModel, PickupData>) {
    let mut bytes = Vec::new();
    {
        let mut visor = Reader::new(&pickup_table[&PickupModel::Visor].bytes)
            .read::<Pickup>(())
            .clone();
        visor.name = Cow::Borrowed(c"X-Ray Visor");
        visor.cmdl = custom_asset_ids::XRAY_CMDL;
        visor.ancs.file_id = custom_asset_ids::XRAY_ANCS;
        visor.write_to(&mut bytes).unwrap();
    }

    let mut deps: HashSet<_> = pickup_table[&PickupModel::Visor]
        .deps
        .iter()
        .filter(|i| {
            ![
                b"SCAN".into(),
                b"STRG".into(),
                b"CMDL".into(),
                b"ANCS".into(),
            ]
            .contains(&i.fourcc)
        })
        .cloned()
        .collect();
    deps.extend(&[
        ResourceKey::from(custom_asset_ids::XRAY_CMDL),
        ResourceKey::from(ResId::<res_id::TXTR>::new(0xBE4CD99D)),
        ResourceKey::from(custom_asset_ids::XRAY_ANCS),
    ]);
    assert!(pickup_table
        .insert(
            PickupModel::XRayVisor,
            PickupData {
                bytes,
                deps,
                attainment_audio_file_name: b"/audio/jin_itemattain.dsp\0".to_vec(),
            }
        )
        .is_none());
}

fn create_flamethrower(pickup_table: &mut HashMap<PickupModel, PickupData>) {
    let mut bytes = Vec::new();
    {
        let mut pickup = Reader::new(&pickup_table[&PickupModel::Flamethrower].bytes)
            .read::<Pickup>(())
            .clone();
        pickup.name = Cow::Borrowed(c"Flamethrower");
        pickup.cmdl = custom_asset_ids::FLAMETHROWER_PICKUP_CMDL;
        pickup.ancs.file_id = custom_asset_ids::FLAMETHROWER_PICKUP_ANCS;
        pickup.write_to(&mut bytes).unwrap();
    }

    let mut deps: HashSet<_> = pickup_table[&PickupModel::Flamethrower]
        .deps
        .iter()
        .filter(|i| {
            ![
                b"SCAN".into(),
                b"STRG".into(),
                b"CMDL".into(),
                b"ANCS".into(),
                b"TXTR".into(),
            ]
            .contains(&i.fourcc)
        })
        .cloned()
        .collect();

    deps.extend(&[
        ResourceKey::from(custom_asset_ids::FLAMETHROWER_PICKUP_CMDL),
        ResourceKey::from(custom_asset_ids::FLAMETHROWER_PICKUP_ANCS),
        ResourceKey::from(custom_asset_ids::FLAMETHROWER_PICKUP_TXTR1),
        ResourceKey::from(custom_asset_ids::FLAMETHROWER_PICKUP_TXTR2),
        ResourceKey::from(custom_asset_ids::FLAMETHROWER_PICKUP_TXTR3),
        ResourceKey::from(ResId::<res_id::TXTR>::new(0xB59EB7E6)),
    ]);

    assert!(pickup_table
        .insert(
            PickupModel::FlamethrowerNew,
            PickupData {
                bytes,
                deps,
                attainment_audio_file_name: b"/audio/jin_itemattain.dsp\0".to_vec(),
            }
        )
        .is_none());
}

fn create_combat_visor(pickup_table: &mut HashMap<PickupModel, PickupData>) {
    let mut bytes = Vec::new();
    {
        let mut visor = Reader::new(&pickup_table[&PickupModel::Visor].bytes)
            .read::<Pickup>(())
            .clone();
        visor.name = Cow::Borrowed(c"Combat Visor");
        visor.cmdl = custom_asset_ids::COMBAT_CMDL;
        visor.ancs.file_id = custom_asset_ids::COMBAT_ANCS;
        visor.write_to(&mut bytes).unwrap();
    }

    let mut deps: HashSet<_> = pickup_table[&PickupModel::Visor]
        .deps
        .iter()
        .filter(|i| {
            ![
                b"SCAN".into(),
                b"STRG".into(),
                b"CMDL".into(),
                b"ANCS".into(),
            ]
            .contains(&i.fourcc)
        })
        .cloned()
        .collect();
    deps.extend(&[
        ResourceKey::from(custom_asset_ids::COMBAT_CMDL),
        ResourceKey::from(ResId::<res_id::TXTR>::new(0x1D588B22)),
        ResourceKey::from(custom_asset_ids::COMBAT_ANCS),
    ]);
    assert!(pickup_table
        .insert(
            PickupModel::CombatVisor,
            PickupData {
                bytes,
                deps,
                attainment_audio_file_name: b"/audio/jin_itemattain.dsp\0".to_vec(),
            }
        )
        .is_none());
}

fn main() {
    let file = File::open(args().nth(1).unwrap()).unwrap();
    let mmap = unsafe { memmap::Mmap::map(&file).unwrap() };
    let mut reader = Reader::new(&mmap[..]);
    let gc_disc: structs::GcDisc = reader.read(());

    let filenames = [
        "Metroid1.pak",
        "Metroid2.pak",
        "Metroid3.pak",
        "Metroid4.pak",
        "metroid5.pak",
        "Metroid6.pak",
        "Metroid7.pak",
        "Metroid8.pak",
    ];

    let mut pickup_table = HashMap::new();
    let mut cmdl_aabbs = HashMap::new();
    let mut locations: Vec<Vec<RoomInfo>> = Vec::new();

    for f in &filenames {
        let file_entry = gc_disc.find_file(f).unwrap();
        let pak = match *file_entry.file().unwrap() {
            structs::FstEntryFile::Pak(ref pak) => pak.clone(),
            structs::FstEntryFile::Unknown(ref reader) => reader.clone().read(()),
            _ => panic!(),
        };

        let resources = &pak.resources;

        let mut res_db = ResourceDb::new();
        for res in resources.iter() {
            res_db.add_resource(res.into_owned());
        }

        let mrea_name_strg_map: HashMap<_, _> = resources
            .iter()
            .find(|res| res.fourcc() == b"MLVL".into())
            .unwrap()
            .kind
            .as_mlvl()
            .unwrap()
            .areas
            .iter()
            .map(|area| (area.mrea, area.area_name_strg))
            .collect();

        let mut mapw_res = resources
            .iter()
            .find(|res| res.fourcc() == b"MAPW".into())
            .unwrap()
            .into_owned();
        let mut mapw = mapw_res.kind.as_mapw_mut().unwrap().area_maps.iter();

        locations.push(vec![]);
        let pak_locations = locations.last_mut().unwrap();

        for res in resources.iter() {
            if res.fourcc() != b"MREA".into() {
                continue;
            };

            let mut res = res.into_owned();
            let mrea = res.kind.as_mrea_mut().unwrap();

            let model_count = mrea.world_model_count;

            let scly = mrea.scly_section_mut();

            let mut total_object_size = 0;
            for layer in scly.layers.as_mut_vec() {
                for obj in layer.objects.as_mut_vec() {
                    total_object_size += obj.property_data.size();
                }
            }

            let mut size_index =
                (model_count as f32 / 544.0 + (total_object_size as f32 / 150536.0)) / 2.0;

            let mut room_locations = vec![];
            let mut room_removals = HashMap::new();
            let mut door_locations = vec![];

            let target_mapa_id = mapw.next().unwrap().into_owned();
            let target_mapa = resources
                .iter()
                .find(|res| res.fourcc() == b"MAPA".into() && res.file_id == target_mapa_id)
                .unwrap()
                .into_owned();
            let mapa_id = &ResId::<res_id::MAPA>::new(target_mapa.file_id);
            let room_id = ResId::<res_id::MREA>::new(res.file_id);

            // println!("\n\n");
            // let mut hudmemos = HashSet::new();
            // let mut layer_changers: Vec<(u32,u32,u32)> = Vec::new();
            // let mut enable_disable: Vec<(u32,bool)> = Vec::new();
            for (layer_num, scly_layer) in scly.layers.iter().enumerate() {
                // for obj in scly_layer.objects.iter() {
                //     if obj.property_data.is_hud_memo() {
                //         let memo = obj.property_data.as_hud_memo().unwrap();
                //         hudmemos.insert(memo.strg);
                //     }
                // }
                // for obj in scly_layer.objects.iter() {
                //     if obj.property_data.is_pickup() {
                //         let pickup = obj.property_data.as_pickup().unwrap();
                //         if pickup.drop_rate == 0.0 {
                //             continue;
                //         }
                //         if pickup.kind == PickupType::HealthRefill.kind() {
                //             println!("Health     | {}% | {}", pickup.drop_rate, pickup.curr_increase);
                //         } else if pickup.kind == PickupType::PowerBomb.kind() && pickup.max_increase == 0 {
                //             println!("Power Bomb | {}% | {}", pickup.drop_rate, pickup.curr_increase);
                //         } else if pickup.kind == PickupType::Missile.kind() && pickup.max_increase == 0 {
                //             println!("Missile    | {}% | {}", pickup.drop_rate, pickup.curr_increase);
                //         }
                //     }
                // }

                // for obj in scly_layer.objects.iter() {
                //     if obj.property_data.is_special_function() {
                //         let sf = obj.property_data.as_special_function().unwrap();
                //         if sf.type_ == 16 {
                //             layer_changers.push((obj.instance_id, sf.layer_change_room_id, sf.layer_change_layer_id));
                //         }
                //     }
                //     for conn in obj.connections.iter() {
                //         if conn.message == structs::ConnectionMsg::INCREMENT {
                //             enable_disable.push((conn.target_object_id,true));
                //         } else if conn.message == structs::ConnectionMsg::DECREMENT {
                //             enable_disable.push((conn.target_object_id,false));
                //         }
                //     }
                // }

                // trace door resources //
                for obj in scly_layer.objects.iter() {
                    let obj = obj.into_owned();
                    if !obj.property_data.is_dock() {
                        continue;
                    }
                    let dock = obj.property_data.as_dock().unwrap();

                    let mut door_force_locations: Vec<ScriptObjectLocation> = Vec::new();
                    let mut door_shield_locations: Vec<ScriptObjectLocation> = Vec::new();

                    let mut door_loc: Option<ScriptObjectLocation> = None;
                    let mut door_rotation: Option<[f32; 3]> = None;

                    for obj in scly_layer.objects.iter() {
                        if obj.property_data.is_actor() {
                            let actor = obj.property_data.as_actor().unwrap();

                            if f32::abs(actor.position[0] - dock.position[0]) > 4.0
                                || f32::abs(actor.position[1] - dock.position[1]) > 4.0
                                || f32::abs(actor.position[2] - dock.position[2]) > 4.0
                            {
                                continue; // not associated with dock number
                            }

                            let name = actor.name.to_str().unwrap();
                            if name.starts_with("Actor_DoorShield") && !name.contains("Key") {
                                // unlock_shield
                                door_shield_locations.push(ScriptObjectLocation {
                                    layer: layer_num as u32,
                                    instance_id: obj.instance_id,
                                });
                            } else if name.starts_with("Actor_DoorShield_Key") {
                                // key_shield
                                door_shield_locations.push(ScriptObjectLocation {
                                    layer: layer_num as u32,
                                    instance_id: obj.instance_id,
                                });
                            }
                        } else if obj.property_data.is_damageable_trigger() {
                            let damageable_trigger =
                                obj.property_data.as_damageable_trigger().unwrap();
                            if f32::abs(damageable_trigger.position[0] - dock.position[0]) > 4.0
                                || f32::abs(damageable_trigger.position[1] - dock.position[1]) > 4.0
                                || f32::abs(damageable_trigger.position[2] - dock.position[2]) > 4.0
                            {
                                continue; // not associated with dock number
                            }

                            let name = damageable_trigger.name.to_str().unwrap();
                            if name.contains("DoorUnlock") {
                                // forceunlock
                                door_force_locations.push(ScriptObjectLocation {
                                    layer: layer_num as u32,
                                    instance_id: obj.instance_id,
                                });
                            } else if name.contains("DoorKey") {
                                // forcekey
                                door_force_locations.push(ScriptObjectLocation {
                                    layer: layer_num as u32,
                                    instance_id: obj.instance_id,
                                });
                            }
                        } else if obj.property_data.is_door() {
                            let door = obj.property_data.as_door().unwrap();
                            if f32::abs(door.position[0] - dock.position[0]) > 4.0
                                || f32::abs(door.position[1] - dock.position[1]) > 4.0
                                || f32::abs(door.position[2] - dock.position[2]) > 4.0
                            {
                                continue; // not associated with dock number
                            }

                            if door_loc.is_some() {
                                panic!("multiple valid doors in 0x{:X}", room_id.to_u32());
                            }

                            door_loc = Some(ScriptObjectLocation {
                                layer: layer_num as u32,
                                instance_id: obj.instance_id,
                            });
                            door_rotation = Some(door.rotation.into());
                        }
                    }

                    door_locations.push(DoorLocation {
                        door_location: door_loc,
                        door_rotation,
                        door_force_locations,
                        door_shield_locations,
                        dock_number: dock.dock_index,
                        dock_position: dock.position.into(),
                        dock_scale: dock.scale.into(),
                    });
                }

                // trace pickup resources //
                for obj in scly_layer.objects.iter() {
                    let obj = obj.into_owned();
                    let pickup = if let Some(pickup) = obj.property_data.as_pickup() {
                        pickup
                    } else {
                        continue;
                    };

                    let pickup_model = if let Some(pm) = pickup_model_for_pickup(&pickup) {
                        pm
                    } else {
                        continue;
                    };

                    if pickup_model != PickupModel::HealthRefill
                        && pickup_model != PickupModel::MissileRefill
                        && pickup_model != PickupModel::PowerBombRefill
                    {
                        let obj_loc = ScriptObjectLocation {
                            instance_id: obj.instance_id,
                            layer: layer_num as u32,
                        };
                        let (pickup_loc, removals) =
                            extract_pickup_location(res.file_id, scly, &obj, obj_loc);

                        for loc in removals {
                            room_removals
                                .entry(loc.layer)
                                .or_insert_with(Vec::new)
                                .push(loc.instance_id);
                        }
                        room_locations.push(pickup_loc);
                    }

                    // XXX There's a couple of pickups where the first occurances don't have scans,
                    // so skip those for the pickup_table
                    if (pickup_model == PickupModel::Missile
                        || pickup_model == PickupModel::EnergyTank)
                        && pickup.actor_params.scan_params.scan == 0xFFFFFFFF
                    {
                        continue;
                    }

                    if pickup_table.contains_key(&pickup_model) {
                        continue;
                    }

                    pickup_table.insert(pickup_model, extract_pickup_data(scly, &obj, &mut res_db));

                    if pickup.cmdl != u32::MAX {
                        // Add an aabb entry for this pickup's cmdl
                        cmdl_aabbs.entry(pickup.cmdl).or_insert_with(|| {
                            let cmdl_key = ResourceKey::from(pickup.cmdl);
                            // Cmdls are compressed
                            let res_data = res_db.map[&cmdl_key].data.decompress();
                            let cmdl: Cmdl = Reader::new(&res_data).read(());
                            let aabb = cmdl.maab;
                            // Convert from GenericArray to [f32; 6]
                            [aabb[0], aabb[1], aabb[2], aabb[3], aabb[4], aabb[5]]
                        });
                    }
                }
            }
            // for (id, room_id, layer_num) in layer_changers.iter() {
            //     for (target_id, is_enable) in enable_disable.iter() {
            //         if id & 0x0000FFFF == target_id & 0x0000FFFF {
            //             println!("{}, {}, {}, {:?}", res.file_id, room_id, layer_num, is_enable);
            //         }
            //     }
            // }

            {
                let strg_id = mrea_name_strg_map[&ResId::<res_id::MREA>::new(res.file_id)];

                let name = match f {
                    &"Metroid8.pak" => "End Cinema\0".to_string(),
                    _ => {
                        let strg: structs::Strg = res_db.map[&ResourceKey::from(strg_id)]
                            .data
                            .data
                            .clone()
                            .read(());
                        let name = strg
                            .string_tables
                            .iter()
                            .next()
                            .unwrap()
                            .strings
                            .iter()
                            .next()
                            .unwrap()
                            .into_owned()
                            .into_string();
                        name
                    }
                };

                if [
                    0x2398E906, // artifact temple
                    0x5F2EB7B6,
                ]
                .contains(&res.file_id)
                {
                    size_index = 1.0;
                }

                // consolidate redundant locations
                // let mut consolidated_door_locations: Vec<DoorLocation> = Vec::new();
                // for door_location in door_locations {
                //     let existing_location = consolidated_door_locations.iter().find(|dl| dl.dock_number == door_location.dock_number);

                //     // This dock number has no data yet, put what we have
                //     if existing_location.is_none() {
                //         consolidated_door_locations.push(door_location);
                //         continue;
                //     }

                //     // Update the existing data
                //     let mut existing_location = existing_location.unwrap().clone();
                //     if door_location.door_force_location.is_some() {
                //         existing_location.door_force_location = door_location.door_force_location;
                //     }
                //     if door_location.door_shield_location.is_some() {
                //         existing_location.door_shield_location = door_location.door_shield_location;
                //     }

                //     assert_eq!(existing_location.dock_position,door_location.dock_position);
                //     assert_eq!(existing_location.dock_scale,door_location.dock_scale);

                //     consolidated_door_locations.retain(|dl| dl.dock_number != existing_location.dock_number);
                //     consolidated_door_locations.push(existing_location);
                // }

                // let mut hudmemos2 = Vec::new();
                // for strg_id in hudmemos {
                //     if strg_id.to_u32() == 0xFFFFFFFF || strg_id.to_u32() == 0 || strg_id.to_u32() == 0x6B01D75C {
                //         continue;
                //     }

                //     let key = ResourceKey::from(strg_id);
                //     if !res_db.map.contains_key(&key) {
                //         hudmemos2.push(format!("0x{:X} not found", strg_id.to_u32()));
                //         continue;
                //     }

                //     let strg: structs::Strg = res_db.map[&key]
                //         .data.data.clone().read(());

                //     let text = strg
                //         .string_tables.iter().next().unwrap()
                //         .strings.iter().next().unwrap()
                //         .into_owned().into_string();

                //     if text.contains("acquired!") {
                //         continue;;
                //     }

                //     if text.contains("Energy fully replenished") {
                //         continue;
                //     }

                //     hudmemos2.push(text);
                // }

                // if !hudmemos2.is_empty() {
                //     println!("=== {} ===", name.replace("\0", ""));
                //     for text in hudmemos2 {
                //         let text = text.replace("&just=center;", "");
                //         let text = text.replace("\0", "");
                //         println!("{}", text);
                //     }
                //     println!("\n\n\n");
                // }

                pak_locations.push(RoomInfo {
                    room_id,
                    name,
                    name_id: strg_id,
                    mapa_id: *mapa_id,
                    pickups: room_locations,
                    // doors: consolidated_door_locations,
                    doors: door_locations,
                    objects_to_remove: room_removals,
                    size_index,
                })
            }
        }
    }

    // Special case of custom CMDLs
    let suit_aabb = *cmdl_aabbs
        .get(&ResId::<res_id::CMDL>::new(
            resource_info!("Node1_11.CMDL").res_id,
        ))
        .unwrap();
    assert!(cmdl_aabbs
        .insert(custom_asset_ids::PHAZON_SUIT_CMDL, suit_aabb)
        .is_none());
    assert!(cmdl_aabbs
        .insert(custom_asset_ids::NOTHING_CMDL, suit_aabb)
        .is_none());
    assert!(cmdl_aabbs
        .insert(custom_asset_ids::ZOOMER_CMDL, suit_aabb)
        .is_none());
    let cog_aabb = [
        suit_aabb[0],
        suit_aabb[1],
        suit_aabb[2] + 0.5,
        suit_aabb[3],
        suit_aabb[4],
        suit_aabb[5] + 0.5,
    ];
    assert!(cmdl_aabbs
        .insert(custom_asset_ids::COG_CMDL, cog_aabb)
        .is_none());

    let missile_aabb = *cmdl_aabbs
        .get(&ResId::<res_id::CMDL>::new(
            resource_info!("Node1_36_0.CMDL").res_id,
        ))
        .unwrap();
    assert!(cmdl_aabbs
        .insert(custom_asset_ids::SHINY_MISSILE_CMDL, missile_aabb)
        .is_none());

    let visor_aabb = *cmdl_aabbs
        .get(&ResId::<res_id::CMDL>::new(
            resource_info!("Node1_39_1.CMDL").res_id,
        ))
        .unwrap();
    assert!(cmdl_aabbs
        .insert(custom_asset_ids::THERMAL_CMDL, visor_aabb)
        .is_none());
    assert!(cmdl_aabbs
        .insert(custom_asset_ids::XRAY_CMDL, visor_aabb)
        .is_none());
    assert!(cmdl_aabbs
        .insert(custom_asset_ids::COMBAT_CMDL, visor_aabb)
        .is_none());

    create_gamecube(&mut pickup_table);
    create_nothing(&mut pickup_table);
    create_zoomer(&mut pickup_table);
    create_cog(&mut pickup_table);
    create_shiny_missile(&mut pickup_table);
    create_thermal_visor(&mut pickup_table);
    create_xray_visor(&mut pickup_table);
    create_combat_visor(&mut pickup_table);
    create_flamethrower(&mut pickup_table);
    // create_power_beam(&mut pickup_table);

    println!("// This file is generated by bin/resource_tracing.rs");
    println!();
    println!();

    println!("pub const ROOM_INFO: &[(&str, &[RoomInfo]); 8] = &[");
    for (fname, locations) in filenames.iter().zip(locations.into_iter()) {
        // println!("    // {}", fname);
        println!("    ({:?}, &[", fname);
        for room_info in locations {
            println!("        RoomInfo {{");
            println!(
                "            room_id: ResId::<res_id::MREA>::new(0x{:08X}),",
                room_info.room_id.to_u32()
            );
            println!(
                "            name: {:?},",
                &room_info.name[..(room_info.name.len() - 1)]
            );
            println!(
                "            name_id: ResId::<res_id::STRG>::new(0x{:08X}),",
                room_info.name_id.to_u32()
            );
            println!(
                "            mapa_id: ResId::<res_id::MAPA>::new(0x{:08X}),",
                room_info.mapa_id.to_u32()
            );
            println!("            size_index: {:.4},", room_info.size_index);
            println!("            pickup_locations: &[");
            for location in room_info.pickups {
                println!("                PickupLocation {{");
                println!("                    location: {:?},", location.location);
                println!(
                    "                    attainment_audio: {:?},",
                    location.attainment_audio
                );
                println!("                    hudmemo: {:?},", location.hudmemo);
                println!(
                    "                    memory_relay: {:?},",
                    location.memory_relay
                );
                println!("                    position: {:?},", location.position);
                if location.post_pickup_relay_connections.is_empty() {
                    println!("                    post_pickup_relay_connections: &[]");
                } else {
                    println!("                    post_pickup_relay_connections: &[");
                    for conn in &location.post_pickup_relay_connections {
                        println!("                        Connection {{");
                        println!("                            state: {:?},", conn.state);
                        println!("                            message: {:?},", conn.message);
                        println!(
                            "                            target_object_id: 0x{:x},",
                            conn.target_object_id
                        );
                        println!("                        }},");
                    }
                    println!("                    ],");
                }
                println!("                }},");
            }
            println!("            ],");
            println!("            door_locations: &[");
            for door in room_info.doors {
                println!("                DoorLocation {{");
                println!(
                    "                    door_location: {:?},",
                    door.door_location
                );
                println!(
                    "                    door_rotation: {:?},",
                    door.door_rotation
                );
                println!(
                    "                    door_force_locations: &{:?},",
                    door.door_force_locations
                );
                println!(
                    "                    door_shield_locations: &{:?},",
                    door.door_shield_locations
                );
                println!("                    dock_number: {:?},", door.dock_number);
                println!(
                    "                    dock_position: {:?},",
                    door.dock_position
                );
                println!("                    dock_scale: {:?},", door.dock_scale);
                println!("                }},");
            }
            println!("            ],");

            if room_info.objects_to_remove.is_empty() {
                println!("            objects_to_remove: &[],");
            } else {
                println!("            objects_to_remove: &[");
                let mut objects_to_remove: Vec<_> = room_info.objects_to_remove.iter().collect();
                objects_to_remove.sort_by_key(|&(k, _)| k);
                for otr in objects_to_remove {
                    println!("                ObjectsToRemove {{");
                    println!("                    layer: {},", otr.0);
                    println!("                    instance_ids: &{:?},", otr.1);
                    println!("                }},");
                }
                println!("            ],");
            }
            println!("        }},");
        }
        println!("    ]),");
    }
    println!("];");

    let mut cmdl_aabbs: Vec<_> = cmdl_aabbs.iter().collect();

    let ice_trab_cmdl = ResId::<res_id::CMDL>::new(0xA3108E43);
    let ice_trap_aabb = [
        f32::from_bits(0xBF09348B),
        f32::from_bits(0xBFB99CD0),
        f32::from_bits(0xBEAA8B6E),
        f32::from_bits(0x3F08ADAC),
        f32::from_bits(0x3EDC6CFC),
        f32::from_bits(0x3F003E64),
    ];

    cmdl_aabbs.push((&ice_trab_cmdl, &ice_trap_aabb));
    cmdl_aabbs.sort_by_key(|&(k, _)| k);
    println!(
        "const PICKUP_CMDL_AABBS: [(u32, [u32; 6]); {}] = [",
        cmdl_aabbs.len()
    );
    for (cmdl_id, aabb) in cmdl_aabbs {
        let aabb: [u32; 6] = unsafe { mem::transmute(*aabb) };
        println!(
            "    (0x{:08X}, [0x{:08X}, 0x{:08X}, 0x{:08X}, 0x{:08X}, 0x{:08X}, 0x{:08X}]),",
            cmdl_id.to_u32(),
            aabb[0],
            aabb[1],
            aabb[2],
            aabb[3],
            aabb[4],
            aabb[5]
        );
    }
    println!("];");

    println!("impl PickupType");
    println!("{{");

    println!("    pub fn attainment_audio_file_name(&self) -> &'static str");
    println!("    {{");
    println!("        match self {{");
    for pt in PickupType::iter() {
        if pt == PickupType::IceTrap {
            continue;
        }

        let pm = PickupModel::from_type(pt);
        let filename = stdstr::from_utf8(&pickup_table[&pm].attainment_audio_file_name).unwrap();
        println!("            PickupType::{:?} => {:?},", pt, filename);
    }
    println!("            PickupType::IceTrap => \"/audio/evt_x_event_00.dsp\\0\", // being a bit trolly here");
    println!("        }}");
    println!("    }}");

    println!("}}");
    println!("impl PickupModel");
    println!("{{");

    println!("    pub fn dependencies(&self) -> &'static [(u32, FourCC)]");
    println!("    {{");
    println!("        match self {{");
    for pm in PickupModel::iter() {
        if pm == PickupModel::IceTrap {
            let string = "PickupModel::IceTrap => {
                const DATA: &[(u32, FourCC)] = &[
                    (0x1544D478, FourCC::from_bytes(b\"TXTR\")),
                    (0x394D3877, FourCC::from_bytes(b\"ANIM\")),
                    (0x454FB170, FourCC::from_bytes(b\"TXTR\")),
                    (0x4B26EFDA, FourCC::from_bytes(b\"EVNT\")),
                    (0xAF9DEFBE, FourCC::from_bytes(b\"CINF\")),
                    (0x5E027EA1, FourCC::from_bytes(b\"TXTR\")),
                    (0xA0DA476B, FourCC::from_bytes(b\"PART\")),
                    (0x29782CA6, FourCC::from_bytes(b\"TXTR\")),
                    (0x0DEB9456, FourCC::from_bytes(b\"PART\")),
                    (0xA4B75FDE, FourCC::from_bytes(b\"TXTR\")),
                    (0x0F7E19D9, FourCC::from_bytes(b\"TXTR\")),
                    (0x09E3FB4C, FourCC::from_bytes(b\"TXTR\")),
                    (0xA3108E43, FourCC::from_bytes(b\"CMDL\")),
                    (0xFEBBC197, FourCC::from_bytes(b\"CSKR\")),
                ];
                DATA
            },";

            println!("{}", string);
            continue;
        }

        let mut deps: Vec<_> = pickup_table[&pm].deps.iter().collect();

        let res = ResourceKey::new(0x29782CA6, FourCC::from_bytes(b"TXTR"));
        deps.push(&res);

        let res = ResourceKey::new(0xA0DA476B, FourCC::from_bytes(b"PART"));
        deps.push(&res);

        let res = ResourceKey::new(0x5E027EA1, FourCC::from_bytes(b"TXTR"));
        deps.push(&res);

        let res = ResourceKey::new(0x79B5BD15, FourCC::from_bytes(b"TXTR"));
        deps.push(&res);

        let res = ResourceKey::new(0xE224FF03, FourCC::from_bytes(b"TXTR"));
        deps.push(&res);

        let res = ResourceKey::new(0x57fe7e67, FourCC::from_bytes(b"AGSC"));
        deps.push(&res);

        deps.sort();
        println!("            PickupModel::{:?} => {{", pm);
        println!("                const DATA: &[(u32, FourCC)] = &[");
        for dep in deps {
            println!(
                "                    (0x{:08X}, FourCC::from_bytes(b\"{}\")),",
                dep.file_id, dep.fourcc
            );
        }
        println!("                ];");
        println!("                DATA");
        println!("            }},");
    }
    println!("        }}");
    println!("    }}");

    const BYTES_PER_LINE: usize = 8;
    println!("    fn raw_pickup_data(&self) -> &'static [u8]");
    println!("    {{");
    println!("        match self {{");
    for pm in PickupModel::iter() {
        if pm == PickupModel::IceTrap {
            let string = "           PickupModel::IceTrap => &[
                0x00, 0x00, 0x00, 0x12, 0x49, 0x63, 0x65, 0x20,
                0x54, 0x72, 0x61, 0x70, 0x00, 0xC3, 0x18, 0x19,
                0x25, 0x41, 0xCB, 0xC3, 0x2E, 0xC3, 0x0C, 0xB0,
                0x2F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x3F, 0xA0, 0x00,
                0x00, 0x3F, 0xA0, 0x00, 0x00, 0x3F, 0xA0, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x42, 0xC8, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0xA3, 0x10, 0x8E, 0x43, 0xDE, 0xAF, 0x00,
                0x72, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x0E, 0x00, 0x00, 0x00,
                0x0E, 0x01, 0x3F, 0x80, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x3F, 0x80, 0x00, 0x00, 0x41, 0xA0,
                0x00, 0x00, 0x3F, 0x80, 0x00, 0x00, 0x3F, 0x80,
                0x00, 0x00, 0x3F, 0x80, 0x00, 0x00, 0x3F, 0x80,
                0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00,
                0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x04, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0x01, 0x3F, 0x80, 0x00,
                0x00, 0x3F, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0F, 0x01,
                0x00, 0x00, 0x3F, 0x80, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF,
            ]";

            println!("{}", string);
            continue;
        }

        println!("            PickupModel::{:?} => &[", pm);
        let pickup_bytes = &pickup_table[&pm].bytes;
        for y in 0..pickup_bytes.len().div_ceil(BYTES_PER_LINE) {
            let len = ::std::cmp::min(BYTES_PER_LINE, pickup_bytes.len() - y * BYTES_PER_LINE);
            print!("               ");
            for x in 0..len {
                print!(" 0x{:02X},", pickup_bytes[y * BYTES_PER_LINE + x]);
            }
            println!();
        }
        println!("            ],");
    }
    println!("        }}");
    println!("    }}");

    println!("}}");
}
