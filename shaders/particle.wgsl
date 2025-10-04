struct Particle {
    position: vec3<f32>,
    velocity: vec3<f32>,
    mass: f32,
    lifetime: f32,
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

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= arrayLength(&particles)) {
        return;
    }

    var seed = hash(index * 123456789u);

    // Generate random orbital radius (distance from center/planet)
    let min_radius = 4.0;
    let max_radius = 16.0;
    let radius = random_range(&seed, min_radius, max_radius);

    // Position on sphere at given radius
    let direction = random_on_sphere(&seed);
    let position = direction * radius;

    // Calculate orbital velocity using v = sqrt(GM/r)
    // For game/demo purposes, we use a simplified constant
    let gravitational_constant = 10.0; // Adjust for desired speed
    let orbital_speed = sqrt(gravitational_constant / radius);

    // Velocity is perpendicular to position vector (tangent to orbit)
    // Cross product with a random up vector creates circular motion
    let up = vec3<f32>(0.0, 1.0, 0.0);
    var tangent = normalize(cross(direction, up));

    // If direction is parallel to up, use different axis
    if (length(tangent) < 0.1) {
        tangent = normalize(cross(direction, vec3<f32>(1.0, 0.0, 0.0)));
    }

    // Add some randomness to orbit inclination
    let inclination = random_range(&seed, -0.3, 0.3);
    tangent = normalize(tangent + direction * inclination);

    let velocity = tangent * orbital_speed;

    particles[index].position = position;
    particles[index].velocity = velocity;
    particles[index].mass = 1.0;
    particles[index].lifetime = 4.0;
}
