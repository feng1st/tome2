//! HUD: message log, stats panel, quit prompt, death screen.
//! All HUD elements are bevy_ui nodes, fully separated from the
//! tilemap content (which lives in the world, rendered by WorldCam).

use bevy::prelude::*;

use crate::data::GameData;
use crate::game::{GameEntity, MessageLog, PlayerState, TurnState};
use crate::item::Inventory;
use crate::render::{TileAssets, HUD_RIGHT_PX};
use crate::AppState;

#[derive(Component)]
pub struct HudStats;
#[derive(Component)]
pub struct HudMessages;
#[derive(Component)]
pub struct HudConfirm;
#[derive(Component)]
pub struct DeadRoot;

pub fn setup_hud(mut commands: Commands, tiles: Res<TileAssets>) {
    let font = TextFont {
        font: tiles.font.clone().into(),
        font_size: 15.0.into(),
        ..default()
    };
    commands
        .spawn((
            GameEntity,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
        ))
        .with_children(|root| {
            // Background for the top bar (message log area).
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    right: Val::Px(HUD_RIGHT_PX),
                    height: Val::Px(80.0),
                    ..default()
                },
                BackgroundColor(Color::BLACK),
            ));
            // Message log, top left (above the map viewport).
            root.spawn((
                HudMessages,
                Text::new(""),
                TextFont {
                    font_size: 14.0.into(),
                    ..font.clone()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(10.0),
                    top: Val::Px(6.0),
                    ..default()
                },
            ));
            // Quit confirmation prompt.
            root.spawn((
                HudConfirm,
                Text::new(""),
                TextFont {
                    font_size: 18.0.into(),
                    ..font.clone()
                },
                TextColor(Color::srgb(1.0, 0.35, 0.35)),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(10.0),
                    top: Val::Px(56.0),
                    ..default()
                },
            ));
            // Stats panel, right side.
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Px(HUD_RIGHT_PX),
                    height: Val::Percent(100.0),
                    padding: UiRect::all(Val::Px(14.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.06, 0.06, 0.09)),
            ))
            .with_children(|panel| {
                panel.spawn((
                    HudStats,
                    Text::new(""),
                    font.clone(),
                    TextColor(Color::srgb(0.88, 0.88, 0.88)),
                ));
            });
        });
}

