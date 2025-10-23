struct UpdateUniforms {
    gravity_center: vec4<f32>,
    elapsed_time: f32,
    delta_time: f32,
}

struct Particle {
    position: vec4<f32>,
    velocity: vec4<f32>,
    mass: f32,
    lifetime: f32,
    age: f32,
}

@group(0) @binding(0) var<uniform> uniforms: UpdateUniforms;
@group(0) @binding(1) var<storage, read> particles_in: array<Particle>;
@group(0) @binding(2) var<storage, read_write> particles_out: array<Particle>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= arrayLength(&particles_in)) {
        return;
    }

    let dt = uniforms.delta_time;

    // Read from input buffer
    let particle = particles_in[index];

    // Calculate direction and distance to gravity center
    let to_center = uniforms.gravity_center - particle.position;
    let distance = length(to_center);

    // Prevent division by zero and extreme forces
    let min_distance = 0.1;
    let safe_distance = max(distance, min_distance);

    // Calculate gravitational force (F = G * m / r^2)
    let direction = normalize(to_center);
    let force_magnitude = 10.0 / (safe_distance * safe_distance);
    let acceleration = direction * force_magnitude;

    // Update velocity and position
    let velocity = particle.velocity + acceleration * dt;
    let position = particle.position + velocity * dt;
    let mass = particle.mass;
    let lifetime = particle.lifetime;
    let age = particle.age + dt;

    // Write to output buffer
    particles_out[index].position = position;
    particles_out[index].velocity = velocity;
    particles_out[index].mass = mass;
    particles_out[index].lifetime = lifetime;
    particles_out[index].age = age;
}
