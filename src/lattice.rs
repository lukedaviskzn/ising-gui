use rand::Rng;

use crate::spin::Spin;

/// Boltzman probability for given energy/delta energy and temperature.
fn boltzman(energy: f32, temperature: f32) -> f32 {
    f32::exp(-energy / temperature)
}

#[derive(Debug, Clone)]
struct InterationsStorage {
    up: f32,
    left: f32,
    // down/right interations stored in up/left interactions of neighbours
    // down: f32,
    // right: f32,
}

impl InterationsStorage {
    const FERROMAGNETIC: InterationsStorage = InterationsStorage {
        up: 1.0,
        left: 1.0,
    };
    
    const ANTIFERROMAGNETIC: InterationsStorage = InterationsStorage {
        up: -1.0,
        left: -1.0,
    };
    
    pub fn spin_glass(antiferromagnetic_probability: f64) -> InterationsStorage {
        InterationsStorage {
            up: (rand::thread_rng().gen_bool(1.0 - antiferromagnetic_probability) as i32 * 2 - 1) as f32,
            left: (rand::thread_rng().gen_bool(1.0 - antiferromagnetic_probability) as i32 * 2 - 1) as f32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LatticeInitialState {
    Random,
    AllUp,
    AllDown,
}

#[derive(Debug, Clone)]
struct Interactions {
    up: f32,
    left: f32,
    down: f32,
    right: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LatticeType {
    Ferromagnetic,
    Antiferromagnetic,
    SpinGlass { p_antiferro: f64 },
}

#[derive(Debug)]
pub struct Lattice {
    state: Vec<Spin>,
    interations: Vec<InterationsStorage>,
    size: usize,
    pub temperature: f32,
    // magnetic field B, z component
    pub magnetic_field: f32,
}

impl Lattice {
    /// Lattice with random initial state
    pub fn new_random(size: usize, temperature: f32, magnetic_field: f32, lattice_type: LatticeType) -> Lattice {
        let mut spins = Vec::with_capacity(size * size);

        for _ in 0..size*size {
            let spin = if rand::thread_rng().gen::<bool>() {
                Spin::Up
            } else {
                Spin::Down
            };

            spins.push(spin);
        }

        let interations = match lattice_type {
            LatticeType::Ferromagnetic => vec![InterationsStorage::FERROMAGNETIC;size*size],
            LatticeType::Antiferromagnetic => vec![InterationsStorage::ANTIFERROMAGNETIC;size*size],
            LatticeType::SpinGlass { p_antiferro } => {
                let mut ints = Vec::with_capacity(size*size);
                for _ in 0..size*size {
                    ints.push(InterationsStorage::spin_glass(p_antiferro));
                }
                ints
            },
        };

        Lattice {
            state: spins,
            interations,
            size,
            temperature,
            magnetic_field,
        }
    }

    /// Lattice with uniform initial state
    pub fn new_uniform(size: usize, temperature: f32, magnetic_field: f32, spin: Spin, lattice_type: LatticeType) -> Lattice {
        let mut spins = Vec::with_capacity(size * size);

        for _ in 0..size*size {
            spins.push(spin);
        }

        let interations = match lattice_type {
            LatticeType::Ferromagnetic => vec![InterationsStorage::FERROMAGNETIC;size*size],
            LatticeType::Antiferromagnetic => vec![InterationsStorage::ANTIFERROMAGNETIC;size*size],
            LatticeType::SpinGlass { p_antiferro } => {
                let mut ints = Vec::with_capacity(size*size);
                for _ in 0..size*size {
                    ints.push(InterationsStorage::spin_glass(p_antiferro));
                }
                ints
            },
        };

        Lattice {
            state: spins,
            interations,
            size,
            temperature,
            magnetic_field,
        }
    }

    pub fn internal_energy(&self) -> f32 {
        let mut energy = 0.0;

        let s = self.size as isize;
        
        for y in 0..s {
            for x in 0..s {
                energy += self.hamiltonian(x, y);
            }
        }
        
        energy
    }

    pub fn heat_capacity(&self) -> f32 {
        let mut energy = 0.0;

        let s = self.size as isize;
        
        for y in 0..s {
            for x in 0..s {
                let h = self.hamiltonian(x, y);
                energy += h*h;
            }
        }
        
        (energy - self.internal_energy()) as f32 / self.state.len() as f32
    }

    pub fn magnetisation(&self) -> f32 {
        self.state.iter().map(|s| Into::<i32>::into(*s)).sum::<i32>() as f32 / self.state.len() as f32
    }

    pub fn hamiltonian(&self, x: isize, y: isize) -> f32 {
        let mut energy = 0.0;
        
        // Spin interaction component
        let spin = self.get(x, y);

        let interactions = self.get_interactions(x, y);
        
        energy += -interactions.left  * (spin * self.get(x-1, y)) as f32;
        energy += -interactions.up    * (spin * self.get(x, y-1)) as f32;
        energy += -interactions.right * (spin * self.get(x+1, y)) as f32;
        energy += -interactions.down  * (spin * self.get(x, y+1)) as f32;

        // Magnetic component
        energy -= Into::<i32>::into(spin) as f32 * self.magnetic_field;

        energy
    }

    pub fn step(&mut self) {
        let s = self.size as isize;
        let x = rand::thread_rng().gen_range(0..s);
        let y = rand::thread_rng().gen_range(0..s);

        let mut d_energy = -self.hamiltonian(x, y);

        d_energy -= self.hamiltonian(x-1, y);
        d_energy -= self.hamiltonian(x, y-1);
        d_energy -= self.hamiltonian(x+1, y);
        d_energy -= self.hamiltonian(x, y+1);

        self.flip(x, y);

        d_energy += self.hamiltonian(x, y);

        d_energy += self.hamiltonian(x-1, y);
        d_energy += self.hamiltonian(x, y-1);
        d_energy += self.hamiltonian(x+1, y);
        d_energy += self.hamiltonian(x, y+1);

        // internal energy increases with this change, accept with boltzman probability
        if d_energy > 0.0 && rand::thread_rng().gen_range(0.0..1.0) > boltzman(d_energy, self.temperature) {
            // failed dice roll, undo flip
            self.flip(x, y);
        }
    }

    pub fn epoch(&mut self) {
        for _ in 0..self.size*self.size {
            self.step();
        }
    }

    // I know it's horribly inefficient to generate the image this way, but it's too much work to do it properly.
    pub fn as_image(&self, available_space: usize) -> egui::ColorImage {
        let scale = available_space / self.size + 1;
        
        let mut rgb = Vec::with_capacity(self.size * self.size * scale * scale * 3);

        for y in 0..self.size {
            for _ in 0..scale {
                for x in 0..self.size {
                    match self.get(x as isize, y as isize) {
                        Spin::Up => {
                            // blue
                            for _ in 0..scale {
                                rgb.push(0);
                                rgb.push(0);
                                rgb.push(255);
                            }
                        },
                        Spin::Down => {
                            // red
                            for _ in 0..scale {
                                rgb.push(255);
                                rgb.push(0);
                                rgb.push(0);
                            }
                        },
                    }
                }
            }
        }

        egui::ColorImage::from_rgb([self.size * scale, self.size * scale], rgb.as_slice())
    }

    // I know it's horribly inefficient to generate the image this way, but it's too much work to do it properly.
    pub fn as_image_raw(&self) -> (Vec<u8>, usize) {
        let mut rgb = Vec::with_capacity(self.size * self.size * 3);

        for y in 0..self.size {
            for x in 0..self.size {
                match self.get(x as isize, y as isize) {
                    Spin::Up => {
                        // blue
                        rgb.push(0);
                        rgb.push(0);
                        rgb.push(255);
                    },
                    Spin::Down => {
                        // red
                        rgb.push(255);
                        rgb.push(0);
                        rgb.push(0);
                    },
                }
            }
        }

        (rgb, self.size)
    }

    fn index(&self, x: isize, y: isize) -> usize {
        let s = self.size as isize;
        (x.rem_euclid(s) + y.rem_euclid(s) * s) as usize
    }

    fn get(&self, x: isize, y: isize) -> Spin {
        self.state[self.index(x, y)]
    }

    fn get_interactions(&self, x: isize, y: isize) -> Interactions {
        let current = &self.interations[self.index(x, y)];
        let right = &self.interations[self.index(x+1, y)];
        let bottom = &self.interations[self.index(x, y+1)];
        
        Interactions {
            up: current.up,
            left: current.left,
            down: bottom.up,
            right: right.left,
        }
    }

    fn flip(&mut self, x: isize, y: isize) {
        let i = self.index(x, y);
        self.state[i] = -self.state[i];
    }
}
