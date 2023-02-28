struct VsOut {
    float4 position: SV_Position;
    [[vk::location(0)]] float3 color: TEXCOORD0;
};

// static float3 colors[3] = {
//     float3(1.0, 0.0, 0.0),
//     float3(0.0, 1.0, 0.0),
//     float3(0.0, 0.0, 1.0)
// };

static float2 vertices[3] = {
    float2(0.0, -0.5),
    float2(0.5, 0.5),
    float2(-0.5, 0.5)
};

[[vk::binding(0, 0)]] StructuredBuffer<float4> colors;

VsOut main(uint vid: SV_VertexID) {
    VsOut vsout;

    float2 pos = vertices[vid];

    vsout.position = float4(pos.x, pos.y, 0, 1.0);
    vsout.color = colors[vid].xyz;

    return vsout;
}
