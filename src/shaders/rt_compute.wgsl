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
    first_geometry: u32,
    last_geometry: u32,
    _pad: u32
};

struct Material {
    roughness_exponent: f32,
    metalness: f32,
    specularity: f32,
    albedo: vec3<f32>,
    emissive: vec3<f32>,
    emissive_strength: f32
}

struct Geometry {
    first_index: u32,
    material: Material,
};

@group(0) @binding(0)
var output: texture_storage_2d<rgba8unorm, write>;

@group(0) @binding(1)
var<uniform> uniforms: Uniforms;

@group(0) @binding(2)
var<storage, read> vertices: array<Vertex>;

@group(0) @binding(3)
var<storage, read> indices: array<u32>;

@group(0) @binding(4)
var<storage, read> geometries: array<Geometry>;

@group(0) @binding(5)
var<storage, read> instances: array<Instance>;

@group(0) @binding(6)
var acc_struct: acceleration_structure;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let target_size = textureDimensions(output);
    var color = vec4<f32>(vec2<f32>(global_id.xy) / vec2<f32>(target_size), 0.0, 1.0);

    let pixel_center = vec2<f32>(global_id.xy) + vec2<f32>(0.5);
    let in_uv = pixel_center / vec2<f32>(target_size.xy);
    let d = in_uv * 2.0 - 1.0;

    let origin = (uniforms.view_inv * vec4<f32>(0.0, 0.0, 0.0, 1.0)).xyz;
    let temp = uniforms.proj_inv * vec4<f32>(d.x, d.y, 1.0, 1.0);
    let direction = (uniforms.view_inv * vec4<f32>(normalize(temp.xyz), 0.0)).xyz;

    var rq: ray_query;
    rayQueryInitialize(&rq, acc_struct, RayDesc(0u, 0xFFu, 0.1, 200.0, origin, direction));
    rayQueryProceed(&rq);

    let intersection = rayQueryGetCommittedIntersection(&rq);
    if intersection.kind != RAY_QUERY_INTERSECTION_NONE {
        let instance = instances[intersection.instance_custom_data];
        let geometry = geometries[intersection.geometry_index + instance.first_geometry];

        let index_offset = geometry.first_index;
        let vertex_offset = instance.first_vertex;

        let first_index_index = intersection.primitive_index * 3u + index_offset;

        let v_0 = vertices[vertex_offset + indices[first_index_index + 0u]];
        let v_1 = vertices[vertex_offset + indices[first_index_index + 1u]];
        let v_2 = vertices[vertex_offset + indices[first_index_index + 2u]];

        let bary = vec3<f32>(1.0 - intersection.barycentrics.x - intersection.barycentrics.y, intersection.barycentrics);

        let pos = v_0.pos * bary.x + v_1.pos * bary.y + v_2.pos * bary.z;
        let normal_raw = v_0.normal * bary.x + v_1.normal * bary.y + v_2.normal * bary.z;
        // let uv = v_0.uv * bary.x + v_1.uv * bary.y + v_2.uv * bary.z;

        let normal = normalize(normal_raw);

        let material = geometry.material;

        if material.emissive_strength > 0.0 {
            color = vec4<f32>(material.emissive, 1.0);
        } else {
            color = vec4<f32>(material.albedo, 1.0);
        }
    }

    textureStore(output, global_id.xy, color);
}
