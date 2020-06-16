use std::time;
use std::collections::HashMap;
//#[macro_use]
use glium::{glutin, Surface, Display, Program, Frame};
use std::f32;

static GAME_HEIGHT: usize = 8;
static GAME_WIDTH: usize = 8;
static GAME: [u8;64] = [
    1,1,1,1,1,1,1,1,
    1,0,1,0,0,0,0,0,
    1,0,1,0,1,1,0,1,
    1,0,0,0,1,0,0,1,
    1,1,2,0,1,0,1,1,
    1,0,0,0,1,0,0,1,
    1,0,1,0,1,1,0,1,
    1,1,1,1,1,1,1,1,
];

static MOVE_SPEED: f32 = 1.0/3.0;
static LOOK_SPEED: f32 = 2.0;

static RAYS: usize = 60;

static START_POS: PlayerPos = PlayerPos { position: [0.35, 0.2], dir: 1.0 };

#[derive(Copy, Clone)]
struct Vertex 
{
    position: [f32; 2],
}
glium::implement_vertex!(Vertex, position);

#[derive(Copy, Clone)]
struct PlayerPos
{
    position: [f32; 2],
    dir: f32
}

fn draw_rect(top_left: Vertex, bottom_right: Vertex, color: (f32,f32,f32), target: &mut Frame, display: &Display, program: &Program)
{
    // from two triangles
    let vertex1 = top_left.clone();
    let vertex2 = Vertex { position: [ bottom_right.position[0],  top_left.position[1]] };
    let vertex3 = bottom_right.clone();
    let vertex4 = Vertex { position: [ top_left.position[0], bottom_right.position[1]] };
    let shape1 = vec![vertex1, vertex2, vertex4];
    let shape2 = vec![vertex2, vertex3, vertex4];
    // upload shape data to video memory
    let shape1_vb = glium::VertexBuffer::new(display, &shape1).unwrap();
    let shape2_vb = glium::VertexBuffer::new(display, &shape2).unwrap();
    let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);
    
    let uniforms = glium::uniform! {
        rgb_color: color
    };

    target.draw(&shape1_vb, &indices, program, &uniforms, &Default::default()).unwrap();
    target.draw(&shape2_vb, &indices, program, &uniforms, &Default::default()).unwrap();
}

fn draw_line(v1: Vertex, v2: Vertex, color: (f32,f32,f32), target: &mut Frame, display: &Display, program: &Program)
{
    let line = vec![v1, v2];
    let line_vb = glium::VertexBuffer::new(display, &line).unwrap();
    let indices = glium::index::NoIndices(glium::index::PrimitiveType::LinesList);

    let uniforms = glium::uniform! {
        rgb_color: color
    };
    
    target.draw(&line_vb, &indices, program, &uniforms, &Default::default()).unwrap();
}

fn at_wall(pos: (f32, f32), horz: bool) -> u8
{
    let pos2 = ((pos.0 + 1.0) * (GAME_WIDTH as f32) / 2.0, (pos.1 + 1.0) * (GAME_WIDTH as f32) / 2.0);
    let (col, row) = (f32::floor(pos2.0) as usize, f32::floor(pos2.1) as usize);
    if row as usize * GAME_WIDTH + col < GAME_WIDTH * GAME_HEIGHT
    {
        if horz
        {
            let v1 = GAME[row as usize * GAME_WIDTH + col as usize];
            let v2 = if row == 0 {v1} else { GAME[(row - 1) as usize * GAME_WIDTH + col as usize] };
            
            u8::max(v1,v2)
        }
        else
        {
            let v1 = GAME[row as usize * GAME_WIDTH + col as usize];
            let v2 = if col == 0 {v1} else { GAME[row as usize * GAME_WIDTH + col - 1 as usize] };

            u8::max(v1,v2)
        }
    }
    else
    {
        0
    }
}