pub fn sync_hud(
    ps: Res<PlayerState>,
    gd: Res<GameData>,
    inv: Res<Inventory>,
    plot: Res<crate::game::PlotQuest>,
    log: Res<MessageLog>,
    turn: Res<TurnState>,
    mut stats_q: Query<&mut Text, With<HudStats>>,
    mut msg_q: Query<&mut Text, (With<HudMessages>, Without<HudStats>)>,
    mut confirm_q: Query<&mut Text, (With<HudConfirm>, Without<HudStats>, Without<HudMessages>)>,
) {
    if let Ok(mut text) = stats_q.single_mut() {
        let next = if ps.level >= 50 {
            "-".to_string()
        } else {
            ps.exp_needed().to_string()
        };
        let lite = inv.totals_for(&gd, &ps).lite;
        // The main plot's current objective (q_main.cc quest states); the
        // port no longer uses the retired linear quest chain.
        let quest_line = {
            let st = |q: u32| plot.status(q);
            if plot.ultra_won || plot.won {
                "*** WINNER ***".to_string()
            } else if st(crate::game::PLOT_ULTRA_GOOD) >= crate::game::PLOT_TAKEN {
                "Destroy Melkor in the Void".to_string()
            } else if st(crate::game::PLOT_MORGOTH) >= crate::game::PLOT_TAKEN {
                "Kill Morgoth, Lord of Darkness".to_string()
            } else if st(crate::game::PLOT_SAURON) >= crate::game::PLOT_TAKEN {
                "Kill Sauron, the Sorcerer".to_string()
            } else if st(crate::game::PLOT_ONE) >= crate::game::PLOT_TAKEN {
                "Find and destroy the One Ring".to_string()
            } else if st(crate::game::PLOT_NECRO) >= crate::game::PLOT_TAKEN {
                "Kill the Necromancer of Dol Guldur".to_string()
            } else {
                String::new()
            }
        };
        let food_word = if ps.food <= 0 {
            "Starving!"
        } else if ps.food < crate::game::FOOD_WEAK {
            "Weak"
        } else if ps.food < crate::game::FOOD_HUNGRY {
            "Hungry"
        } else if ps.food >= crate::game::FOOD_FULL {
            "Full"
        } else {
            ""
        };
        let mut status = String::new();
        if ps.cut > 0 {
            status.push_str("Bleeding ");
        }
        if ps.stun > 50 {
            status.push_str("HeavyStun ");
        } else if ps.stun > 0 {
            status.push_str("Stunned ");
        }
        if ps.poison > 0 {
            status.push_str("Poisoned ");
        }
        if ps.fear > 0 {
            status.push_str("Afraid ");
        }
        if ps.blind > 0 {
            status.push_str("Blind ");
        }
        if ps.confuse > 0 {
            status.push_str("Confused ");
        }
        let status_line = if status.is_empty() {
            String::new()
        } else {
            format!("{}\n", status.trim_end())
        };
        // God, grace and possession status lines.
        let god_str = if ps.god > 0 {
            format!(
                "God:   {} ({}{})\n",
                crate::spell::god_name(ps.god),
                ps.grace,
                if ps.praying { ", praying" } else { "" }
            )
        } else {
            String::new()
        };
        let form_str = if let Some((def, hp, max)) = ps.possessed {
            format!("Form:  {} {}/{}\n", gd.monsters[def].name, hp, max)
        } else {
            String::new()
        };
        let speed = crate::game::player_speed(&ps, &inv, &gd);
        let speed_str = if speed != 0 {
            format!(" ({:+})", speed)
        } else {
            String::new()
        };
        // The location line: quest name, town/dungeon name or wilderness
        // world coordinates.
        let depth_str = if ps.wild_mode {
            format!("Wilderness ({}, {})", ps.wild_x, ps.wild_y)
        } else if ps.depth >= crate::game::PLOT_DEPTH_BASE {
            crate::game::plot_quest_name(ps.depth - crate::game::PLOT_DEPTH_BASE).to_string()
        } else if ps.depth == 0 {
            let town = gd
                .wf(gd.world.feat(ps.wild_x, ps.wild_y, &plot.destroyed_towns))
                .town();
            if town != 0 {
                gd.town(town, plot.town_destroyed(town))
                    .map(|t| t.name.clone())
                    .unwrap_or_default()
            } else {
                format!("Wilderness ({}, {})", ps.wild_x, ps.wild_y)
            }
        } else {
            let base = format!("{} (depth {})", gd.dungeon(ps.dungeon).name, ps.depth);
            if crate::game::random_town_at(&ps, ps.dungeon, ps.depth).is_some() {
                format!("{} — dungeon town", base)
            } else {
                base
            }
        };

        // Day/time of day (the game starts on day 1 at sunrise).
        let (hh, mm) = crate::game::clock_of(ps.turn);
        let day_str = format!(
            "Day {} {:02}:{:02}{}",
            crate::game::day_of(ps.turn),
            hh,
            mm,
            if crate::game::is_daytime(ps.turn) {
                ""
            } else {
                " (night)"
            }
        );

        *text = Text::new(format!(
            "{}\n{} {}\n\n\
             Level: {}\nExp:   {}\nNext:  {}\n\n\
             HP:   {}/{}\nMana: {}/{}\nSN:   {}/{}\n\n\
             STR: {:2}  INT: {:2}\nWIS: {:2}  DEX: {:2}\nCON: {:2}  CHA: {:2}\n\n\
             AC:    {}\nGold:  {}\nLight: {}\nSpeed:{}\nFood:  {} {}\n{}{}{}\
             Depth: {}\n{}\nTurn:  {}\nQuest: {}\n\n\
             g get  i inv  e equip\nd drop w wield t off\n\
              q quaff p read E eat\na wand z staff Z rod\n\
              A activ f fire m cast\nC skills B abil X action\n\
             O pray Shift+Q save/quit Ctrl+Q quest\n\
             Shift+U powers Ctrl+X save/quit\n\
             + alter  v throw  V spike  Shift+C close\n\
             Shift+I exam Shift+D dest Shift+N inscr\n\
             Shift+K know Shift+M browse Shift+P msgs\n\
             Shift+O pets Shift+R cure Shift+S look\n\
             Shift+T tactic Shift+X cut Shift+B fount\n\
             Shift+G give",
            ps.name,
            crate::game::display_race_name(&gd, &ps),
            ps.class_name,
            ps.level,
            ps.exp,
            next,
            ps.hp,
            ps.max_hp,
            ps.mana,
            ps.max_mana,
            ps.sanity,
            ps.max_sanity,
            ps.stats[0],
            ps.stats[1],
            ps.stats[2],
            ps.stats[3],
            ps.stats[4],
            ps.stats[5],
            ps.ac,
            ps.gold,
            lite,
            speed_str,
            ps.food,
            food_word,
            status_line,
            god_str,
            form_str,
            depth_str,
            day_str,
            ps.turn,
            quest_line,
        ));
    }
    if let Ok(mut text) = msg_q.single_mut() {
        let all: Vec<String> = log.display_lines().collect();
        let lines = &all[all.len().saturating_sub(4)..];
        *text = Text::new(lines.join("\n"));
    }
    if let Ok(mut text) = confirm_q.single_mut() {
        *text = Text::new(if turn.confirm_quit {
            "Really quit? [y/n]"
        } else {
            ""
        });
    }
}

