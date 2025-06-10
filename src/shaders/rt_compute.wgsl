struct Uniforms {
    view_inv: mat4x4<f32>,
    proj_inv: mat4x4<f32>,
};

struct Vertex {
    pos: vec3<f32>,
    normal: vec3<f32>,
};


struct Instance {
    first_vertex: u32,
    first_index: u32,
    material_index: u32,
    _pad: u32
};

struct Material {
    albedo: vec3<f32>,
    emissive: vec3<f32>,
    emissive_strength: f32
}

@group(0) @binding(0)
var output: texture_storage_2d<rgba8unorm, write>;

@group(0) @binding(1)
var<uniform> uniforms: Uniforms;

@group(0) @binding(2)
var<storage, read> vertices: array<Vertex>;

@group(0) @binding(3)
var<storage, read> indices: array<u32>;

@group(0) @binding(4)
var<storage, read> materials: array<Material>;

@group(0) @binding(5)
var<storage, read> instances: array<Instance>;

@group(0) @binding(6)
var acc_struct: acceleration_structure;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let target_size = textureDimensions(output);

    let pixel_center = vec2<f32>(global_id.xy) + vec2<f32>(0.5);
    let in_uv = pixel_center / vec2<f32>(target_size.xy);
    var d = in_uv * 2.0 - 1.0;
    d.y = -d.y; // Flip so objects with +y are on the top and -y are on the bottom

    let origin = (uniforms.view_inv * vec4<f32>(0.0, 0.0, 0.0, 1.0)).xyz;
    let temp = uniforms.proj_inv * vec4<f32>(d.x, d.y, 1.0, 1.0);
    let direction = (uniforms.view_inv * vec4<f32>(normalize(temp.xyz), 0.0)).xyz;

    let pixel_index = global_id.x + global_id.y * target_size.x;
    var state = pixel_index;

    var color = vec3<f32>();

    let rays_per_pixel = 100u;
    for (var i: u32 = 0; i < rays_per_pixel; i++) {
        color += trace_ray(origin, direction, &state);
    }

    textureStore(output, global_id.xy, vec4<f32>(color / f32(rays_per_pixel), 1.0));
}

fn trace_ray(initial_origin: vec3<f32>, initial_direction: vec3<f32>, state: ptr<function, u32>) -> vec3<f32> {
    var origin = initial_origin;
    var direction = initial_direction;

    var light = vec3<f32>();
    var color = vec3<f32>(1.0, 1.0, 1.0);

    var rq: ray_query;

    for (var i: u32 = 0; i < 10; i++) {
        rayQueryInitialize(&rq, acc_struct, RayDesc(0u, 0xFFu, 0.001, 100.0, origin, direction));

        if rayQueryProceed(&rq) {
            // The closest hit is `Candidate` and not `Committed`
            break;
        }

        let intersection = rayQueryGetCommittedIntersection(&rq);
        if intersection.kind == RAY_QUERY_INTERSECTION_NONE {
            // Sky color
            light += (vec3<f32>(143.0, 210.0, 255.0) / 255.0) * color;
            break;
        }

        let instance = instances[intersection.instance_custom_data];

        let index_offset = instance.first_index;
        let vertex_offset = instance.first_vertex;

        let first_index_index = intersection.primitive_index * 3u + index_offset;

        let v_0 = vertices[vertex_offset + indices[first_index_index + 0u]];
        let v_1 = vertices[vertex_offset + indices[first_index_index + 1u]];
        let v_2 = vertices[vertex_offset + indices[first_index_index + 2u]];

        let bary = vec3<f32>(1.0 - intersection.barycentrics.x - intersection.barycentrics.y, intersection.barycentrics);

        let local_pos = v_0.pos * bary.x + v_1.pos * bary.y + v_2.pos * bary.z;
        let pos = (intersection.object_to_world * vec4<f32>(local_pos, 1.0)).xyz;
        let normal_raw = v_0.normal * bary.x + v_1.normal * bary.y + v_2.normal * bary.z;
        let normal = normalize((intersection.object_to_world * vec4<f32>(normal_raw, 0.0)).xyz);

        origin = pos;
        direction = normalize(normal + random_direction(state)); // Lambertian distribution

        let material = materials[instance.material_index];

        light += material.emissive * material.emissive_strength * color;
        color *= material.albedo;
    }

    return light;
}

fn pcg_random(state: ptr<function, u32>) -> f32 {
    *state = *state * 747796405u + 2891336453u;

    var word = ((*state >> ((*state >> 28u) + 4u)) ^ *state) * 277803737u;
    word = (word >> 22u) ^ word;

    return f32(word) / 4294967295.0;
}

fn random_normal_dist(state: ptr<function, u32>) -> f32 {
    let theta = 2 * 3.1415926 * pcg_random(state);
    let rho = sqrt(-2 * log(pcg_random(state)));

    return rho * cos(theta);
}

fn random_direction(state: ptr<function, u32>) -> vec3<f32> {
    return normalize(vec3<f32>(random_normal_dist(state), random_normal_dist(state), random_normal_dist(state)));
}
