struct EmitUniforms {
    position: vec4<f32>,
    count: u32,
    shape: u32,
    lifetime: f32,
    elapsed_time: f32,
}

struct Particle {
    position: vec4<f32>,
    velocity: vec4<f32>,
    mass: f32,
    lifetime: f32,
}

@group(0) @binding(0) var<uniform> uniforms: EmitUniforms;
@group(0) @binding(1) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(2) var<storage, read_write> particle_count: atomic<u32>;

fn hash(x: u32) -> u32 {
    var s = x;
    s = (s ^ 61u) ^ (s >> 16u);
    s = s + (s << 3u);
    s = s ^ (s >> 4u);
    s = s * 0x27d4eb2du;
    s = s ^ (s >> 15u);
    return s;
}

fn random_float(state: ptr<function, u32>) -> f32 {
    *state = hash(*state);
    return f32(*state) / f32(0xFFFFFFFFu);
}

fn random_range(state: ptr<function, u32>, min_val: f32, max_val: f32) -> f32 {
    return min_val + random_float(state) * (max_val - min_val);
}

fn random_on_sphere(state: ptr<function, u32>) -> vec3<f32> {
    let u = random_float(state);
    let v = random_float(state);

    let theta = u * 2.0 * 3.14159265359;
    let phi = acos(2.0 * v - 1.0);

    let x = sin(phi) * cos(theta);
    let y = sin(phi) * sin(theta);
    let z = cos(phi);

    return vec3<f32>(x, y, z);
}

fn random_on_cube(state: ptr<function, u32>) -> vec3<f32> {
    let face = u32(random_float(state) * 6.0);
    let u = random_float(state) * 2.0 - 1.0;
    let v = random_float(state) * 2.0 - 1.0;

    switch (face) {
        case 0u: { return vec3<f32>( 1.0,    u,    v); } // +X
        case 1u: { return vec3<f32>(-1.0,    u,    v); } // -X
        case 2u: { return vec3<f32>(   u,  1.0,    v); } // +Y
        case 3u: { return vec3<f32>(   u, -1.0,    v); } // -Y
        case 4u: { return vec3<f32>(   u,    v,  1.0); } // +Z
        case 5u: { return vec3<f32>(   u,    v, -1.0); } // -Z
        default: { return vec3<f32>( 0.0,  0.0,  0.0); }
    }
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= uniforms.count) {
        return;
    }

    let write_index = atomicAdd(&particle_count, 1u);
    if (write_index >= arrayLength(&particles)) {
        return;
    }

    var seed = hash(hash(write_index) ^ (bitcast<u32>(uniforms.elapsed_time) * 7919u));

    let scale = 8.0;
    var vector = vec3(0.0, 0.0, 0.0);
    if (uniforms.shape == 0u) {
        vector = vec3(0.0, 0.0, 0.0);
    } else if (uniforms.shape == 1u) {
        vector = random_on_sphere(&seed) * scale;
    } else if (uniforms.shape == 2u) {
        vector = random_on_cube(&seed) * scale;
    }

    let gravitational_constant = 10.0;
    let orbital_speed = sqrt(gravitational_constant / scale);

    let up = vec3<f32>(0.0, 1.0, 0.0);
    let tangent = normalize(cross(vector, up));

    let velocity = vec4(tangent * orbital_speed, 0.0);

    particles[write_index].position = uniforms.position + vec4(vector, 0.0);
    particles[write_index].velocity = velocity;
    particles[write_index].mass = 1.0;
    particles[write_index].lifetime = uniforms.lifetime;
}