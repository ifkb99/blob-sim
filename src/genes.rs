use bevy::prelude::Component;
use rand::Rng;

const MUT_RATE: f64 = 0.001;

// TODO: turn this into a vector or array to support more genes
// also remember to turn i8 weights into i16s
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
        let mut mutate = |start: i32, end: i32| {
            if r.gen_bool(MUT_RATE) {
                gene ^= 1u128 << r.gen_range(start..end);
            }
        };
        for i in 0..8 {
            mutate(i * 16, (i + 1) * 16);
        }

        Genes { gene }
    }
}
