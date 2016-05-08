extern crate clap;
extern crate csv;
extern crate env_logger;
#[macro_use] extern crate glium;
#[macro_use] extern crate log;

use clap::{Arg, App};
use glium::{DisplayBuild, Surface};
use glium::backend::Facade;
use glium::glutin;
use std::f32;
use std::thread;
use std::time::{Duration, Instant};


#[derive(Clone, Copy)]
struct Point {
    position: [f32; 3],
}
implement_vertex!(Point, position);

#[derive(Clone, Copy)]
struct TextureVertex {
    position:   [f32; 2],
    tex_coords: [f32; 2],
}
implement_vertex!(TextureVertex, position, tex_coords);


static VERTEX_SHADER_POINTS_SRC:    &'static str = include_str!("../res/shader.points.vertex.glsl");
static FRAGMENT_SHADER_POINTS_SRC:  &'static str = include_str!("../res/shader.points.fragment.glsl");
static VERTEX_SHADER_TEXTURE_SRC:   &'static str = include_str!("../res/shader.texture.vertex.glsl");
static FRAGMENT_SHADER_TEXTURE_SRC: &'static str = include_str!("../res/shader.texture.fragment.glsl");

static FRAME_MILLIS:       u64  = 50;
static GAMMA_CHANGE:       f32  = 1.1;
static GAMMA_DEFAULT:      f32  = 10.0;
static GAMMA_MIN:          f32  = 1.0;
static GAMMA_MAX:          f32  = 100.0;
static POINTSIZE_CHANGE:   f32  = 1.1;
static POINTSIZE_DEFAULT:  f32  = 10.0;
static POINTSIZE_MIN:      f32  = 2.0;
static POINTSIZE_MAX:      f32  = 100.0;
static SCALE_MIN:          f32  = 0.00000001;
static SCROLL_BASE:        f32  = 1.1;
static SHOWBORDER_DEFAULT: bool = true;


fn is_uint_and_geq_100(s: String) -> Result<(), String> {
    match s.parse::<u32>() {
        Ok(i) => {
            if i >= 100 {
                Ok(())
            } else {
                Err(String::from("Number has to be at least 100"))
            }
        },
        Err(_) => {
            Err(String::from("Not a positive number"))
        }
    }
}


struct Column {
    name: String,
    data: Vec<f32>,
    min: f32,
    max: f32,
}

impl Column {
    fn new(name: &String) -> Column {
        Column {
            name: name.clone(),
            data: vec![],
            min: f32::INFINITY,
            max: f32::NEG_INFINITY,
        }
    }

    fn push(&mut self, point: f32) {
        self.data.push(point);
        self.min = f32::min(self.min, point);
        self.max = f32::max(self.max, point);
    }
}


fn is_na_string(s: &String) -> bool {
    let lower = s.to_lowercase();
    if lower == "?" {
        true
    } else if lower == "na" {
        true
    } else {
        false
    }
}


fn columns_from_file(fname: &String) -> Result<Vec<Column>, String> {
    let mut rdr = match csv::Reader::from_file(fname) {
        Ok(f) => f.has_headers(true),
        Err(_) => {
            return Err(String::from("cannot open file!"));
        }
    };

    let headers = rdr.headers().unwrap();
    let m = headers.len();
    if m < 2 {
        return Err(String::from("we need at least 2 CSV columns"));
    }

    let mut columns = headers.iter().map(|name| {
        Column::new(name)
    }).collect::<Vec<Column>>();

    for (i, row) in rdr.records().enumerate() {
        let row = row.unwrap();
        if row.len() != m {
            return Err(format!("row {} has {} entries but should have {}", i + 1, row.len(), m));
        }

        for (j, cell) in row.iter().enumerate() {
            let value = if is_na_string(cell) {
                f32::NAN
            } else {
                match cell.parse::<f32>() {
                    Ok(v) => v,
                    Err(_) => {
                        return Err(format!("cannot parse column {} in row {}", j + 1, i + 1));
                    }
                }
            };
            columns[j].push(value);
        }
    }

    Ok(columns)
}

fn points_from_columns(cols: &Vec<Column>, a: usize, b: usize, c: usize) -> Vec<Point> {
    cols[a].data.iter().zip(cols[b].data.iter()).zip(cols[c].data.iter()).map(|((x, y), z)| {
        Point {
            position: [x.clone(), y.clone(), z.clone()]
        }
    }).collect()
}


pub fn build_renderable_texture<F>(facade: &F, width: u32, height: u32) -> glium::Texture2d where F: Facade {
    glium::Texture2d::empty_with_format(
        facade,
        glium::texture::UncompressedFloatFormat::F32F32F32F32,
        glium::texture::MipmapsOption::NoMipmap,
        width,
        height
    ).unwrap()
}


