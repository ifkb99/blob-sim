use bevy::{
    core::FixedTimestep,
    math::{Vec2, Vec3},
    prelude::{
        App, Color, Commands, Component, Entity, Plugin, Query, Res, ResMut, SystemSet, Transform,
    },
    sprite::{Sprite, SpriteBundle},
    utils::HashSet,
};
use rand::Rng;

use crate::{Acceleration, Chem, Stages, Velocity, WinSize};

#[derive(Component)]
pub struct Food {
    pub nutriton: f32,
    chem_id: u8,
    emit_life: f64,
}
impl Default for Food {
    fn default() -> Self {
        Food {
            nutriton: 33.33,
            chem_id: 1u8,
            emit_life: 0.1,
        }
    }
}

struct MinFood(u32);
impl Default for MinFood {
    fn default() -> Self {
        Self(128)
    }
}
struct CurFood(u32);
impl Default for CurFood {
    fn default() -> Self {
        Self(0)
    }
}

pub struct EatenFood(pub HashSet<Entity>);
impl Default for EatenFood {
    fn default() -> Self {
        Self(HashSet::new())
    }
}

pub struct FoodPlugin;
impl Plugin for FoodPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CurFood::default())
            .insert_resource(MinFood::default())
            .insert_resource(EatenFood::default())
            .add_system_set_to_stage(
                Stages::FoodStage,
                SystemSet::new()
                    .with_run_criteria(FixedTimestep::step(1.0))
                    .with_system(spawn_food),
            )
            .add_system_to_stage(Stages::FoodStage, remove_food)
            .add_system(emit_chems)
            .add_system(dissolve_chems);
    }
}

// Runs once per second, spawns food if there is less than needed
fn spawn_food(
    mut commands: Commands,
    mut cur_food: ResMut<CurFood>,
    min_food: Res<MinFood>,
    win: Res<WinSize>,
) {
    let mut r = rand::thread_rng();
    let mut ctr = 0u8; // only bring eight back in one go
    while cur_food.0 < min_food.0 && ctr < 8 {
        commands
            .spawn_bundle(SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(0., 1., 0.),
                    custom_size: Some(Vec2::new(3., 3.)),
                    ..Default::default()
                },
                transform: Transform {
                    translation: Vec3::new(
                        r.gen_range(-win.w..win.w),
                        r.gen_range(-win.h..win.h),
                        1.,
                    ),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(Food::default())
            .insert(Velocity::default())
            .insert(Acceleration::default());
        cur_food.0 += 1;
        ctr += 1;
    }
}

// Emits chemicals that blobs can perceive
fn emit_chems(mut commands: Commands, query: Query<(&Transform, &Food)>) {
    let mut r = rand::thread_rng();
    query.for_each(|(trans, food)| {
        if r.gen_bool(food.emit_life) {
            commands
                .spawn_bundle(SpriteBundle {
                    sprite: Sprite {
                        color: Color::rgba_u8(food.chem_id, food.chem_id, food.chem_id, 123u8),
                        custom_size: Some(Vec2::new(1., 1.)),
                        ..Default::default()
                    },
                    transform: Transform {
                        translation: trans.translation.clone(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .insert(Chem {
                    // id: food.chem_id,
                    dissolve_life: 256u16,
                })
                .insert(Velocity::default())
                .insert(Acceleration::default());
        }
    });
}

fn dissolve_chems(mut commands: Commands, mut query: Query<(Entity, &mut Chem)>) {
    query.for_each_mut(|(ent, mut chem)| {
        if chem.dissolve_life == 0u16 {
            commands.entity(ent).despawn();
        } else {
            chem.dissolve_life -= 1u16;
        }
    });
}

// TODO error if two blobs eat same food at once, and ent is sent across two separate cycles
fn remove_food(
    mut commands: Commands,
    mut cur_food: ResMut<CurFood>,
    mut eaten_food: ResMut<EatenFood>,
) {
    for food in eaten_food.0.iter() {
        // println!("{:?}", food);
        commands.entity(*food).despawn();
        cur_food.0 -= 1;
    }
    eaten_food.0.clear();
}
