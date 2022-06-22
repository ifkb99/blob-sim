use bevy::{
    math::Vec2,
    prelude::{
        App, Commands, Component, OrthographicCameraBundle, Query, Res, StageLabel, SystemStage,
        Transform,
    },
    tasks::ComputeTaskPool,
    window::{WindowDescriptor, Windows},
    DefaultPlugins,
};

mod food;
use food::FoodPlugin;

mod blob;
use blob::BlobPlugin;

mod genes;
mod network;

use rand::Rng;
use rand_distr::StandardNormal;

// const HEIGHT: f32 = 480.;
const HEIGHT: f32 = 720.;
// const WIDTH: f32 = 640.;
const WIDTH: f32 = 1280.;

const N_BLOBS: u32 = 32;
const FD_TO_BLOB: f32 = 1.5;

struct Speed(f32);
impl Default for Speed {
    fn default() -> Self {
        Self(0.3)
    }
}

#[derive(Component)]
struct Chem {
    // is also colour in shader
    // id: u8,
    dissolve_life: u16,
}

// make a grid that will be drawn on by a shader

struct WinSize {
    w: f32,
    h: f32,
}

#[derive(Component)]
struct Velocity(Vec2);
impl Default for Velocity {
    fn default() -> Self {
        Self(Vec2::ZERO)
    }
}

#[derive(Component)]
struct Acceleration(Vec2);
impl Default for Acceleration {
    fn default() -> Self {
        Self(Vec2::ZERO)
    }
}

// TODO:
// - make rng simwide, instead of creating new one when needed
// - make entire 'screen' follow fluid dynamics to move everything
// - turn spawn command into a closure, most of code is repeated. Maybe a macro

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum Stages {
    FoodStage,
    BlobStage,
    MoveStage,
}

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            title: "Blobs".to_string(),
            width: WIDTH,
            height: HEIGHT,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_stage(Stages::BlobStage, SystemStage::parallel())
        .add_stage_after(
            Stages::BlobStage,
            Stages::FoodStage,
            SystemStage::parallel(),
        )
        .add_plugin(FoodPlugin)
        .add_plugin(BlobPlugin)
        .add_startup_system(setup)
        // .add_system_before(brownian_drift)
        .add_stage_before(
            Stages::BlobStage,
            Stages::MoveStage,
            SystemStage::parallel(),
        )
        .add_system_to_stage(Stages::MoveStage, brownian_drift)
        .run();
}

fn setup(mut commands: Commands, win: Res<Windows>) {
    let w = win.get_primary().unwrap();

    // set origin to top left
    let mut camera = OrthographicCameraBundle::new_2d();
    camera.transform.translation.x += w.width() / 2.;
    camera.transform.translation.y += w.height() / 2.;
    commands.spawn_bundle(camera);
    commands.insert_resource(WinSize {
        w: w.width() - 5.,
        h: w.height() - 5.,
    });
}

// Moves all entities according to Brownian motion to simulate the movement of water
// Also takes other movement into account
const BROWN_SCALE: f32 = 8.;

fn brownian_drift(
    win: Res<WinSize>,
    mut query: Query<(&mut Acceleration, &mut Velocity, &mut Transform)>,
    pool: Res<ComputeTaskPool>,
) {
    query.par_for_each_mut(&pool, 128, |(mut accel, mut vel, mut trans)| {
        // Brownian
        let mut r = rand::thread_rng();
        // maybe try perlin noise?
        accel.0.x += r.sample::<f32, _>(StandardNormal) / BROWN_SCALE;
        accel.0.y += r.sample::<f32, _>(StandardNormal) / BROWN_SCALE;

        vel.0.x += accel.0.x;
        vel.0.y += accel.0.y;

        trans.translation.x = ((trans.translation.x + vel.0.x) % win.w + win.w) % win.w;
        trans.translation.y = ((trans.translation.y + vel.0.y) % win.h + win.h) % win.h;

        // drag
        vel.0.x *= 0.8;
        vel.0.y *= 0.8;

        accel.0.x = 0.;
        accel.0.y = 0.;
    });
}