struct Projection {
    scale_x: f32,
    scale_y: f32,
    scale_z: f32,
    delta_x: f32,
    delta_y: f32,
    delta_z: f32,
}

impl Projection {
    fn new() -> Projection {
        Projection {
            scale_x: 1.0,
            scale_y: 1.0,
            scale_z: 1.0,
            delta_x: 0.0,
            delta_y: 0.0,
            delta_z: 0.0,
        }
    }

    fn adjust_x(&mut self, min: f32, max: f32) {
        if max != min {
            self.scale_x = 2.0 / (max - min);
        }
        self.delta_x = -1.0 - min * self.scale_x;
        debug!("adjust x projection: data_range=[{}, {}] scale={} delta={}", min, max, self.scale_x, self.delta_x);
    }

    fn adjust_y(&mut self, min: f32, max: f32) {
        if max != min {
            self.scale_y = 2.0 / (max - min);
        }
        self.delta_y = -1.0 - min * self.scale_y;
        debug!("adjust y projection: data_range=[{}, {}] scale={} delta={}", min, max, self.scale_y, self.delta_y);
    }

    fn adjust_z(&mut self, min: f32, max: f32) {
        if max != min {
            self.scale_z = 1.0 / (max - min);
        }
        self.delta_z = -min * self.scale_z;
        debug!("adjust z projection: data_range=[{}, {}] scale={} delta={}", min, max, self.scale_z, self.delta_z);
    }

    fn move_x(&mut self, dx: i32, width: u32) {
        self.delta_x += 2.0 * (dx as f32) / (width as f32);
    }

    fn move_y(&mut self, dy: i32, height: u32) {
        self.delta_y -= 2.0 * (dy as f32) / (height as f32);
    }

    fn scroll_x(&mut self, dx: f32, posx: u32, width: u32) {
        let posx_relative = 2.0 * (posx as f32) / (width as f32) - 1.0;
        let scale_x_old = self.scale_x;
        let factor_x = SCROLL_BASE.powf(dx);
        self.scale_x = f32::max(SCALE_MIN, self.scale_x * factor_x);
        self.delta_x += (scale_x_old - self.scale_x) * (posx_relative - self.delta_x) / scale_x_old;
    }

    fn scroll_y(&mut self, dy: f32, posy: u32, height: u32) {
        let posy_relative = -(2.0 * (posy as f32) / (height as f32) - 1.0);
        let scale_y_old = self.scale_y;
        let factor_y = SCROLL_BASE.powf(dy);
        self.scale_y = f32::max(SCALE_MIN, self.scale_y * factor_y);
        self.delta_y += (scale_y_old - self.scale_y) * (posy_relative - self.delta_y) / scale_y_old;
    }

    fn get_matrix(&self) -> [[f32; 4]; 4] {
        [
            [self.scale_x, 0.0         , 0.0         , 0.0],
            [0.0         , self.scale_y, 0.0         , 0.0],
            [0.0         , 0.0         , self.scale_z, 0.0],
            [self.delta_x, self.delta_y, self.delta_z, 1.0],
        ]
    }
}


