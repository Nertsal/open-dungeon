varying vec2 v_vt;

#ifdef VERTEX_SHADER
attribute vec2 a_pos;
attribute vec2 a_vt;

void main() {
    v_vt = a_vt;
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
#endif

#ifdef FRAGMENT_SHADER
// Adapted from <https://www.shadertoy.com/view/Ms3XWH>

uniform sampler2D u_texture;
uniform float u_time;
uniform float u_intensity;

const float range = 0.02;
const float noiseQuality = 250.0;
const float noiseIntensity = 0.0008;
const float offsetIntensity = 0.005;
const float colorOffsetIntensity = 0.5;

float rand(vec2 co)
{
    return fract(sin(dot(co.xy ,vec2(12.9898,78.233))) * 43758.5453);
}

float verticalBar(float pos, float uvY, float offset)
{
    float edge0 = (pos - range);
    float edge1 = (pos + range);

    float x = smoothstep(edge0, pos, uvY) * offset;
    x -= smoothstep(pos, edge1, uvY) * offset;
    return x;
}

void main()
{
    vec2 uv = v_vt;
    
    for (float i = 0.0; i < 0.71; i += 0.1313)
    {
        float d = mod(u_time * i, 1.7);
        float o = sin(1.0 - tan(u_time * 0.24 * i));
    	o *= offsetIntensity * u_intensity;
        uv.x += verticalBar(d, uv.y, o);
    }
    
    float uvY = uv.y;
    uvY *= noiseQuality;
    uvY = float(int(uvY)) * (1.0 / noiseQuality);
    float noise = rand(vec2(u_time * 0.00001, uvY));
    uv.x += noise * noiseIntensity * u_intensity;

    vec2 offsetR = vec2(0.006 * sin(u_time), 0.0) * colorOffsetIntensity * u_intensity;
    vec2 offsetG = vec2(0.0073 * (cos(u_time * 0.97)), 0.0) * colorOffsetIntensity * u_intensity;
    
    float r = texture2D(u_texture, uv + offsetR).r;
    float g = texture2D(u_texture, uv + offsetG).g;
    float b = texture2D(u_texture, uv).b;

    vec4 tex = vec4(r, g, b, 1.0);
    gl_FragColor = tex;
}
#endif