// --- Death screen ---

pub fn setup_dead(
    mut commands: Commands,
    tiles: Res<TileAssets>,
    gd: Res<GameData>,
    mut ps: ResMut<PlayerState>,
    mut next: ResMut<NextState<AppState>>,
    mut log: ResMut<crate::game::MessageLog>,
    resurrect: Option<Res<crate::game::Resurrect>>,
) {
    // Eru's ultimate grace raises the faithful (the roll happened when
    // the Playing state was left; possessions were kept).
    if resurrect.is_some() {
        commands.remove_resource::<crate::game::Resurrect>();
        ps.hp = ps.max_hp;
        ps.grace = -200000;
        log.add("Eru Iluvatar raises you from the dead!");
        log.add("Your grace is utterly spent.");
        next.set(AppState::Playing);
        return;
    }
    // The base class level title (files.cc:4072 cp_ptr->titles, one per
    // five levels; winners are "Magnificent").
    let p_title = if ps.level > 50 {
        "Magnificent".to_string()
    } else {
        gd.classes
            .iter()
            .find(|c| c.name == ps.base_class)
            .and_then(|c| c.titles.get(((ps.level.saturating_sub(1)) / 5) as usize))
            .cloned()
            .unwrap_or_else(|| ps.class_name.clone())
    };
    // Permadeath: the save file dies with the character.
    crate::save::delete();
    commands
        .spawn((
            DeadRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.0, 0.0)),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new(format!(
                    "R.I.P.\n\n{}\nthe\n{}\n{}\nLevel {} — died on dungeon level {}\n\n\
                     Press Enter to return to character creation.",
                    ps.name,
                    p_title,
                    ps.class_name,
                    ps.level,
                    ps.depth
                )),
                TextFont {
                    font: tiles.font.clone().into(),
                    font_size: 22.0.into(),
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.85, 0.85)),
                TextLayout {
                    justify: Justify::Center,
                    ..default()
                },
            ));
        });
}

pub fn dead_input(keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<AppState>>) {
    if keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::Escape)
        || keys.just_pressed(KeyCode::Space)
    {
        next.set(AppState::Birth);
    }
}

pub fn cleanup_dead(mut commands: Commands, q: Query<Entity, With<DeadRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}
