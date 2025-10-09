struct VertexInput
{
    float3 Position : TEXCOORD0;
    float2 TexCoord : TEXCOORD1;
};

struct VertexOutput
{
    float2 TexCoord : SV_Target0;
    float4 Position : SV_Position;
};

[[vk::binding(0, 0)]]
StructuredBuffer<float4x4> Matrices : register(t0, space0);

[[vk::binding(0, 2)]]
Texture2D<float4> Texture : register(t0, space2);
[[vk::binding(0, 2)]]
SamplerState Sampler : register(s0, space2);

[shader("vertex")]
VertexOutput VertexMain(VertexInput input)
{
    float4x4 view = Matrices[0];
    float4x4 projection = Matrices[1];

    var mvp = projection * view;

    VertexOutput output;
    output.TexCoord = input.TexCoord;
    // output.Position = float4(input.Position, 1.0);
    output.Position = mul(float4(input.Position, 1.0), mvp);
    return output;
}

[shader("pixel")]
float4 FragmentMain(VertexOutput in) : SV_Target0
{
    return Texture.Sample(Sampler, in.TexCoord);
    // return Sampler.Sample(in.TexCoord);
}
