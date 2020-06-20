//! A small simple ray-casted 3d (ish) game. It uses the same method that wolfenstein 3d used to have "3d".
//!
//! I built this using the OpenGL wrapper glium.

use std::time;
use std::collections::HashMap;
use glium::{glutin, Surface, Display, Program, Frame};
use glium::texture::Texture2d;

const GAME_HEIGHT: usize = 12;
const GAME_WIDTH: usize = 12;
const GAME: [u8;144] = [
    1,1,1,1,1,1,1,1,1,1,1,1,
    1,0,1,0,0,0,0,1,0,0,0,1,
    1,0,1,0,1,1,0,0,0,3,0,1,
    1,0,0,0,1,0,0,1,0,0,0,1,
    1,3,2,0,1,0,1,1,0,0,0,1,
    1,0,0,0,1,0,0,1,0,1,1,1,
    1,0,1,0,1,1,0,1,0,1,1,1,
    1,1,0,1,1,1,0,1,0,0,0,1,
    1,0,0,0,0,0,0,1,0,1,1,1,
    1,0,3,1,1,1,0,1,0,1,0,1,
    1,0,0,0,0,0,2,1,0,0,0,1,
    1,1,1,1,1,1,1,1,1,1,1,1,
];

const MOVE_SPEED: f32 = 4.0 / GAME_HEIGHT as f32;
const LOOK_SPEED: f32 = 2.0;

const RAYS: usize = 256;
const FOV: f32 = 1.2;

const START_POS: PlayerPos = PlayerPos { position: [0.8, 0.8], dir: 3.7 };

const COLORS: bool = false;

