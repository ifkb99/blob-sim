use crate::genes::Genes;

#[derive(Clone, Debug)]
pub struct Neuron {
    pub weight: f32,
    pub cur_sum: f32,
}
impl Neuron {
    fn activate(&mut self) {
        self.weight = 1. / (1. + (-self.cur_sum).exp());
    }
}

const FOUR_BITS: u128 = 15u128;
const SXTEEN_BITS: u128 = 65535u128;
// const N_SYNAPS: usize = 7;
const N_INPUT: u8 = 3;
const N_OUTPUT: u8 = 4;

#[derive(Debug)]
struct Synapse {
    from: u8,
    to: u8,
    weight: f32,
}

// TODO: pack these into one vec and set slice idx for faster compute
#[derive(Debug)]
struct SynBundle {
    // input to output
    direct_synaps: Vec<Synapse>,
    // internal to output
    int_out_synaps: Vec<Synapse>,
    // input to internal
    to_int_synaps: Vec<Synapse>,
    // internal to self
    self_synaps: Vec<Synapse>,
    // internal to other internal
    int_synaps: Vec<Synapse>,
}

// TODO: make more internal nodes use more energy
#[derive(Debug)]
pub struct Network {
    pub inputs: Vec<Neuron>,
    internal: Vec<Neuron>,
    pub outputs: Vec<Neuron>,
    s_bundle: SynBundle,
}
impl Network {
    // gene contains num internal nodes, weights of inputs, and details about connections (where and weight)
    // one u128 is a gene
    // first 4 is num internal nodes, next 12 are unused for now
    // the rest are u16s detailing connection info
    // x0 = from_internal. x1,x2,x3 = from_idx. x4 = to_internal. x5,x6,x7 = to_idx
    // x8-x15 is the weight of the connection (-4.0..4.0)
    pub fn new(genes: Genes) -> Network {
        let gene = genes.gene;
        let n_internal = ((gene & (FOUR_BITS << 124)) >> 124) as usize; // 11110000...

        let gen_synaps = |gene: u128, offset: u8| -> (bool, bool, Synapse) {
            let off = (7 - offset) * 16;
            let mask = ((gene & (SXTEEN_BITS << off)) >> off) as u16;
            // get first bit
            let from_internal = mask >= 1u16 << 15;
            // get bits 2-4
            let from = ((mask & (7u16 << 14)) >> 14) as u8;
            // get 5th bit
            let to_internal = mask & (1u16 << 11) != 0u16;
            // get bits 6-8
            let to = ((mask & (7u16 << 9)) >> 9) as u8;

            // get last ten bits
            // normalized to [-4.0..4.0]
            // TODO: consider cubing for accuracy
            let weight = (mask & 255u16) as i8 as f32 / 32.;

            (from_internal, to_internal, Synapse { from, to, weight })
        };
        // input to output or internal to output
        let mut int_out_synaps = Vec::new();
        let mut direct_synaps = Vec::new();
        // input to internal
        let mut to_int_synaps = Vec::new();
        // internal to self
        let mut self_synaps = Vec::new();
        // internal to other internal
        let mut int_synaps = Vec::new();
        // remove synaps that are conected to neurons that don't exist
        for i in 1..8 {
            let (from_int, to_int, syn) = gen_synaps(gene, i);
            if to_int {
                if from_int {
                    if syn.from == syn.to {
                        if syn.from < n_internal as u8 {
                            self_synaps.push(syn);
                        }
                    } else {
                        if syn.from < n_internal as u8 && syn.to < n_internal as u8 {
                            int_synaps.push(syn);
                        }
                    }
                } else {
                    if syn.from < N_INPUT && syn.to < n_internal as u8 {
                        to_int_synaps.push(syn);
                    }
                }
            } else if from_int {
                if syn.from < n_internal as u8 && syn.to < N_OUTPUT {
                    int_out_synaps.push(syn)
                }
            } else {
                if syn.from < N_INPUT && syn.to < N_OUTPUT {
                    direct_synaps.push(syn);
                }
            }
        }

        let s_bundle = SynBundle {
            direct_synaps,
            to_int_synaps,
            int_out_synaps,
            self_synaps,
            int_synaps,
        };

        let inputs = Vec::from_iter(
            std::iter::repeat(Neuron {
                weight: 0.,
                cur_sum: 0.,
            })
            .take(N_INPUT as usize),
        );
        let internal = Vec::from_iter(
            std::iter::repeat(Neuron {
                weight: 0.5,
                cur_sum: 0.,
            })
            .take(n_internal),
        );
        let outputs = Vec::from_iter(
            std::iter::repeat(Neuron {
                weight: 0.,
                cur_sum: 0.,
            })
            .take(N_OUTPUT as usize),
        );

        // TODO: consider separating out synapses to internal neurons and self loops

        Network {
            inputs,
            internal,
            outputs,
            s_bundle,
        }
    }

