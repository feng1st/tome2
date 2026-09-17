//! Content rendering: tilemap sprites (actual game content, NOT the HUD).

use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ScalingMode, Viewport};
use bevy::prelude::*;

use crate::colors;
use crate::data::GameData;
use crate::game::{GridPos, Monster, Player, TurnState};
use crate::item::{FloorGold, FloorItem, Inventory};
use crate::map::{self, Map};

pub const TILE: f32 = 16.0;
pub const TILE_PX: u32 = 16;
pub const FG_COLS: u32 = 16;
pub const FG_ROWS: u32 = 6;
pub const BG_COLS: u32 = 16;
pub const BG_ROWS: u32 = 1;

/// World area is 1280x640; the rest of the 1600x720 window is HUD.
pub const WORLD_W: f32 = 1280.0;
pub const WORLD_H: f32 = 640.0;
pub const HUD_TOP_PX: f32 = 80.0;
pub const HUD_RIGHT_PX: f32 = 320.0;

#[derive(Resource, Clone)]
pub struct TileAssets {
    pub font: Handle<Font>,
    pub bg_image: Handle<Image>,
    pub bg_layout: Handle<TextureAtlasLayout>,
    pub fg_image: Handle<Image>,
    pub fg_layout: Handle<TextureAtlasLayout>,
}

pub fn load_tile_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    commands.insert_resource(TileAssets {
        font: asset_server.load("fonts/LiberationMono-Regular.ttf"),
        bg_image: asset_server.load("tiles/background.png"),
        bg_layout: layouts.add(TextureAtlasLayout::from_grid(
            UVec2::splat(TILE_PX),
            BG_COLS,
            BG_ROWS,
            None,
            None,
        )),
        fg_image: asset_server.load("tiles/foreground.png"),
        fg_layout: layouts.add(TextureAtlasLayout::from_grid(
            UVec2::splat(TILE_PX),
            FG_COLS,
            FG_ROWS,
            None,
            None,
        )),
    });
}

/// The camera that renders the actual game content (tilemap, actors).
#[derive(Component)]
pub struct WorldCam;

pub fn setup_cameras(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        WorldCam,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: WORLD_W,
                height: WORLD_H,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
    // The HUD camera: UI is rendered full-window on top of the content.
    // It is restricted to render layer 1 so it does NOT render the world
    // sprites (default layer 0) a second time; UI nodes ignore RenderLayers.
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        RenderLayers::layer(1),
        IsDefaultUiCamera,
    ));
}

/// Keep the content camera's viewport out of the HUD areas.
pub fn update_camera_viewport(mut cam: Query<&mut Camera, With<WorldCam>>, window: Query<&Window>) {
    let (Ok(mut cam), Ok(window)) = (cam.single_mut(), window.single()) else {
        return;
    };
    let sf = window.resolution.scale_factor();
    let phys = window.resolution.physical_size();
    let right = (HUD_RIGHT_PX * sf) as u32;
    let top = (HUD_TOP_PX * sf) as u32;
    cam.viewport = Some(Viewport {
        physical_position: UVec2::new(0, top),
        physical_size: UVec2::new(phys.x.saturating_sub(right), phys.y.saturating_sub(top)),
        ..default()
    });
}

pub fn grid_to_world(x: i32, y: i32, z: f32) -> Vec3 {
    Vec3::new(
        (x as f32 - (map::MAP_W as f32 - 1.0) / 2.0) * TILE,
        ((map::MAP_H as f32 - 1.0) / 2.0 - y as f32) * TILE,
        z,
    )
}

pub fn glyph_index(ch: char) -> usize {
    let c = ch as usize;
    if (32..=126).contains(&c) {
        c - 32
    } else {
        ('?' as usize) - 32
    }
}

/// A foreground glyph sprite (transparent background, tinted).
pub fn glyph_sprite(tiles: &TileAssets, ch: char, color_idx: u8) -> Sprite {
    let mut s = Sprite::from_atlas_image(
        tiles.fg_image.clone(),
        TextureAtlas {
            layout: tiles.fg_layout.clone(),
            index: glyph_index(ch),
        },
    );
    s.color = colors::palette(color_idx);
    s
}

#[derive(Clone, Copy, PartialEq)]
struct CellKey {
    bg: Option<usize>,
    fg: Option<(usize, u8, bool)>, // glyph, color, dimmed
}

#[derive(Resource)]
pub struct TileGrid {
    pub bg: Vec<Entity>,
    pub fg: Vec<Entity>,
    cache: Vec<CellKey>,
}

