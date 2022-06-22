use bevy::{
    core::FixedTimestep,
    math::{Vec2, Vec3},
    prelude::{
        info_span, App, Color, Commands, Component, Entity, Plugin, Query, Res, ResMut, SystemSet,
        Transform, With,
    },
    sprite::{Sprite, SpriteBundle},
    tasks::ComputeTaskPool,
};
use rand::Rng;
use std::cmp::Ordering::Equal;

use crate::{
    food::{EatenFood, Food},
    genes::Genes,
    network::Network,
    Chem, Stages, WinSize,
};
use crate::{Acceleration, Velocity, N_BLOBS};

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
        Self(N_BLOBS)
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

// MOVE THIS TO OWN FILE??
// TODO: make a test for this
fn bin_search(
    l: usize,
    r: usize,
    c: &[Vec3],
    blob_x: f32,
    limit: f32,
    accept: fn(cur: usize, c: &[Vec3], blob_x: f32, limit: f32) -> i8,
) -> (usize, bool) {
    if r >= l {
        let mid = l + (r - l) / 2;

        let dir = accept(mid, c, blob_x, limit);
        if dir == 0 {
            // found bound
            return (mid, true);
        }
        if dir == 1 {
            if mid == c.len() - 1 {
                return (mid, false);
            }
            // bound is right
            return bin_search( mid + 1, r, c, blob_x, limit, accept);
        }
        if mid == 0 {
            return (mid, false);
        }
        // bound is left
        return bin_search(l, mid - 1, c, blob_x, limit, accept);
    }
    (0, false)
}

fn find_window(blob_x: f32, limit: f32, c: &[Vec3]) -> (usize, usize, bool) {
    let c_len = c.len();
    // I use dist_squared when looking at pos
    let limit = limit * limit;
    if c_len < 2 {
        // if there is only one chem on screen I literally don't care
        return (0, 0, false);
    }

    let (r, exists) = bin_search(
        0,
        c_len - 1,
        c,
        blob_x,
        limit,
        // I think the issue is here
        |cur: usize, c: &[Vec3], blob_x: f32, limit: f32| -> i8 {
            // want to find spot where cur is within limit, and just right of cur is over limit (or cur is len of arr-1)
            if (blob_x - c[cur].x).abs() <= limit {
                if cur == c.len() - 1 || (blob_x - c[cur + 1].x).abs() > limit {
                    0
                } else {
                // go right
                    1
                }
            } else if blob_x > c[cur].x {
                1
            } else {
                -1
            }
        },
    );
    if !exists {
        return (0, 0, false);
    }

    let (l, _) = bin_search(
        0,
        r,
        c,
        blob_x,
        limit,
        // I think the issue is here
        |cur: usize, c: &[Vec3], blob_x: f32, limit: f32| -> i8 {
            // want to find spot where cur is within limit, and just left of cur is over limit (or cur is 0)
            if (blob_x - c[cur].x).abs() <= limit {
                // within limit
                if cur == 0 || (blob_x - c[cur - 1].x).abs() > limit {
                    // found bound
                    0
                } else {
                    // go left
                    -1
                }
            // outside limit
            } else if blob_x > c[cur].x {
                1
            } else {
                -1
            }
        },
    );
    

    // if left exists, right does too
    // if !exists {
    //     return (0, 0, false);
    // }

    (l, r, true)
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
    // sweep and prune for faster distance check
    let sort_span = info_span!("sorting_chems", name = "sorting_chems").entered();
    let mut c: Vec<Vec3> = chem_query
        .iter()
        .map(|(trans, _)| trans.translation)
        .collect();
    c.sort_unstable_by(|t1, t2| t1.x.partial_cmp(&t2.x).unwrap_or(Equal));
    sort_span.exit();

    blob_query.par_for_each_mut(&pool, 16, |(mut accel, blob_trans, mut blob)| {
        // update sensors, x and y dir for nearby chems
        // I'm lazy so I will simply perform a binary search to create a window of chems that need to be considered
        // perhaps I can turn this (chem list) into some sort of tree later
        let blob_loc = blob_trans.translation;
        let window_span = info_span!("find_window", name = "find_window").entered();
        let (l, r, window_exists) = find_window(blob_loc.x, 100., &c);
        if window_exists {
            // println!("l: {} r: {}, c_len: {}", l, r, c.len() - 1);
            for chem_idx in l..r {
                let dist = blob_loc.distance_squared(c[chem_idx]);
                if dist < 100. {
                    blob.brain.inputs[0].cur_sum += blob_loc.x - c[chem_idx].x;
                    blob.brain.inputs[1].cur_sum += blob_loc.y - c[chem_idx].y;
                }
            }
        }
        window_span.exit();

        // let naive_span = info_span!("n2rd", name = "n2rd").entered();
        // c.iter().for_each(|trans| {
        //     let loc = trans;
        //     let dist = blob_loc.distance_squared(*loc);

        //     if dist < 100. {
        //         blob.brain.inputs[0].cur_sum += blob_loc.x - loc.x;
        //         blob.brain.inputs[1].cur_sum += blob_loc.y - loc.y;
        //     }
        // });
        // naive_span.exit();
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

#[cfg(test)]
mod tests {
    use bevy::math::Vec3;
    use crate::blob::find_window;

    #[test]
    fn find_window_test() {
        let blob_x = 2.0;
        let chems = vec!(
            Vec3::new(0., 1., 1.), Vec3::new(1., 1., 1.),
            Vec3::new(2., 1., 1.),
            Vec3::new(3., 3., 3.), Vec3::new(4., 4., 4.)
        );
        let (l, r, _) = find_window(blob_x, 1., &chems);
        
        assert_eq!(l, 1);
        assert_eq!(r, 3);
    }
}