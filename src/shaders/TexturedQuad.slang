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

// Texture2D<float4> Texture;
// SamplerState Sampler;

// Texture2D<float4> Texture : register(t0, space0);
// SamplerState Sampler : register(s0, space0);

[[vk::binding(0, 2)]]
Texture2D<float4> Texture : register(t0, space2);
[[vk::binding(0, 2)]]
SamplerState Sampler : register(s0, space2);

// Sampler2D<float4> Sampler : register(s0, space0);

[shader("vertex")]
VertexOutput VertexMain(VertexInput input)
{
    VertexOutput output;
    output.TexCoord = input.TexCoord;
    output.Position = float4(input.Position, 1.0);
    return output;
}

[shader("pixel")]
float4 FragmentMain(VertexOutput in) : SV_Target0
{
    return Texture.Sample(Sampler, in.TexCoord);
    // return Sampler.Sample(in.TexCoord);
}