/// Spawn the two sprite layers (background + foreground) for every map cell.
pub fn spawn_tile_grid(commands: &mut Commands, tiles: &TileAssets) -> TileGrid {
    let mut bg = Vec::with_capacity((map::MAP_W * map::MAP_H) as usize);
    let mut fg = Vec::with_capacity(bg.capacity());
    for y in 0..map::MAP_H {
        for x in 0..map::MAP_W {
            let b = commands
                .spawn((
                    crate::game::GameEntity,
                    Sprite::from_atlas_image(
                        tiles.bg_image.clone(),
                        TextureAtlas {
                            layout: tiles.bg_layout.clone(),
                            index: colors::BG_DARK,
                        },
                    ),
                    Transform::from_translation(grid_to_world(x, y, 0.0)),
                    Visibility::Hidden,
                ))
                .id();
            let f = commands
                .spawn((
                    crate::game::GameEntity,
                    Sprite::from_atlas_image(
                        tiles.fg_image.clone(),
                        TextureAtlas {
                            layout: tiles.fg_layout.clone(),
                            index: 0,
                        },
                    ),
                    Transform::from_translation(grid_to_world(x, y, 1.0)),
                    Visibility::Hidden,
                ))
                .id();
            bg.push(b);
            fg.push(f);
        }
    }
    TileGrid {
        bg,
        fg,
        cache: vec![
            CellKey {
                bg: Some(usize::MAX),
                fg: None,
            };
            (map::MAP_W * map::MAP_H) as usize
        ],
    }
}

/// What to draw for one map cell.
fn cell_render(gd: &GameData, map: &Map, x: i32, y: i32) -> CellKey {
    let i = Map::idx(x, y);
    if !map.explored[i] {
        return CellKey { bg: None, fg: None };
    }
    // Shop entrances show their store glyph on the floor. Memorized
    // glowing cells stay bright (see the terrain branch below).
    if let Some(mark) = map.shops.get(&i) {
        let bright = map.visible[i] || map.lit[i];
        let bg = if bright {
            Some(colors::BG_FLOOR_LIT)
        } else {
            None
        };
        return CellKey {
            bg,
            fg: Some((glyph_index(mark.ch), mark.color, !bright)),
        };
    }
    // Unknown traps look like ordinary floor.
    let t = if map.terrain_at(x, y) == map::T_TRAP && !map.known_traps.contains(&i) {
        map::T_FLOOR
    } else {
        map.display_terrain(gd, x, y)
    };
    let def = gd.terrain(t);
    let glyph = glyph_index(def.ch.chars().next().unwrap_or(' '));
    if map.visible[i] {
        let bg = if def.is_floor {
            colors::BG_FLOOR_LIT
        } else {
            colors::BG_DARK
        };
        CellKey {
            bg: Some(bg),
            fg: Some((glyph, def.color, false)),
        }
    } else if map.lit[i] {
        // Memorized glowing grids stay lit -- the original draws
        // CAVE_GLOW|CAVE_MARK grids (out-of-sight but self-lit) at
        // nearly full brightness, which is why the whole village is
        // uniformly visible.
        let bg = if def.is_floor {
            Some(colors::BG_FLOOR_LIT)
        } else {
            None
        };
        CellKey {
            bg,
            fg: Some((glyph, def.color, false)),
        }
    } else {
        // Remembered dark grid: no background, dimmed glyph.
        CellKey {
            bg: None,
            fg: Some((glyph, def.color, true)),
        }
    }
}

