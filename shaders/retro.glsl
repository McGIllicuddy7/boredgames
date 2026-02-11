#version 330

// Input vertex attributes (from vertex shader)
in vec2 fragTexCoord;
in vec4 fragColor;

// Input uniform values
uniform sampler2D texture0;
uniform vec4 colDiffuse;
uniform float scanline;
// Output fragment color
out vec4 finalColor;
float cbrt256 = 6.34960420787;
float div = 6.0;
uniform vec4 pallete[256];
float length_sqr(vec3 v) { return v.x * v.x + v.y * v.y + v.z * v.z; }
// NOTE: Add your custom variables here
float col_dif(vec3 c1, vec3 c2) {
  float dv = (length_sqr(c1) - length_sqr(c2));
  dv *= dv;
  dv = dv;
  if (length(c1) == 0 || length(c2) == 0) {
    return dv;
  }
  float dt = (1 - dot(normalize(c1), normalize(c2))) / 2.0;
  return dt + dv;
}
vec3 nearest_rgb_to_pallet(vec3 col) {
  vec3 nearest = pallete[0].rgb;
  float min_delta = col_dif(col, nearest);
  float l = length(col);
  int base = 0;
  if (l > 1) {
    base = 3 * 64;
  } else if (l > 0.6) {
    base = 2 * 64;
  } else if (l > 0.3) {
    base = 64;
  } else {
    base = 0;
  }
  for (int i = base; i < base + 64; i += 1) {
    float delta = col_dif(col, pallete[i].rgb);
    if (col == pallete[i].rgb) {
      return pallete[i].rgb;
    }
    if (delta < min_delta) {
      min_delta = delta;
      nearest = pallete[i].rgb;
    }
  }
  if (base != 3 * 64) {
    for (int i = 256 - 16; i < 256; i += 1) {
      float delta = col_dif(col, pallete[i].rgb);
      if (col == pallete[i].rgb) {
        return pallete[i].rgb;
      }
      if (delta < min_delta) {
        min_delta = delta;
        nearest = pallete[i].rgb;
      }
    }
  }

  return nearest;
}

void main() {
  // Texel color fetching from texture sampler
  // vec4 texelColor = texture(texture0, fragTexCoord);

  // NOTE: Implement here your fragment shader code

  // final color is the color from the texture
  //    times the tint color (colDiffuse)
  //    times the fragment color (interpolated vertex color)
  // finalColor = texelColor * colDiffuse * fragColor;

  vec4 base = texture(texture0, fragTexCoord);
  vec4 col = vec4(0.0, 0.0, 0.0, 1.0);
  col.rgb = nearest_rgb_to_pallet(base.rgb);
  col.a = base.a;
  float count = 600;
  float h = fragTexCoord.y * count;
  float p = count - scanline * count;
  float line_delta = abs(round(h) - h);
  if (line_delta < 0.5) {
    if (line_delta < 0.01) {
      line_delta = 0.01;
    }
    col.rgb += (1. / line_delta * sqrt(line_delta)) * 0.005;
  }
  if (abs(h - p) < 0.7) {
    float delt = abs(h - p);
    delt *= 10;
    if (delt < 0.1) {
      delt = 0.1;
    }
    col.rgb += (1. / (delt * sqrt(delt))) * 0.005;
    col.rgb *= 1.0;
  }
  finalColor = col;
}
