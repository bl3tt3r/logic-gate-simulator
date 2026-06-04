#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> background_color: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> line_space: f32;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> line_thickness: f32;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var<uniform> line_color: vec4<f32>;

// Intensité du trait de grille le plus proche en world-space.
// pixel_size (via fwidth) permet d'adapter l'épaisseur au niveau de zoom.
fn grid_intensity(v: f32, pixel_size: f32) -> f32 {
    // Distance au trait le plus proche en world-space
    let dist = abs(fract(v / line_space + 0.5) - 0.5) * line_space;

    // Au moins 0.5px pour rester visible au dézoom
    let half_t = max(line_thickness * 0.5, pixel_size * 0.5);

    // smoothstep au lieu de step : fondu sur ±1px pour éviter l'aliasing
    return 1.0 - smoothstep(half_t - pixel_size, half_t + pixel_size, dist);
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let pos = mesh.world_position.xy;

    // Taille d'un pixel en world-space : grandit quand on dézoome
    let pixel_x = fwidth(pos.x);
    let pixel_y = fwidth(pos.y);

    // Grille : union des traits verticaux et horizontaux
    let gx = grid_intensity(pos.x, pixel_x);
    let gy = grid_intensity(pos.y, pixel_y);
    let grid = max(gx, gy);

    // Axes centraux (x=0, y=0) : 2x plus épais, même anti-aliasing
    let half_t_thick = max(line_thickness * 1.5, pixel_x * 0.5);
    let ax = 1.0 - smoothstep(half_t_thick - pixel_x, half_t_thick + pixel_x, abs(pos.x));
    let ay = 1.0 - smoothstep(half_t_thick - pixel_y, half_t_thick + pixel_y, abs(pos.y));
    let axis = max(ax, ay);

    // Superpose grille et axes, mélange avec les couleurs
    let line = max(grid, axis);

    // Couleur de ligne à 25% d'opacité pour la grille, 100% pour les axes centraux
    let line_color_transparent = vec4<f32>(line_color.rgb, line_color.a);
    let blended_color = mix(line_color_transparent, line_color, axis);

    return mix(background_color, blended_color, line);
}