/// Sync tile sprites with the map state (recomputes FOV first if needed).
/// Blinded players see only their own cell. Monsters carrying light
/// (HAS_LITE) illuminate the 3x3 around them, visible in line of sight.
pub fn sync_tiles(
    gd: Res<GameData>,
    inv: Res<Inventory>,
    ps: Res<crate::game::PlayerState>,
    opts: Res<crate::options::Options>,
    mut map: ResMut<Map>,
    mut turn: ResMut<TurnState>,
    mut grid: ResMut<TileGrid>,
    player: Query<&GridPos, With<Player>>,
    monsters: Query<(&GridPos, &Monster), Without<Player>>,
    mut sprites: Query<&mut Sprite>,
    mut vis: Query<&mut Visibility>,
) {
    if turn.fov_dirty {
        if let Ok(p) = player.single() {
            let light = crate::game::player_lite_ex(
                &ps,
                &inv,
                &gd,
                turn.running.is_some(),
                opts.view_reduce_lite,
            );
            map::compute_fov(&mut map, &gd, p.x, p.y, light);
            if ps.blind > 0 {
                for v in map.visible.iter_mut() {
                    *v = false;
                }
                map.visible[Map::idx(p.x, p.y)] = true;
            } else {
                // Monster-carried light shines in the dark (cave.cc
                // update_mon_lite): a lit 3x3 around each HAS_LITE
                // monster within MAX_SIGHT, on grids the player can see;
                // a wall only lights up when the monster itself is in
                // line of sight.
                for (mp, m) in monsters.iter() {
                    if !gd.monsters[m.def].has("HAS_LITE") {
                        continue;
                    }
                    if map::chebyshev(p.x, p.y, mp.x, mp.y) > map::MAX_SIGHT + 1 {
                        continue;
                    }
                    let invis = !map::line_of_sight(&map, &gd, p.x, p.y, mp.x, mp.y);
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            let (x, y) = (mp.x + dx, mp.y + dy);
                            if !Map::in_bounds(x, y) {
                                continue;
                            }
                            let i = Map::idx(x, y);
                            // Only grids already in the player's view.
                            if !map.visible[i] {
                                continue;
                            }
                            if invis && gd.terrain(map.terrain_at(x, y)).no_vision {
                                continue;
                            }
                            map.visible[i] = true;
                            map.explored[i] = true;
                        }
                    }
                }
            }
        }
        turn.fov_dirty = false;
    }
    for y in 0..map::MAP_H {
        for x in 0..map::MAP_W {
            let i = Map::idx(x, y);
            let key = cell_render(&gd, &map, x, y);
            if grid.cache[i] == key {
                continue;
            }
            grid.cache[i] = key;
            if let Ok(mut v) = vis.get_mut(grid.bg[i]) {
                *v = if key.bg.is_some() {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
            }
            if let Some(idx) = key.bg {
                if let Ok(mut s) = sprites.get_mut(grid.bg[i]) {
                    if let Some(atlas) = &mut s.texture_atlas {
                        atlas.index = idx;
                    }
                }
            }
            if let Ok(mut v) = vis.get_mut(grid.fg[i]) {
                *v = if key.fg.is_some() {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
            }
            if let Some((glyph, color, dim)) = key.fg {
                if let Ok(mut s) = sprites.get_mut(grid.fg[i]) {
                    if let Some(atlas) = &mut s.texture_atlas {
                        atlas.index = glyph;
                    }
                    s.color = if dim {
                        colors::dimmed(color)
                    } else {
                        colors::palette(color)
                    };
                }
            }
        }
    }
}

/// Whether the player senses this monster without seeing it
/// (telepathy, infravision; EMPTY_MIND blocks full telepathy and
/// WEIRD_MIND only occasionally answers, monster2.cc:1724). `seed` keeps
/// the WEIRD_MIND roll stable for a turn/entity pair.
pub fn monster_sensed(
    def: &crate::data::MonsterDef,
    totals: &crate::item::EquipTotals,
    esp_timer: bool,
    dist: i32,
    seed: u64,
) -> bool {
    let can_esp = totals.esp.contains("ALL")
        || esp_timer
        || totals.esp.iter().any(|r| r != "ALL" && def.has(r));
    if can_esp {
        if def.has("EMPTY_MIND") {
            // No telepathy.
        } else if def.has("WEIRD_MIND") {
            // Occasional telepathy: rand_int(100) < 10.
            let h = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            if (h >> 33) % 100 < 10 {
                return true;
            }
        } else {
            return true;
        }
    }
    // Infravision picks up warm-blooded monsters in range.
    totals.infra > 0 && dist <= totals.infra && !def.has("COLD_BLOOD")
}

/// Sync actor (player + monster) sprite positions and visibility.
/// Sleeping mimics render as the object they imitate (r_info F:MIMIC).
pub fn sync_actors(
    gd: Res<GameData>,
    inv: Res<Inventory>,
    ps: Res<crate::game::PlayerState>,
    map: Res<Map>,
    mut player: Query<(&GridPos, &mut Transform), (With<Player>, Without<Monster>)>,
    mut monsters: Query<
        (
            Entity,
            &GridPos,
            &Monster,
            &mut Transform,
            &mut Visibility,
            &mut Sprite,
        ),
        (With<Monster>, Without<Player>),
    >,
) {
    let totals = inv.totals_for(&gd, &ps);
    let mut rng = crate::rng::current();
    let mut ppos = (0, 0);
    for (pos, mut tf) in &mut player {
        ppos = (pos.x, pos.y);
        tf.translation = grid_to_world(pos.x, pos.y, 3.0);
    }
    for (ent, pos, m, mut tf, mut v, mut sprite) in &mut monsters {
        tf.translation = grid_to_world(pos.x, pos.y, 2.0);
        let def = &gd.monsters[m.def];
        if m.mimic > 0 && !m.awake {
            // An asleep mimic is indistinguishable from a floor object.
            // A detection spell marks it (detect_monsters_string), which
            // reveals the imitated object even out of sight.
            let i = Map::idx(pos.x, pos.y);
            *v = if map.visible[i] || map.explored[i] || m.detected > 0 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
            let o = &gd.objects[m.mimic as usize];
            if let Some(atlas) = &mut sprite.texture_atlas {
                atlas.index = glyph_index(o.glyph());
            }
            sprite.color = colors::palette(o.color);
            continue;
        }
        if m.mimic > 0 {
            // Woken up: show the true form.
            if let Some(atlas) = &mut sprite.texture_atlas {
                atlas.index = glyph_index(def.glyph());
            }
            sprite.color = colors::palette(def.color);
        }
        // Multi-hued monsters shimmer (spells1.cc mh_attr).
        if def.has("MULTI_HUED") {
            sprite.color = colors::palette(colors::mh_attr(15, &mut rng));
        }
        let seen = if map.visible[Map::idx(pos.x, pos.y)] {
            // Invisible monsters need see-invisible to show at all.
            !def.has("INVISIBLE") || totals.see_invis || ps.tim_invis > 0
        } else if m.detected > 0 {
            true
        } else {
            let dist = map::chebyshev(pos.x, pos.y, ppos.0, ppos.1);
            if ps.tim_infra > 0 && dist <= ps.tim_infra && !def.has("COLD_BLOOD") {
                true
            } else {
                monster_sensed(
                    def,
                    &totals,
                    ps.esp_timer > 0,
                    dist,
                    ps.turn ^ ent.to_bits(),
                )
            }
        };
        *v = if seen {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// Sync floor objects and gold piles: position, glyph, visibility.
pub fn sync_items(
    gd: Res<GameData>,
    map: Res<Map>,
    inv: Res<Inventory>,
    ps: Res<crate::game::PlayerState>,
    mut items: Query<
        (
            &GridPos,
            &FloorItem,
            &mut Transform,
            &mut Visibility,
            &mut Sprite,
        ),
        (Without<Player>, Without<FloorGold>),
    >,
    mut golds: Query<
        (&GridPos, &FloorGold, &mut Transform, &mut Visibility),
        (Without<Player>, Without<FloorItem>),
    >,
) {
    for (pos, stack, mut tf, mut v, mut sprite) in &mut items {
        tf.translation = grid_to_world(pos.x, pos.y, 1.5);
        let i = Map::idx(pos.x, pos.y);
        let visible = map.visible[i];
        *v = if visible || map.explored[i] {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        let (glyph, color) = if stack.stack.len() > 1 {
            ('&', 1)
        } else if let Some(it) = stack.stack.first() {
            let o = &gd.objects[it.def];
            // Unidentified flavoured kinds show the per-game shuffled
            // colour (object1.cc flavor_init).
            let known = it.identified || inv.known.contains(&it.def);
            let color = if o.tval == crate::data::TV_RANDART {
                // Junkarts always use the per-game colour table
                // (object1.cc object_attr).
                crate::item::object_attr(&gd, it, &ps)
            } else if !known {
                crate::item::flavor_color(&gd, &ps, it.def).unwrap_or(o.color)
            } else {
                o.color
            };
            (o.glyph(), color)
        } else {
            ('?', 1)
        };
        if let Some(atlas) = &mut sprite.texture_atlas {
            atlas.index = glyph_index(glyph);
        }
        sprite.color = if visible {
            colors::palette(color)
        } else {
            colors::dimmed(color)
        };
    }
    for (pos, _gold, mut tf, mut v) in &mut golds {
        tf.translation = grid_to_world(pos.x, pos.y, 1.5);
        let i = Map::idx(pos.x, pos.y);
        *v = if map.visible[i] || map.explored[i] {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// Keep the world camera centred on the player, clamped to the map edges.
pub fn camera_follow(
    player: Query<&GridPos, With<Player>>,
    mut cam: Query<&mut Transform, With<WorldCam>>,
) {
    let (Ok(p), Ok(mut tf)) = (player.single(), cam.single_mut()) else {
        return;
    };
    let target = grid_to_world(p.x, p.y, 0.0);
    let half_map_w = map::MAP_W as f32 * TILE / 2.0;
    let half_map_h = map::MAP_H as f32 * TILE / 2.0;
    let half_view_w = WORLD_W / 2.0;
    let half_view_h = WORLD_H / 2.0;
    tf.translation.x = if half_map_w > half_view_w {
        target
            .x
            .clamp(-(half_map_w - half_view_w), half_map_w - half_view_w)
    } else {
        0.0
    };
    tf.translation.y = if half_map_h > half_view_h {
        target
            .y
            .clamp(-(half_map_h - half_view_h), half_map_h - half_view_h)
    } else {
        0.0
    };
}
