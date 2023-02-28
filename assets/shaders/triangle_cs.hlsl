[[vk::binding(0, 0)]] RWStructuredBuffer<float4> output_buffer;

static float4 colors[3] = {
    float4(1.0, 0.0, 0.0, 1.0),
    float4(0.0, 1.0, 0.0, 1.0),
    float4(0.0, 0.0, 1.0, 1.0)
};


[[vk::push_constant]]
struct {
    uint rgb_index[3];
} push_constants;

[numthreads(3, 1, 1)]
void main(uint3 thread_id : SV_DispatchThreadID)
{
    if (thread_id.x < 3)
    {
        output_buffer[thread_id.x] = colors[push_constants.rgb_index[thread_id.x]];
    }
}
