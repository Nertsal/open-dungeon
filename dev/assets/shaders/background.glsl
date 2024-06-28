varying vec2 v_vt;
varying vec2 v_pos;

#ifdef VERTEX_SHADER
attribute vec2 a_pos;
attribute vec2 a_vt;

void main() {
    v_vt = a_vt;
    v_pos = a_pos;
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
#endif

#ifdef FRAGMENT_SHADER
uniform sampler2D u_texture;
uniform float u_time;
uniform vec4 u_mask_color;
uniform vec4 u_mask2_color;
uniform mat3 u_world_matrix;

void main() {
    vec4 tex_color = texture2D(u_texture, v_vt);
    if (tex_color != u_mask_color && tex_color != u_mask2_color) {
        gl_FragColor = tex_color;
        return;
    }
    
    vec3 world_pos = u_world_matrix * vec3(v_pos, 1.0);
    vec2 pos = world_pos.xy / world_pos.z;
    float size = 2.0;
    pos = floor(pos / size);
    float pattern_mask = mod(pos.x + mod(pos.y, 2.0), 2.0);
    gl_FragColor = tex_color + pattern_mask * vec4(vec3(0.05), 1.0);
}
#endif