enum ColorTex<'a>
{
    Color(&'a Texture2d, (f32,f32,f32)),
    Texture(&'a Texture2d, ([f32; 2],[f32; 2],[f32; 2],[f32; 2]))
}

#[derive(Copy, Clone)]
struct Vertex 
{
    position: [f32; 2],
    tex_coords: [f32; 2],
}
glium::implement_vertex!(Vertex, position, tex_coords);

#[derive(Copy, Clone)]
struct PlayerPos
{
    position: [f32; 2],
    dir: f32
}

#[derive(Copy, Clone)]
struct Pos
{
    position: [f32; 2],
}

/// Draws a quad from 2 triangles.
/// ```text
///  ___
/// |\  |
/// | \ |
/// |  \|
///  ---
/// ```
fn draw_quad(top_left: Pos, top_right: Pos, bottom_right: Pos, bottom_left: Pos, color_tex: ColorTex,
    mul: f32, target: &mut Frame, display: &Display, program: &Program)
{
    let tex_coords = match color_tex
    {
        ColorTex::Texture(_, coords) => coords,
        ColorTex::Color(_,_) => ([0.0,1.0],[1.0,1.0],[1.0,0.0],[0.0, 0.0])
    };

    let vertex1 = Vertex { position: top_left.position, tex_coords: tex_coords.0 };
    let vertex2 = Vertex { position: top_right.position, tex_coords: tex_coords.1 };
    let vertex3 = Vertex { position: bottom_right.position, tex_coords: tex_coords.2 };
    let vertex4 = Vertex { position: bottom_left.position, tex_coords: tex_coords.3 };

    let shape = vec![vertex1, vertex2, vertex3, vertex4];

    // upload shape data to video memory
    let shape_vb = glium::VertexBuffer::new(display, &shape).unwrap();
    let indices = glium::IndexBuffer::new(display, glium::index::PrimitiveType::TrianglesList, &[0u16,1,3,1,2,3][..]).unwrap();

    let uniforms = match color_tex
    {
        ColorTex::Color(empty_tex, color) => glium::uniform! {
            rgb_color: color,
            use_texture: false,
            tex: empty_tex,
            mult: mul
        },
        ColorTex::Texture(texture, _) => glium::uniform! {
            rgb_color: (0.0,0.0,0.0),
            use_texture: true,
            tex: texture,
            mult: mul
        }
    };

    target.draw(&shape_vb, &indices, program, &uniforms, &Default::default()).unwrap();
}

fn draw_rect(top_left: Pos, bottom_right: Pos, color_tex: ColorTex, mul: f32, target: &mut Frame,
    display: &Display, program: &Program)
{
    let top_right = Pos { position: [ bottom_right.position[0],  top_left.position[1]] };
    let bottom_left = Pos { position: [ top_left.position[0], bottom_right.position[1]] };

    draw_quad(top_left, top_right, bottom_right, bottom_left, color_tex, mul, target, display, program)
}

fn draw_line(v1: Pos, v2: Pos, color: (f32,f32,f32), mul: f32, empty_tex: &Texture2d, target: &mut Frame,
    display: &Display, program: &Program)
{
    let line = vec![
        Vertex { position: v1.position, tex_coords: [0.0,0.0] },
        Vertex { position: v2.position, tex_coords: [0.0,0.0] },
    ];
    let line_vb = glium::VertexBuffer::new(display, &line).unwrap();
    let indices = glium::index::NoIndices(glium::index::PrimitiveType::LinesList);

    let uniforms = glium::uniform! {
            rgb_color: color,
            use_texture: false,
            tex: empty_tex,
            mult: mul
    };
    
    target.draw(&line_vb, &indices, program, &uniforms, &Default::default()).unwrap();
}

fn at_wall(pos: (f32, f32), horz: bool) -> u8
{
    let pos2 = ((pos.0 + 1.0) * (GAME_WIDTH as f32) / 2.0, (pos.1 + 1.0) * (GAME_WIDTH as f32) / 2.0);
    let (mut col, mut row) = (f32::floor(pos2.0) as usize, f32::floor(pos2.1) as usize);

    if row as usize * GAME_WIDTH + col < GAME_WIDTH * GAME_HEIGHT
    {
        let v1;
        let v2;
        if horz
        {
            row = pos2.1.round() as usize; // We have to round here because of floating point rounding errors.
            v1 = GAME[row * GAME_WIDTH + col];
            if row != 0
            {
                v2 = GAME[(row-1) * GAME_WIDTH + col];
            }
            else
            {
                v2 = v1;
            }
        }
        else
        {
            col = pos2.0.round() as usize;
            v1 = GAME[row * GAME_WIDTH + col];
            if col != 0
            {
                v2 = GAME[row * GAME_WIDTH + (col - 1)];
            }
            else
            {
                v2 = v1;
            }
        }

        // If the two options are not wall (0) and wall (1) then we will say we are at a wall.
        // This also brings up if the two options are a normal wall (1) or a special wall (>1).
        // I chose to just pick the special wall because that made the code easier.
        // The only reason this would ever be an issue is if you are inside a wall looking at the
        // adjacent wall. It's hard to explain.
        u8::max(v1,v2)
    }
    else
    {
        0
    }
}

fn calc_dist_to_wall(player_pos: &PlayerPos, angle: f32) -> (f32, bool, u8, (f32,f32))
{
    let mut yoffset;
    let mut xoffset;
    
    let mut ray_y;
    let mut ray_x;
     
    let yrungs = (GAME_HEIGHT as f32) / 2.0;
    let xrungs = (GAME_WIDTH as f32) / 2.0;

    let mut dist_to_horz = 10000.0;
    let mut dist_to_vert = 10000.0;

    let mut horz_wall = 0;
    let mut vert_wall = 0;

    // Check Horizontal grid lines
    {
        yoffset = 2.0 / (GAME_HEIGHT as f32);
        if f32::sin(angle) > 0.0
        {
            ray_y = f32::ceil(player_pos.position[1] * yrungs) / yrungs;
        }
        else if f32::sin(angle) < 0.0
        {
            yoffset *= -1.0;
            ray_y = f32::floor(player_pos.position[1] * yrungs) / yrungs;
        }
        else
        {
            ray_y = player_pos.position[1];
        }
        ray_x = (ray_y - player_pos.position[1]) / f32::tan(angle) + player_pos.position[0];
        xoffset = yoffset / f32::tan(angle);

        while ray_x >= -1.0 && ray_x <= 1.0 && ray_y >= -1.0 && ray_y <= 1.0 && f32::sin(angle) != 0.0
        {
            horz_wall = at_wall((ray_x, ray_y), true);
            if horz_wall > 0
            {
                dist_to_horz = f32::sqrt((ray_y - player_pos.position[1]).powf(2.0) + (ray_x - player_pos.position[0]).powf(2.0));
                break;
            }
            ray_y += yoffset;
            ray_x += xoffset;
        }
    }
    let ray_x_h = ray_x;
    let ray_y_h = ray_y;
    
    // Check vertical grid lines
    {
        xoffset = 2.0 / (GAME_WIDTH as f32);
        if f32::cos(angle) > 0.0
        {
            ray_x = f32::ceil(player_pos.position[0] * xrungs) / xrungs;
        }
        else if f32::cos(angle) < 0.0
        {
            xoffset *= -1.0;
            ray_x = f32::floor(player_pos.position[0] * xrungs) / xrungs;
        }
        else
        {
            ray_x = player_pos.position[0];
        }
        ray_y = (ray_x - player_pos.position[0]) * f32::tan(angle) + player_pos.position[1];
        yoffset = xoffset * f32::tan(angle);

        while ray_x >= -1.0 && ray_x <= 1.0 && ray_y >= -1.0 && ray_y <= 1.0 && f32::cos(angle) != 0.0
        {
            vert_wall = at_wall((ray_x, ray_y), false);
            if vert_wall > 0
            {
                dist_to_vert = f32::sqrt((ray_y - player_pos.position[1]).powf(2.0) + (ray_x - player_pos.position[0]).powf(2.0));
                break;
            }
            ray_y += yoffset;
            ray_x += xoffset;
        }
    }

    // pick shortest
    if dist_to_horz < dist_to_vert
    {
        (dist_to_horz, true, horz_wall, (ray_x_h, ray_y_h))
    }
    else
    {
        (dist_to_vert, false, vert_wall, (ray_x, ray_y))
    }
    
}

fn ray_cast(player_pos: &PlayerPos, rays: usize, fov: f32) -> Vec<(usize, f32, f32, bool, u8, (f32,f32))>
{
    (0..rays)
        .map(|i| (i, player_pos.dir - fov/2.0 + i as f32 * fov / (rays as f32)))
        .map(|(i, ray_ang)| {let res = calc_dist_to_wall(player_pos, ray_ang); (i, ray_ang, res.0, res.1, res.2, res.3)})
        .collect()
}

fn get_colortex_from_wall<'a>(wall: u8, colors: bool, main_wall_texture: &'a Texture2d, 
    wall2_texture: &'a Texture2d, wall3_texture: &'a Texture2d, empty_tex: &'a Texture2d, 
    tex_coords: ([f32; 2], [f32; 2], [f32; 2], [f32; 2]))
    -> ColorTex<'a>
{
    match (wall, colors)
    {
        (3,true) => ColorTex::Color(empty_tex, (1.0/f32::sqrt(2.0), 0.0, 1.0/f32::sqrt(2.0))),
        (2,true) => ColorTex::Color(empty_tex, (0.0, 1.0, 0.0)),
        (1, true) => ColorTex::Color(empty_tex, (1.0, 0.0, 0.0)),
        (_, true) => ColorTex::Color(empty_tex, (0.0, 0.0, 0.0)),

        (3,false) => ColorTex::Texture(wall3_texture, tex_coords),
        (2,false) => ColorTex::Texture(wall2_texture, tex_coords),
        (1, false) => ColorTex::Texture(main_wall_texture, tex_coords),
        (_, false) => ColorTex::Texture(empty_tex, tex_coords),
    }
}

// TODO: dont re draw calc/create the rects every time
fn draw_3d_game(display: &Display, program: &Program, player_pos: &PlayerPos, main_wall_texture: &Texture2d, 
    wall2_texture: &Texture2d, wall3_texture: &Texture2d, empty_tex: &Texture2d)
{
    let mut target = display.draw();
    target.clear_color(0.0, 0.0, 1.0, 1.0);
    draw_rect(Pos{ position: [-1.0,1.0]}, Pos{ position: [1.0,0.0]}, 
        ColorTex::Color(empty_tex, (0.5, 0.5, 0.5)), 1.0, &mut target, display, program);

    let rays = RAYS;

    for (i, ray_ang, ray_dist, horz, wall, ray_pos) in ray_cast(player_pos, rays, FOV)
    {
        if ray_dist > 100.0 || wall == 0 { continue; }
        // I want to make the walls look more linear but I cant seem to figure out how.
        let dist = ray_dist*f32::cos(f32::abs(ray_ang - player_pos.dir));//f32::cos(f32::abs(ray_ang - player_pos.dir)/10.0);
        let height = (2.0/GAME_HEIGHT as f32) / dist;

        let pos_on_wall = if horz
        {
            let block_on = f32::floor((ray_pos.0 + 1.0) * (GAME_WIDTH as f32) / 2.0);
            let pos = (ray_pos.0 + 1.0 - block_on * (2.0/GAME_WIDTH as f32)) / (2.0/GAME_WIDTH as f32);
            if f32::sin(ray_ang) > 0.0 {1.0 - pos} else {pos}
        }
        else
        {
            let block_on = f32::floor((ray_pos.1 + 1.0) * (GAME_WIDTH as f32) / 2.0);
            let pos = (ray_pos.1 + 1.0 - block_on * (2.0/GAME_WIDTH as f32)) / (2.0/GAME_WIDTH as f32);
            if f32::cos(ray_ang) < 0.0 {1.0 - pos} else {pos}
        };
        let slice_width = f32::sin(FOV/RAYS as f32)*dist/(2.0/GAME_HEIGHT as f32);

        let tl = Pos { position: [(rays-i) as f32 * 2.0 / rays as f32 - 1.0, 0.0 + height] };
        let br = Pos { position: [(rays-i-1) as f32 * 2.0 / rays as f32 - 1.0, 0.0 - height] };
        
        let tex_coords = ([pos_on_wall,1.0],[pos_on_wall+slice_width,1.0],
            [pos_on_wall+slice_width,0.0],[pos_on_wall, 0.0]);

        let color_tex = get_colortex_from_wall(wall, COLORS, main_wall_texture, wall2_texture, wall3_texture, empty_tex, tex_coords);
        let mul = if horz {0.8} else {1.0};

        draw_rect(tl, br, color_tex, mul, &mut target, display, program);
    }

    target.finish().unwrap();
}

fn draw_2d_game(display: &Display, program: &Program, player_pos: &PlayerPos, main_wall_texture: &Texture2d, 
    wall2_texture: &Texture2d, wall3_texture: &Texture2d, empty_tex: &Texture2d)
{
    let mut target = display.draw();
    target.clear_color(0.5, 0.5, 0.5, 1.0);

    // draw board
    for row in 0..GAME_HEIGHT
    {
        for col in 0..GAME_WIDTH
        {
            let tile = GAME[row * GAME_WIDTH + col];
            let padding_h = 0.02 / (GAME_HEIGHT as f32);
            let padding_w = 0.02 / (GAME_HEIGHT as f32);
            let this_tl = Pos { position: [
                (col as f32 * 2.0 / (GAME_WIDTH as f32)) - 1.0 + padding_w, 
                (row as f32 * 2.0 / (GAME_HEIGHT as f32)) - 1.0 + padding_h
            ] };
            let this_br = Pos { position: [
                ((col + 1) as f32 * 2.0 / (GAME_WIDTH as f32)) - 1.0 - padding_w, 
                ((row + 1) as f32 * 2.0 / (GAME_HEIGHT as f32)) - 1.0 - padding_h
            ] };

            let tex_coords = ([0.0,1.0],[1.0,1.0],[1.0,0.0],[0.0, 0.0]);

            let color_tex = get_colortex_from_wall(tile, COLORS, main_wall_texture, wall2_texture, wall3_texture, empty_tex, tex_coords);
            
            draw_rect(this_tl, this_br, color_tex, 1.0, &mut target, display, program);
        }
    }

    // draw player
    let player_size = 0.05;
    let player_ver = Pos {position: player_pos.position };
    let player_tl = Pos { position: [player_pos.position[0] - player_size/2.0, player_pos.position[1] - player_size/2.0] };
    let player_br = Pos { position: [player_pos.position[0] + player_size/2.0, player_pos.position[1] + player_size/2.0] };
    let player_dir = Pos { position: [player_pos.position[0] + 0.1*f32::cos(player_pos.dir), player_pos.position[1] + 0.1*f32::sin(player_pos.dir)] };
    draw_rect(player_tl, player_br, ColorTex::Color(empty_tex, (0.1, 0.9, 0.1)), 1.0, &mut target, display, program);
    draw_line(player_ver, player_dir, (1.0,1.0,0.0), 1.0, empty_tex, &mut target, display, program);

    // draw rays
    for (_, ray_ang, ray_dist, _, wall, _) in ray_cast(player_pos, RAYS, FOV)
    {
        let color = match wall
        {
            3 => (1.0/f32::sqrt(2.0), 0.0, 1.0/f32::sqrt(2.0)),
            2 => (0.0, 1.0, 0.0),
            1 => (1.0, 0.0, 0.0),
            _ => (0.0, 0.0, 0.0)
        };
        let ray_dir_ver = Pos { position: [player_pos.position[0] + ray_dist*f32::cos(ray_ang), player_pos.position[1] + ray_dist*f32::sin(ray_ang)] };
        
        draw_line(player_ver, ray_dir_ver, color, 1.0, empty_tex, &mut target, display, program);
    }

    target.finish().unwrap();
}

fn main_loop(display: &Display, program: &Program, player_pos: &PlayerPos, draw_3d: bool, 
    main_wall_texture: &Texture2d, wall2_texture: &Texture2d, wall3_texture: &Texture2d, empty_tex: &Texture2d)
{
    if draw_3d
    {
        draw_3d_game(display, program, player_pos, main_wall_texture, wall2_texture, wall3_texture, empty_tex);
    }
    else
    {
        draw_2d_game(display, program, player_pos, main_wall_texture, wall2_texture, wall3_texture, empty_tex);
    }
}

fn move_player(keys: &HashMap<glutin::event::VirtualKeyCode,glutin::event::VirtualKeyCode>, 
    player_pos: &mut PlayerPos, frame_time: f32)
{
    let rays = [calc_dist_to_wall(player_pos, 0.0).0,
        calc_dist_to_wall(player_pos, std::f32::consts::PI / 2.0).0,
        calc_dist_to_wall(player_pos, std::f32::consts::PI).0, 
        calc_dist_to_wall(player_pos, - std::f32::consts::PI / 2.0).0];
    let min_dist = 0.2/GAME_HEIGHT as f32;
    let move_speed = MOVE_SPEED * frame_time;
    let look_speed = LOOK_SPEED * frame_time;

    let mut x_move = 0.0;
    let mut y_move = 0.0;

    if keys.contains_key(&glutin::event::VirtualKeyCode::W)
    {
        x_move += move_speed * f32::cos(player_pos.dir);
        y_move += move_speed * f32::sin(player_pos.dir);
    }
    if keys.contains_key(&glutin::event::VirtualKeyCode::S)
    {
        x_move -= move_speed * f32::cos(player_pos.dir);
        y_move -= move_speed * f32::sin(player_pos.dir);
    }
    if keys.contains_key(&glutin::event::VirtualKeyCode::A) 
    {
        x_move -= move_speed * f32::sin(player_pos.dir);
        y_move += move_speed * f32::cos(player_pos.dir);
    }
    if keys.contains_key(&glutin::event::VirtualKeyCode::D)
    {
        x_move += move_speed * f32::sin(player_pos.dir);
        y_move -= move_speed * f32::cos(player_pos.dir);
    }

    if rays[0] >= min_dist && x_move > 0.0 || rays[2] >= min_dist && x_move < 0.0
    {
        player_pos.position[0] += x_move;
    }
    if rays[1] >= min_dist && y_move > 0.0 || rays[3] >= min_dist && y_move < 0.0
    {
        player_pos.position[1] += y_move;
    }

    if keys.contains_key(&glutin::event::VirtualKeyCode::Left) { player_pos.dir += look_speed }
    if keys.contains_key(&glutin::event::VirtualKeyCode::Right) { player_pos.dir -= look_speed }
}

fn load_texture(file_path: &str, display: &Display) -> Texture2d
{
    let main_wall_texture_image = image::open(file_path).unwrap().to_rgba();
    let main_wall_texture_image_dimensions = main_wall_texture_image.dimensions();
    let main_wall_texture_image = glium::texture::RawImage2d::from_raw_rgba_reversed(&main_wall_texture_image.into_raw(), main_wall_texture_image_dimensions);

    return Texture2d::new(display, main_wall_texture_image).unwrap();
}

fn main() {
    let draw_3d = std::env::args().nth(1).unwrap_or_else(|| String::from("3d")).to_lowercase() != "2d";

    let event_loop = glutin::event_loop::EventLoop::new();
    let wb = glutin::window::WindowBuilder::new()
        .with_title("Ray Trace Game");
    let cb = glutin::ContextBuilder::new().with_vsync(false);
    let display = glium::Display::new(wb, cb, &event_loop).unwrap();

    // load textures
    let main_wall_texture = load_texture(r"textures\stone.jpg", &display);
    let wall2_texture = load_texture(r"textures\brick.png", &display);
    let wall3_texture = load_texture(r"textures\mossy.jpg", &display);
    let empty_tex = Texture2d::empty(&display, 1,1).unwrap();

    let vertex_shader_src = r#"
        #version 140
        in vec2 position;
        in vec2 tex_coords;
        out vec2 v_tex_coords;
        void main() {
            v_tex_coords = tex_coords;
            gl_Position = vec4(position, 0.0, 1.0);
        }
    "#;

    let fragment_shader_src = r#"
        #version 140
        in vec2 v_tex_coords;
        out vec4 color;
        uniform vec3 rgb_color;
        uniform bool use_texture;
        uniform sampler2D tex;
        uniform float mult;
        void main() {
            if(use_texture) {
                color = texture(tex, v_tex_coords) * mult * 0.5;
            } else {
                color = vec4(rgb_color, 1.0) * mult;
            }
        }
    "#;

    let program = glium::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None).unwrap();

