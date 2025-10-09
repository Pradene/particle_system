struct Particle {
    position: vec4<f32>,
    velocity: vec4<f32>,
    color: vec4<f32>,
    mass: f32,
    lifetime: f32,
}

struct EmitUniforms {
    frame: u32,
    count: u32,
    lifetime: f32,
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> uniforms: EmitUniforms;
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

@compute @workgroup_size(256)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    let index = global_id.x;
    
    if (index >= uniforms.count) {
        return;
    }

    let write_index = atomicAdd(&particle_count, 1u);
    if (write_index >= arrayLength(&particles)) {
        return;
    }

    var seed = hash(hash(write_index) ^ (uniforms.frame * 7919u));

    let min_radius = 8.0;
    let max_radius = 16.0;
    let radius = random_range(&seed, min_radius, max_radius);

    let direction = random_on_sphere(&seed);
    let position = direction * radius;

    let gravitational_constant = 10.0;
    let orbital_speed = sqrt(gravitational_constant / radius);

    let up = vec3<f32>(0.0, 1.0, 0.0);
    var tangent = normalize(cross(direction, up));

    if (length(tangent) < 0.1) {
        tangent = normalize(cross(direction, vec3<f32>(1.0, 0.0, 0.0)));
    }

    let inclination = random_range(&seed, -0.8, 0.8);
    tangent = normalize(tangent + direction * inclination);

    let velocity = tangent * orbital_speed;

    particles[write_index].position = vec4(position, 1.0);
    particles[write_index].velocity = vec4(velocity, 0.0);
    particles[write_index].color = vec4(1.0, 0.75, 0.80, 0.1);
    particles[write_index].mass = 1.0;
    particles[write_index].lifetime = uniforms.lifetime;
}