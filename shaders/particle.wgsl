struct Particle {
    position: vec3<f32>,
    velocity: vec3<f32>,
    mass: f32,
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;

fn hash(x: u32) -> u32 {
    var state = x;
    state = state * 747796405u + 2891336453u;
    state = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    state = (state >> 22u) ^ state;
    return state;
}

fn random_float(state: ptr<function, u32>) -> f32 {
    *state = hash(*state);
    return f32(*state) / f32(0xFFFFFFFFu);
}

fn random_on_sphere(state: ptr<function, u32>) -> vec3<f32> {
    let u = random_float(state);
    let v = random_float(state);
    let scale = random_float(state) + 0.5;

    let theta = u * 2.0 * 3.14159265359;
    let phi = acos(2.0 * v - 1.0);

    let x = sin(phi) * cos(theta);
    let y = sin(phi) * sin(theta);
    let z = cos(phi);

    return vec3<f32>(x, y, z) * scale;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= arrayLength(&particles)) {
        return;
    }

    var seed = hash(index * 123456789u);
    particles[index].position = random_on_sphere(&seed);
    particles[index].velocity = vec3<f32>(0.0, 0.0, 0.0);
    particles[index].mass = 0.1;
}
