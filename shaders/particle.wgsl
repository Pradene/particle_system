struct Particle {
    position: vec3<f32>,
    velocity: vec3<f32>,
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;

fn hash(x: u32) -> u32 {
    var state = x;
    state = state * 747796405u + 2891336453u;
    state = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    state = (state >> 22u) ^ state;
    return state;
}

fn hash3(x: u32, y: u32, z: u32) -> u32 {
    var state = x * 1597334673u;
    state = state ^ (y * 3812015801u);
    state = state ^ (z * 2654435761u);
    return hash(state);
}

fn random_float(state: ptr<function, u32>) -> f32 {
    *state = hash(*state);
    let value = (*state >> 8u) & 0xFFFFFFu;
    return f32(value) / 16777216.0;
}

@compute @workgroup_size(64)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    let index = global_id.x;
    if (index >= arrayLength(&particles)) {
        return;
    }
    
    var seed_x = hash3(index, 0u, 0u);
    var seed_y = hash3(index, 1u, 0u);
    var seed_z = hash3(index, 2u, 0u);
    
    let x = random_float(&seed_x) * 2.0 - 1.0;
    let y = random_float(&seed_y) * 2.0 - 1.0;
    let z = random_float(&seed_z);
    
    particles[index].position = vec3<f32>(x, y, z);
    particles[index].velocity = vec3<f32>(0.0, 0.0, 0.0);
}