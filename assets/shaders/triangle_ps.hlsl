struct PsIn {
    [[vk::location(0)]] float3 color: TEXCOORD0;
};

struct PsOut {
    float4 color: SV_TARGET0;
};

PsOut main(PsIn ps) {
    PsOut ps_out;
    ps_out.color = float4(ps.color, 1.0);
    return ps_out;
}