    let mut player_pos = START_POS;

    let mut keys_down = HashMap::<glutin::event::VirtualKeyCode, glutin::event::VirtualKeyCode>::new();

    let mut start = time::Instant::now();

    event_loop.run(move |event, _, control_flow|
    {
        let frame_time = start.elapsed().as_secs_f32();
        start = time::Instant::now();
        let next_frame_time = time::Instant::now() + time::Duration::from_nanos(33_333_333); // 60fps
        *control_flow = glutin::event_loop::ControlFlow::WaitUntil(next_frame_time);

        match event 
        {
            glutin::event::Event::WindowEvent { event, .. } => match event
            {
                glutin::event::WindowEvent::CloseRequested =>
                {
                    *control_flow = glutin::event_loop::ControlFlow::Exit;
                    return;
                },
                _ => return,
            },
            glutin::event::Event::DeviceEvent {event, ..} => 
            {
                if let glutin::event::DeviceEvent::Key(key) = event
                {
                    if let Some(letter) = key.virtual_keycode  
                    { 
                        if glutin::event::ElementState::Pressed == key.state
                        { 
                            keys_down.entry(letter).or_insert(letter);
                        }
                        else
                        {
                            let _ = keys_down.remove(&letter);
                        }
                    }
                }
            },
            _ => (),
        }
        move_player(&keys_down, &mut player_pos, frame_time);
        main_loop(&display, &program, &player_pos, draw_3d, &main_wall_texture, &wall2_texture, 
            &wall3_texture, &empty_tex);
    });
}