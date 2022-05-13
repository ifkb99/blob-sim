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
        if r.gen_bool(MUT_RATE) {
            gene ^= 1u128 << r.gen_range(0..128);
        }
        Genes { gene }
    }
}