fn calc_dist_to_wall(player_pos: &PlayerPos, angle: f32) -> (f32, bool, u8)
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

        while ray_x >= -1.0 && ray_x <= 1.0 && ray_y >= -1.0 && ray_x <= 1.0 && f32::sin(angle) != 0.0
        {
            horz_wall = at_wall((ray_x, ray_y), true);
            if horz_wall > 0
            {
                dist_to_horz = f32::sqrt(f32::powf(ray_y - player_pos.position[1], 2.0) + f32::powf(ray_x - player_pos.position[0], 2.0));
                break;
            }
            ray_y += yoffset;
            ray_x += xoffset;
        }
    }
    
    
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

        while ray_x >= -1.0 && ray_x <= 1.0 && ray_y >= -1.0 && ray_x <= 1.0 && f32::cos(angle) != 0.0
        {
            vert_wall = at_wall((ray_x, ray_y), false);
            if vert_wall > 0
            {
                dist_to_vert = f32::sqrt(f32::powf(ray_y - player_pos.position[1], 2.0) + f32::powf(ray_x - player_pos.position[0], 2.0));
                break;
            }
            ray_y += yoffset;
            ray_x += xoffset;
        }
    }

    if dist_to_horz < dist_to_vert
    {
        (dist_to_horz, true, horz_wall)
    }
    else
    {
        (dist_to_vert, false, vert_wall)
    }

}

fn ray_cast(player_pos: &PlayerPos, rays: usize, fov: f32) -> Vec<(usize, f32, f32, bool, u8)>
{
    (0..rays)
        .map(|i| (i, player_pos.dir - fov/2.0 + i as f32 * fov / (rays as f32)))
        .map(|(i, ray_ang)| {let res = calc_dist_to_wall(player_pos, ray_ang); (i, ray_ang, res.0, res.1, res.2)})
        .collect()
}

fn draw_3d_game(display: &Display, program: &Program, player_pos: &PlayerPos)
{
    let mut target = display.draw();
    target.clear_color(0.0, 0.0, 1.0, 1.0);

    let rays = RAYS;

    for (i, ray_ang, ray_dist, horz, wall) in ray_cast(player_pos, rays, 1.6)
    {
        let dist = ray_dist*f32::cos(f32::abs(ray_ang - player_pos.dir));
        let height = 0.2 / dist;
        //println!("{}, {}", dist, height);

        let tl = Vertex { position: [(rays-i) as f32 * 2.0 / rays as f32 - 1.0, 0.0 + height] };
        let br = Vertex { position: [(rays-i-1) as f32 * 2.0 / rays as f32 - 1.0, 0.0 - height] };
        
        let color = if wall == 2 
        { 
            if horz { (0.0, 0.8, 0.0) } 
            else { (0.0, 1.0, 0.0) }
        } 
        else 
        { 
            if horz { (0.8, 0.0, 0.0) } 
            else { (1.0, 0.0, 0.0) } 
        };

        draw_rect(tl, br, color, &mut target, display, program);
    }

    target.finish().unwrap();
}

fn draw_2d_game(display: &Display, program: &Program, player_pos: &PlayerPos)
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
            let this_tl = Vertex { position: [
                (col as f32 * 2.0 / (GAME_WIDTH as f32)) - 1.0 + padding_w, 
                (row as f32 * 2.0 / (GAME_HEIGHT as f32)) - 1.0 + padding_h
            ] };
            let this_br = Vertex { position: [
                ((col + 1) as f32 * 2.0 / (GAME_WIDTH as f32)) - 1.0 - padding_w, 
                ((row + 1) as f32 * 2.0 / (GAME_HEIGHT as f32)) - 1.0 - padding_h
            ] };

            match tile
            {
                2 => draw_rect(this_tl, this_br, (0.0, 1.0, 0.0), &mut target, display, program),
                1 => draw_rect(this_tl, this_br, (1.0, 0.0, 0.0), &mut target, display, program),
                0 => draw_rect(this_tl, this_br, (0.0, 0.0, 1.0), &mut target, display, program),
                _ => ()
            }
        }
    }

    // draw player
    let player_size = 0.05;
    let player_ver = Vertex {position: player_pos.position.clone() };
    let player_tl = Vertex { position: [player_pos.position[0] - player_size/2.0, player_pos.position[1] - player_size/2.0] };
    let player_br = Vertex { position: [player_pos.position[0] + player_size/2.0, player_pos.position[1] + player_size/2.0] };
    let player_dir = Vertex { position: [player_pos.position[0] + 0.1*f32::cos(player_pos.dir), player_pos.position[1] + 0.1*f32::sin(player_pos.dir)] };
    draw_rect(player_tl, player_br, (0.1, 0.9, 0.1), &mut target, display, program);
    draw_line(player_ver.clone(), player_dir, (1.0,1.0,0.0), &mut target, display, program);

    // draw rays
    for (_, ray_ang, ray_dist, _, wall) in ray_cast(player_pos, RAYS, 1.5)
    {
        let color = if wall == 2 { (0.0, 1.0, 0.0) } else { (1.0, 0.0, 0.0) };
        let ray_dir_ver = Vertex { position: [player_pos.position[0] + ray_dist*f32::cos(ray_ang), player_pos.position[1] + ray_dist*f32::sin(ray_ang)] };
        
        draw_line(player_ver, ray_dir_ver, color, &mut target, display, program);
    }

    target.finish().unwrap();
}

