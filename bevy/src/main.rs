mod base_defs;
mod autopilot;
mod birth;
mod colors;
mod corrupt;
mod data;
mod files;
mod game;
mod hud;
mod input;
mod item;
mod map;
mod mimic;
mod modal;
mod notes;
mod options;
mod recall;
mod render;
mod rng;
mod save;
mod scores;
mod skill;
mod speech;
mod spell;
mod squeltch;
mod town;
mod util;
mod zutil;

use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    Birth,
    Playing,
    Dead,
}

fn main() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            // Keep the 16x16 tiles crisp.
            .set(ImagePlugin::default_nearest())
            // Resolve assets relative to the crate root so the game works
            // no matter where the binary is invoked from.
            .set(AssetPlugin {
                file_path: concat!(env!("CARGO_MANIFEST_DIR"), "/assets").to_string(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: format!("{} - Bevy", zutil::get_version_string()),
                    resolution: (1600, 720).into(),
                    resizable: false,
                    ..default()
                }),
                ..default()
            }),
    )
    .init_state::<AppState>()
    .insert_resource(data::load_game_data())
    .init_resource::<game::MessageLog>()
    .init_resource::<game::TurnState>()
    .init_resource::<game::PlotQuest>()
    .init_resource::<item::Inventory>()
    .init_resource::<item::CreatedArtifacts>()
    .init_resource::<modal::Modal>()
    .init_resource::<modal::ModalInputConsumed>()
    .init_resource::<modal::TargetLock>()
    .init_resource::<input::MoveRepeat>()
    .init_resource::<input::PetOptions>()
    .init_resource::<input::PetMenu>()
    .init_resource::<input::ItemPicker>()
    .init_resource::<town::ShopStocks>()
    .init_resource::<save::LevelStore>()
    .init_resource::<scores::HighScores>()
    .init_resource::<options::Options>()
    .init_resource::<notes::Notes>()
    .init_resource::<squeltch::Automatizer>()
    .add_systems(Startup, (render::load_tile_assets, render::setup_cameras))
    .add_systems(Update, render::update_camera_viewport)
    // Enter a newly loaded game: load the automatizer rules for this
    // character (dungeon.cc automatizer_load) and refresh the level-up /
    // session-note tracker before setup_level consumes PendingLoad.
    .add_systems(
        OnEnter(AppState::Playing),
        (squeltch::load_rules_on_enter, notes::track_level_on_enter)
            .chain()
            .before(game::setup_level),
    )
    // Character creation
    .add_systems(
        Update,
        (
            birth::ensure_birth_ui,
            birth::birth_input,
            birth::update_birth_ui,
        )
            .chain()
            .run_if(in_state(AppState::Birth)),
    )
    .add_systems(OnExit(AppState::Birth), birth::cleanup_birth)
    // Playing
    .add_systems(
        OnEnter(AppState::Playing),
        (game::setup_level, hud::setup_hud).chain(),
    )
    .add_systems(
        Update,
        (
            modal::open_pending_skill_modal,
            modal::modal_input,
            input::player_input,
            game::rest_driver,
            game::run_driver,
            game::monster_turns,
            game::god_sync,
            game::level_transition,
            render::sync_tiles,
            render::sync_actors,
            render::sync_items,
            render::camera_follow,
            hud::sync_hud,
        )
            .chain()
            .run_if(in_state(AppState::Playing)),
    )
    .add_systems(
        Update,
        (modal::ensure_modal_ui, modal::sync_modal)
            .chain()
            .run_if(in_state(AppState::Playing)),
    )
    .add_systems(OnExit(AppState::Playing), game::cleanup_level)
    // Death
    .add_systems(OnEnter(AppState::Dead), hud::setup_dead)
    .add_systems(Update, hud::dead_input.run_if(in_state(AppState::Dead)))
    .add_systems(OnExit(AppState::Dead), hud::cleanup_dead);

    autopilot::maybe_insert(&mut app);
    app.add_systems(
        Update,
        autopilot::autopilot_birth
            .run_if(in_state(AppState::Birth))
            .run_if(resource_exists::<autopilot::Autopilot>),
    );
    app.add_systems(
        Update,
        autopilot::autopilot_play
            .before(modal::modal_input)
            .before(input::player_input)
            .run_if(in_state(AppState::Playing))
            .run_if(resource_exists::<autopilot::Autopilot>),
    );
    app.add_systems(
        Update,
        autopilot::autopilot_dead
            .run_if(in_state(AppState::Dead))
            .run_if(resource_exists::<autopilot::Autopilot>),
    );
    app.run();
}
