struct Particle {
    position: vec4<f32>,
    velocity: vec4<f32>,
    mass: f32,
    lifetime: f32
}

struct ComputeUniforms {
    gravity_center: vec4<f32>,
    gravity_strength: f32,
    rotation_speed: f32,
    drag_strength: f32,
    delta_time: f32,
}

@group(0) @binding(0) var<storage, read> particles_in: array<Particle>;
@group(0) @binding(1) var<storage, read_write> particles_out: array<Particle>;
@group(0) @binding(2) var<uniform> uniforms: ComputeUniforms;

@compute @workgroup_size(64)
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
    let force_magnitude = uniforms.gravity_strength / (safe_distance * safe_distance);
    let acceleration = direction * force_magnitude;

    // Apply drag/damping
    let drag = particle.velocity * uniforms.drag_strength;

    // Update velocity and position
    var new_velocity = particle.velocity + (acceleration - drag) * dt;
    var new_position = particle.position + new_velocity * dt;
    var new_mass = particle.mass;

    // Write to output buffer
    particles_out[index].position = new_position;
    particles_out[index].velocity = new_velocity;
    particles_out[index].mass = new_mass;
    particles_out[index].lifetime = particle.lifetime - dt;
}
