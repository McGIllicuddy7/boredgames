#version 330

// This shader is based on the basic lighting shader
// This only supports one light, which is directional, and it (of course)
// supports shadows

// Input vertex attributes (from vertex shader)

const int directionCount = 16;
in vec3 fragPosition;
in vec2 fragTexCoord;
// in vec4 fragColor;
in vec3 fragNormal;

// Input uniform values
uniform sampler2D texture0;
uniform vec4 colDiffuse;

// Output fragment color
out vec4 finalColor;

// lighting
uniform vec3 lightPositions[16];
uniform vec4 lightColors[16];
uniform float lightRadii[16];
uniform int lightEnabled[16];
uniform float lightDistances[16 * 26];
uniform vec3 directions[26];
struct Light {
  vec3 pos;
  vec4 col;
  float radius;
  int enabled;
  float distances[26];
};

Light get_light(int idx) {
  Light ot;
  ot.pos = lightPositions[idx];
  ot.col = lightColors[idx];
  ot.radius = lightRadii[idx];
  ot.enabled = lightEnabled[idx];
  for (int i = 0; i < 26; i++) {
    ot.distances[i] = lightDistances[idx * 26 + i];
  }
  return ot;
}

vec4 handle_light(int idx) {
  Light l = get_light(idx);
  if (l.enabled == 0) {
    return vec4(0.0, 0.0, 0.0, 1.0);
  }
  vec3 disp = (l.pos - fragPosition);
  float dist = length(disp);
  if (dist > l.radius) {
    return vec4(0.0, 0.0, 0.0, 1.0);
  }
  vec3 ndisp = disp / dist;
  for (int i = 0; i < 26; i++) {
    if (dot(-ndisp, directions[i]) > 0.8) {
      if (dist > l.distances[i]) {
        return vec4(0.0, 0.0, 0.0, 1.0);
      }
    }
  }
  float delt = max(dot(ndisp, fragNormal), 0.0);
  vec3 ot = vec3(1.0);
  return vec4(delt * ot / (length(disp)), 1.0);
}
void main() {
  vec4 col = vec4(0.0);
  for (int i = 0; i < 16; i++) {
    col += handle_light(i);
  }
  col += vec4(0.02, 0.025, 0.025, 0.0);
  finalColor = texture(texture0, fragTexCoord) * col;
  finalColor = pow(finalColor, vec4(1. / 2.2));
}