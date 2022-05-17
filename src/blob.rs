use bevy::{
    core::FixedTimestep,
    math::{Vec2, Vec3},
    prelude::{
        App, Color, Commands, Component, Entity, Plugin, Query, Res, ResMut, SystemSet, Transform,
        With,
    },
    sprite::{Sprite, SpriteBundle},
    tasks::ComputeTaskPool,
};
use rand::Rng;

use crate::{
    food::{EatenFood, Food},
    genes::Genes,
    network::Network,
    Chem, Stages, WinSize,
};
use crate::{Acceleration, Velocity};

#[derive(Component)]
struct Blob {
    energy: f32,
    age: f32,
    generation: u16,
    brain: Network,
}

struct OldestBlob((u128, u16));
impl Default for OldestBlob {
    fn default() -> Self {
        Self((0u128, 0u16))
    }
}

struct CurBlobs(u32);
impl Default for CurBlobs {
    fn default() -> Self {
        Self(0)
    }
}

struct MinBlobs(u32);
impl Default for MinBlobs {
    fn default() -> Self {
        Self(32)
    }
}

#[derive(Component)]
struct EatenChems(Vec<(Entity, Vec3)>);
impl Default for EatenChems {
    fn default() -> Self {
        Self(Vec::new())
    }
}

pub struct BlobPlugin;
impl Plugin for BlobPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CurBlobs::default())
            .insert_resource(MinBlobs::default())
            .insert_resource(OldestBlob::default())
            .add_system_set(
                SystemSet::new()
                    .with_run_criteria(FixedTimestep::step(1.0))
                    .with_system(spawn_blobs)
                    .with_system(blob_replicate)
                    .with_system(get_oldest),
            )
            .add_system_to_stage(Stages::BlobStage, blob_action)
            .add_system(kill_blobs);
    }
}

// TODO: add sensors for chems and other things (hitboxes)
// add ability to evolve more?

// Runs once per second, spawns blobs if there is less than needed
fn spawn_blobs(
    mut commands: Commands,
    mut cur_blobs: ResMut<CurBlobs>,
    min_blobs: Res<MinBlobs>,
    win: Res<WinSize>,
) {
    let mut r = rand::thread_rng();
    while cur_blobs.0 < min_blobs.0 {
        spawn_blob(
            &mut commands,
            Vec3::new(r.gen_range(0.0..win.w), r.gen_range(0.0..win.h), 0.9),
            Genes::default(),
            100.,
            &mut cur_blobs,
            0,
        );
    }
}

fn spawn_blob(
    commands: &mut Commands,
    trans: Vec3,
    gene: Genes,
    energy: f32,
    cur_blobs: &mut ResMut<CurBlobs>,
    generation: u16,
) {
    let gen = gene.gene;
    let r = ((gen & (255u128 << 120)) >> 120) as u8;
    let g = ((gen & (255u128 << 112)) >> 112) as u8;
    let b = ((gen & (255u128 << 106)) >> 106) as u8;
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb_u8(r, g, b),
                custom_size: Some(Vec2::new(5., 5.)),
                ..Default::default()
            },
            transform: Transform {
                translation: trans.clone(),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Blob {
            energy,
            brain: Network::new(gene.clone()),
            age: 0.,
            generation,
        })
        .insert(Velocity::default())
        .insert(Acceleration::default())
        .insert(gene);
    cur_blobs.0 += 1;
}

fn kill_blobs(
    mut commands: Commands,
    mut query: Query<(Entity, &Blob)>,
    mut cur_blobs: ResMut<CurBlobs>,
) {
    query.iter_mut().for_each(|(ent, blob)| {
        if blob.energy < 0. {
            commands.entity(ent).despawn();
            cur_blobs.0 -= 1;
        }
    });
}