fn main() {
    env_logger::init().unwrap();
    info!("that's fluxcore...booting up!");

    info!("parse command line args");
    let matches = App::new("fluxcore_ng")
        .version("???")
        .author("Marco Neumann")
        .about("fast data renderer")
        .arg(Arg::with_name("width")
             .short("w")
             .long("width")
             .default_value("800")
             .validator(is_uint_and_geq_100))
        .arg(Arg::with_name("height")
             .short("h")
             .long("height")
             .default_value("600")
             .validator(is_uint_and_geq_100))
        .arg(Arg::with_name("file")
             .required(true)
             .index(1)
             .value_name("FILE"))
        .get_matches();
    let mut width = matches.value_of("width").unwrap().parse::<u32>().unwrap();
    let mut height = matches.value_of("height").unwrap().parse::<u32>().unwrap();
    let file = String::from(matches.value_of("file").unwrap());

    info!("read data from file");
    let columns = match columns_from_file(&file) {
        Ok(c) => c,
        Err(s) => {
            error!("{}", s);
            return;
        }
    };
    let m = columns.len();
    let mut column_x: usize = 0;
    let mut column_y: usize = 1;
    let mut column_z: usize = if m > 2 { 2 } else { 1 };
    let mut points = points_from_columns(&columns, column_x, column_y, column_z);
    let n = points.len() as u32;

    info!("set up OpenGL stuff");
    let vertices_texture = vec![
        TextureVertex { position: [-1.0, -1.0], tex_coords: [0.0, 0.0] },
        TextureVertex { position: [ 1.0,  1.0], tex_coords: [1.0, 1.0] },
        TextureVertex { position: [-1.0,  1.0], tex_coords: [0.0, 1.0] },
        TextureVertex { position: [-1.0, -1.0], tex_coords: [0.0, 0.0] },
        TextureVertex { position: [ 1.0, -1.0], tex_coords: [1.0, 0.0] },
        TextureVertex { position: [ 1.0,  1.0], tex_coords: [1.0, 1.0] },
    ];

    let display = glutin::WindowBuilder::new()
        .with_dimensions(width, height)
        .with_gl(glutin::GlRequest::Specific(
            glutin::Api::OpenGl,
            (3, 3)
        ))
        .with_gl_profile(glutin::GlProfile::Core)
        .with_title(format!("fluxcore_ng - {}", file))
        .build_glium()
        .unwrap();
    let mut texture = build_renderable_texture(&display, width, height);

    let mut vertex_buffer_points  = glium::VertexBuffer::new(&display, &points).unwrap();
    let vertex_buffer_texture = glium::VertexBuffer::new(&display, &vertices_texture).unwrap();
    let indices_points  = glium::index::NoIndices(glium::index::PrimitiveType::Points);
    let indices_texture = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

    let source_code_points = glium::program::ProgramCreationInput::SourceCode {
        fragment_shader: FRAGMENT_SHADER_POINTS_SRC,
        geometry_shader: None,
        outputs_srgb: false,
        tessellation_control_shader: None,
        tessellation_evaluation_shader: None,
        transform_feedback_varyings: None,
        uses_point_size: true,
        vertex_shader: VERTEX_SHADER_POINTS_SRC,
    };
    let program_points = glium::Program::new(&display, source_code_points).unwrap();
    let program_texture = glium::Program::from_source(&display, VERTEX_SHADER_TEXTURE_SRC, FRAGMENT_SHADER_TEXTURE_SRC, None).unwrap();

    let params_points = glium::DrawParameters {
        blend: glium::Blend {
            color: glium::BlendingFunction::Addition {
                source: glium::LinearBlendingFactor::One,
                destination: glium::LinearBlendingFactor::One,
            },
            alpha: glium::BlendingFunction::Addition {
                source: glium::LinearBlendingFactor::One,
                destination: glium::LinearBlendingFactor::One
            },
            constant_value: (0.0, 0.0, 0.0, 0.0)
        },
        .. Default::default()
    };

    let mut gamma:      f32  = GAMMA_DEFAULT;
    let mut pointsize:  f32  = POINTSIZE_DEFAULT;
    let mut showborder: bool = SHOWBORDER_DEFAULT;
    let mut projection = Projection::new();
    projection.adjust_x(columns[column_x].min, columns[column_x].max);
    projection.adjust_y(columns[column_y].min, columns[column_y].max);
    projection.adjust_z(columns[column_z].min, columns[column_z].max);

    info!("starting main loop");
    let mut mouse_x: u32 = 0;
    let mut mouse_y: u32 = 0;
    let mut mouse_down   = false;
    let mut last_frame   = Instant::now();
    let mut redraw       = false;
    'mainloop: loop {
        // step 1: draw to texture if requested
        if redraw {
            texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
            texture.as_surface().draw(
                &vertex_buffer_points,
                &indices_points,
                &program_points,
                &uniform! {
                    matrix: projection.get_matrix(),
                    inv_n:     1.0 / (n as f32),
                    pointsize: pointsize,
                    showborder: if showborder { 1f32 } else { 0f32 },
                },
                &params_points
            ).unwrap();

            redraw = false;
        }

        // step 2: draw texture to screen
        let mut target = display.draw();
        target.draw(
            &vertex_buffer_texture,
            &indices_texture,
            &program_texture,
            &uniform! {
                inv_gamma: (1.0 / gamma) as f32,
                tex:       &texture,
            },
            &Default::default()
        ).unwrap();
        target.finish().unwrap();

        // step 3: handle events
        let mut rebuild_points = false;
        for ev in display.poll_events() {
            match ev {
                glutin::Event::Closed => {
                    break 'mainloop;
                },
                glutin::Event::KeyboardInput(glutin::ElementState::Pressed, _, Some(code)) => {
                    match code {
                        glutin::VirtualKeyCode::Escape => {
                            break 'mainloop;
                        }
                        glutin::VirtualKeyCode::B => {
                            showborder = !showborder;
                            redraw = true;
                        },
                        glutin::VirtualKeyCode::J => {
                            pointsize = f32::min(pointsize * POINTSIZE_CHANGE, POINTSIZE_MAX);
                            redraw = true;
                        },
                        glutin::VirtualKeyCode::K => {
                            pointsize = f32::max(pointsize / POINTSIZE_CHANGE, POINTSIZE_MIN);
                            redraw = true;
                        },
                        glutin::VirtualKeyCode::N => {
                            gamma = f32::min(gamma * GAMMA_CHANGE, GAMMA_MAX);
                            redraw = true;
                        },
                        glutin::VirtualKeyCode::M => {
                            gamma = f32::max(gamma / GAMMA_CHANGE, GAMMA_MIN);
                            redraw = true;
                        },
                        glutin::VirtualKeyCode::R => {
                            projection.adjust_x(columns[column_x].min, columns[column_x].max);
                            projection.adjust_y(columns[column_y].min, columns[column_y].max);
                            projection.adjust_z(columns[column_z].min, columns[column_z].max);
                            gamma      = GAMMA_DEFAULT;
                            pointsize  = POINTSIZE_DEFAULT;
                            showborder = SHOWBORDER_DEFAULT;
                            redraw = true;
                        },
                        glutin::VirtualKeyCode::Left => {
                            if column_x == 0 {
                                column_x = m as usize;
                            }
                            column_x -= 1;
                            rebuild_points = true;
                            projection.adjust_x(columns[column_x].min, columns[column_x].max);
                            redraw = true;
                        },
                        glutin::VirtualKeyCode::Right => {
                            column_x += 1;
                            if column_x >= m {
                                column_x = 0;
                            }
                            rebuild_points = true;
                            projection.adjust_x(columns[column_x].min, columns[column_x].max);
                            redraw = true;
                        },
                        glutin::VirtualKeyCode::Up => {
                            if column_y == 0 {
                                column_y = m as usize;
                            }
                            column_y -= 1;
                            rebuild_points = true;
                            projection.adjust_y(columns[column_y].min, columns[column_y].max);
                            redraw = true;
                        },
                        glutin::VirtualKeyCode::Down => {
                            column_y += 1;
                            if column_y >= m {
                                column_y = 0;
                            }
                            rebuild_points = true;
                            projection.adjust_y(columns[column_y].min, columns[column_y].max);
                            redraw = true;
                        },
                        glutin::VirtualKeyCode::PageUp => {
                            if column_z == 0 {
                                column_z = m as usize;
                            }
                            column_z -= 1;
                            rebuild_points = true;
                            projection.adjust_z(columns[column_z].min, columns[column_z].max);
                            redraw = true;
                        },
                        glutin::VirtualKeyCode::PageDown => {
                            column_z += 1;
                            if column_z >= m {
                                column_z = 0;
                            }
                            rebuild_points = true;
                            projection.adjust_z(columns[column_z].min, columns[column_z].max);
                            redraw = true;
                        },
                        _ => ()
                    }
                },
                glutin::Event::MouseInput(glutin::ElementState::Pressed, glutin::MouseButton::Left) => {
                    mouse_down = true;
                },
                glutin::Event::MouseInput(glutin::ElementState::Released, glutin::MouseButton::Left) => {
                    mouse_down = false;
                },
                glutin::Event::MouseMoved(posx, posy) => {
                    if mouse_down {
                        let dx = posx - (mouse_x as i32);
                        let dy = posy - (mouse_y as i32);
                        projection.move_x(dx, width);
                        projection.move_y(dy, height);
                        redraw = true;
                    }
                    mouse_x = posx as u32;
                    mouse_y = posy as u32;
                },
                glutin::Event::MouseWheel(glutin::MouseScrollDelta::LineDelta(dx, dy), glutin::TouchPhase::Moved) => {
                    projection.scroll_x(dx, mouse_x, width);
                    projection.scroll_y(dy, mouse_y, height);
                    redraw = true;
                },
                glutin::Event::Resized(w, h) => {
                    width = w;
                    height = h;
                    texture = build_renderable_texture(&display, width, height);
                    redraw = true;
                },
                _ => ()
            }
        }

        // step 4: update geometry
        if rebuild_points {
            points               = points_from_columns(&columns, column_x, column_y, column_z);
            vertex_buffer_points = glium::VertexBuffer::new(&display, &points).unwrap();
        }

        // step 5: throttle FPS
        let this_frame    = Instant::now();
        let frame_delta   = this_frame.duration_since(last_frame);
        let desired_delta = Duration::from_millis(FRAME_MILLIS);
        if frame_delta < desired_delta {
            thread::sleep(desired_delta - frame_delta);
        }

        last_frame = Instant::now();
    }

    info!("shutting down");
}