fn main_loop(display: &Display, program: &Program, player_pos: &PlayerPos, draw_3d: bool)
{
    if draw_3d
    {
        draw_3d_game(display, program, player_pos);
    }
    else
    {
        draw_2d_game(display, program, player_pos);
    }
}

fn handle_keys(keys: &HashMap<glutin::event::VirtualKeyCode,glutin::event::VirtualKeyCode>, player_pos: &mut PlayerPos, frame_time: f32)
{
    let move_speed = MOVE_SPEED * frame_time;
    let look_speed = LOOK_SPEED * frame_time;
    if keys.contains_key(&glutin::event::VirtualKeyCode::W)
    {
        player_pos.position[0] += move_speed * f32::cos(player_pos.dir);
        player_pos.position[1] += move_speed * f32::sin(player_pos.dir);
    }
    if keys.contains_key(&glutin::event::VirtualKeyCode::S)
    {
        player_pos.position[0] -= move_speed * f32::cos(player_pos.dir);
        player_pos.position[1] -= move_speed * f32::sin(player_pos.dir);
    }
    if keys.contains_key(&glutin::event::VirtualKeyCode::A) 
    {
        player_pos.position[0] -= move_speed * f32::sin(player_pos.dir);
        player_pos.position[1] += move_speed * f32::cos(player_pos.dir);
    }
    if keys.contains_key(&glutin::event::VirtualKeyCode::D)
    {
        player_pos.position[0] += move_speed * f32::sin(player_pos.dir);
        player_pos.position[1] -= move_speed * f32::cos(player_pos.dir);
    }
    if keys.contains_key(&glutin::event::VirtualKeyCode::Left) { player_pos.dir += look_speed }
    if keys.contains_key(&glutin::event::VirtualKeyCode::Right) { player_pos.dir -= look_speed }
}

fn main() {
    let draw_3d = std::env::args().nth(1).unwrap_or(String::from("3d")).to_lowercase() != "2d";

    let event_loop = glutin::event_loop::EventLoop::new();
    let wb = glutin::window::WindowBuilder::new();
    let cb = glutin::ContextBuilder::new();
    let display = glium::Display::new(wb, cb, &event_loop).unwrap();

    let vertex_shader_src = r#"
        #version 140
        in vec2 position;
        void main() {
            gl_Position = vec4(position, 0.0, 1.0);
        }
    "#;

    let fragment_shader_src = r#"
        #version 140
        out vec4 color;
        uniform vec3 rgb_color;
        void main() {
            color = vec4(rgb_color, 1.0);
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
            glutin::event::Event::DeviceEvent {event, ..} => match event
            {
                glutin::event::DeviceEvent::Key(key) => 
                {
                    if let Some(letter) = key.virtual_keycode  
                    { 
                        if glutin::event::ElementState::Pressed == key.state
                        { 
                            if !keys_down.contains_key(&letter)
                            { let _ = keys_down.insert(letter, letter); }
                        }
                        else
                        { let _ = keys_down.remove(&letter); }
                    }
                },
                _ => ()
            },
            _ => (),
        }
        handle_keys(&keys_down, &mut player_pos, frame_time);
        main_loop(&display, &program, &player_pos, draw_3d);
    });
}