fn blob_replicate(
    mut commands: Commands,
    mut query: Query<(&Transform, &Genes, &mut Blob)>,
    mut cur_blobs: ResMut<CurBlobs>,
) {
    let mut r = rand::thread_rng();
    query.for_each_mut(|(trans, gene, mut blob)| {
        // reproduce, but not too often
        // TODO: this should be done inside net? or just outside?
        if blob.brain.outputs[3].weight > 0.3
            && r.gen_bool((blob.brain.outputs[3].weight - 0.3) as f64)
        {
            // STOP SPAWNING SO MUCH AAAAAAA
            if blob.energy <= 10. {
                blob.energy = 0.
            } else {
                spawn_blob(
                    &mut commands,
                    trans.translation,
                    gene.replicate(),
                    blob.energy / 2.,
                    &mut cur_blobs,
                    blob.generation + 1,
                );
                blob.energy /= 2.;
            }
        }
    });
}

// Input nodes: sensor(s), oscillator, energy
// n mid nodes
// Output nodes: x_mov, y_mov, consume, reproduce

// THIS IS TEMPORARYYYYY
// works kinda snice tho
fn blob_action(
    mut blob_query: Query<(&mut Acceleration, &Transform, &mut Blob)>,
    chem_query: Query<(&Transform, With<Chem>)>,
    food_query: Query<(Entity, &Transform, &Food)>,
    mut eaten_food: ResMut<EatenFood>,
    pool: Res<ComputeTaskPool>,
) {
    let c: Vec<(&Transform, _)> = chem_query.iter().collect();
    blob_query.par_for_each_mut(&pool, 16, |(mut accel, blob_trans, mut blob)| {
        // update sensors, x and y dir for nearby chems
        let blob_loc = blob_trans.translation;
        c.iter().for_each(|(trans, _)| {
            let loc = trans.translation;
            let dist = blob_loc.distance_squared(loc);

            if dist < 100. {
                blob.brain.inputs[0].cur_sum += blob_loc.x - loc.x;
                blob.brain.inputs[1].cur_sum += blob_loc.y - loc.y;
            }
        });
        blob.brain.inputs[0].activate();
        blob.brain.inputs[1].activate();
        blob.brain.inputs[0].cur_sum = 0.;
        blob.brain.inputs[1].cur_sum = 0.;
        // energy level is second input
        blob.brain.inputs[2].cur_sum = blob.energy;
        blob.brain.inputs[2].activate();
        blob.brain.inputs[2].cur_sum = 0.;

        // oscillator
        // activate or no?
        blob.brain.inputs[3].weight = 0.5 + (blob.age * 10.).sin() / 2.;
        // blob.brain.inputs[3].cur_sum = 0.5 + (blob.age * 10.).sin() / 2.;
        // blob.brain.inputs[3].activate();
        // blob.brain.inputs[3].cur_sum = 0.;

        // this is bad
        // get all things within

        // TODO: move results into blob and pull from there
        let actions = blob.brain.eval();

        accel.0.x += actions.0;
        accel.0.y += actions.1;

        // movement costs energy, scaling quadradically
        let mov = actions.0.abs() + actions.1.abs();
        blob.energy -= mov * mov / 5.;

        // if actions.2 {
        //     // consume
        //     // TODO
        //     blob.energy -= 0.003;
        // }

        // if actions.3 {
        //     // try reproduce
        //     // TODO
        // }

        // die slowly....
        blob.energy -= 0.001;
        blob.age += 0.001;
    });

    blob_query.for_each_mut(|(_, trans, mut blob)| {
        let blob_loc = trans.translation;
        food_query.for_each(|(ent, trans, food)| {
            let dist = blob_loc.distance_squared(trans.translation);
            if dist < 10. {
                eaten_food.0.insert(ent);
                blob.energy += food.nutriton;
            }
        });
    });
}

// Success collection:
// - 27412239664388069923010120978984735311
fn get_oldest(mut oldest: ResMut<OldestBlob>, query: Query<(&Blob, &Genes)>) {
    query.for_each(|(blob, genes)| {
        if blob.generation > oldest.0 .1 {
            println!(
                "New hightest gen blob! Gen: {} Geneome: {}",
                blob.generation, genes.gene
            );
            oldest.0 = (genes.gene, blob.generation);
        }
    });
}