    pub fn eval(&mut self) -> (f32, f32, bool, bool) {
        // go over all synapses
        // TODO: determine execution order, and if activate in between
        // need genes for internal neuron weights

        // activate all input neurons
        // for neuron in &mut self.inputs {
        //     neuron.activate();
        // }

        // first all inputs to internal neurons.
        for syn in &self.s_bundle.to_int_synaps {
            self.internal[syn.to as usize].cur_sum +=
                self.inputs[syn.from as usize].weight * syn.weight;
        }
        // then, self loops
        for syn in &self.s_bundle.self_synaps {
            self.internal[syn.from as usize].activate();
            self.internal[syn.to as usize].cur_sum +=
                self.internal[syn.from as usize].weight * syn.weight;
        }
        // then, to other internal neurons
        for syn in &self.s_bundle.int_synaps {
            self.internal[syn.from as usize].activate();
            self.internal[syn.to as usize].cur_sum +=
                self.internal[syn.from as usize].weight * syn.weight;
        }
        // then, internal to output
        for syn in &self.s_bundle.int_out_synaps {
            self.internal[syn.from as usize].activate();
            self.outputs[syn.to as usize].cur_sum +=
                self.internal[syn.from as usize].weight * syn.weight;
        }
        // then, direct input to output
        for syn in &self.s_bundle.direct_synaps {
            self.outputs[syn.to as usize].cur_sum +=
                self.inputs[syn.from as usize].weight * syn.weight;
        }

        // finally, compute outputs
        for out in &mut self.outputs {
            out.activate();
        }

        for neuron in &mut self.inputs {
            neuron.cur_sum = 0.;
        }
        for neuron in &mut self.internal {
            neuron.cur_sum = 0.;
        }
        for neuron in &mut self.outputs {
            neuron.cur_sum = 0.;
        }

        // TODO: consider random vs breakpoint
        // why am I doing this extra stuff?
        (
            2. * self.outputs[0].weight - 1.,
            2. * self.outputs[1].weight - 1.,
            2. * self.outputs[2].weight - 1. > 0.7,
            2. * self.outputs[3].weight - 1. > 0.7,
        )
    }

    // fn set_inputs(&mut self, inputs: Vec<f32>) {
    //     for i in 0..self.inputs.len() {
    //         self.inputs[i].weight = inputs[i];
    //     }
    // }
}

#[cfg(test)]
mod tests {
    use crate::genes::Genes;
    use crate::network::Network;

    const EPSILON: f32 = 0.0001;

    #[test]
    fn convert_gene() {
        // 1110000000000000 0_011_1_001_01101011 1_001_0_010_10000000 0000....
        let mut test_net = Network::new(Genes {
            gene: 297748235675921506640778121573503598592u128,
        });

        // assert_eq!(test_net.eval(), (0.5, 0.62245935, false, false));
        assert_eq!(test_net.eval(), (0.0, -0.7615942, false, false));

        assert_eq!(test_net.s_bundle.int_out_synaps.len(), 1);
        assert_eq!(test_net.internal.capacity(), 14);
        assert!((test_net.s_bundle.to_int_synaps[0].weight).abs() - 3.34375 <= EPSILON);
        assert!((test_net.s_bundle.int_out_synaps[0].weight).abs() - 4.0 <= EPSILON);
    }
}
