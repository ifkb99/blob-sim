use bevy::prelude::Component;
use rand::Rng;

const MUT_RATE: f64 = 0.001;

type Gene = u128;

#[derive(Clone, Component)]
pub struct Genes {
    pub gene: Gene,
}
impl Default for Genes {
    fn default() -> Self {
        Genes {
            gene: rand::thread_rng().gen::<Gene>(),
        }
    }
}

impl Genes {
    pub fn replicate(&self) -> Genes {
        let mut r = rand::thread_rng();
        let mut gene = self.gene.clone();
        // TODO: hmmm closure maybe lol
        if r.gen_bool(MUT_RATE) {
            gene ^= 1u128 << r.gen_range(0..16);
        }
        if r.gen_bool(MUT_RATE) {
            gene ^= 1u128 << r.gen_range(16..32);
        }
        if r.gen_bool(MUT_RATE) {
            gene ^= 1u128 << r.gen_range(32..48);
        }
        if r.gen_bool(MUT_RATE) {
            gene ^= 1u128 << r.gen_range(48..64);
        }
        if r.gen_bool(MUT_RATE) {
            gene ^= 1u128 << r.gen_range(64..80);
        }
        if r.gen_bool(MUT_RATE) {
            gene ^= 1u128 << r.gen_range(80..96);
        }
        if r.gen_bool(MUT_RATE) {
            gene ^= 1u128 << r.gen_range(96..112);
        }
        if r.gen_bool(MUT_RATE) {
            gene ^= 1u128 << r.gen_range(112..128);
        }
        Genes { gene }
    }
}
