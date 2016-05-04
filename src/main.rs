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


#[derive(Clone, Copy)]
struct Point {
    position: [f32; 2],
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

static GAMMA_CHANGE:      f32 = 1.1;
static GAMMA_DEFAULT:     f32 = 10.0;
static GAMMA_MIN:         f32 = 1.0;
static GAMMA_MAX:         f32 = 100.0;
static POINTSIZE_CHANGE:  f32 = 1.1;
static POINTSIZE_DEFAULT: f32 = 10.0;
static POINTSIZE_MIN:     f32 = 2.0;
static POINTSIZE_MAX:     f32 = 30.0;


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
            let value = match cell.parse::<f32>() {
                Ok(v) => v,
                Err(_) => {
                    return Err(format!("cannot parse column {} in row {}", j + 1, i + 1));
                }
            };
            columns[j].push(value);
        }
    }

    Ok(columns)
}

fn points_from_columns(cols: &Vec<Column>, a: usize, b: usize) -> Vec<Point> {
    cols[a].data.iter().zip(cols[b].data.iter()).map(|(x, y)| {
        Point {
            position: [x.clone(), y.clone()]
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
    delta_x: f32,
    delta_y: f32,
}

impl Projection {
    fn new() -> Projection {
        Projection {
            scale_x: 1.0,
            scale_y: 1.0,
            delta_x: 0.0,
            delta_y: 0.0,
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

    fn get_matrix(&self) -> [[f32; 4]; 4] {
        [
            [self.scale_x, 0.0         , 0.0, 0.0],
            [0.0         , self.scale_y, 0.0, 0.0],
            [0.0         , 0.0         , 1.0, 0.0],
            [self.delta_x, self.delta_y, 0.0, 1.0],
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
    let column_x: usize = 0;
    let column_y: usize = 1;
    let points = points_from_columns(&columns, column_x, column_y);
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
        .with_title(format!("fluxcore_ng"))
        .build_glium()
        .unwrap();
    let mut texture = build_renderable_texture(&display, width, height);

    let vertex_buffer_points  = glium::VertexBuffer::new(&display, &points).unwrap();
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

    let mut gamma:     f32 = GAMMA_DEFAULT;
    let mut pointsize: f32 = POINTSIZE_DEFAULT;
    let mut projection = Projection::new();
    projection.adjust_x(columns[column_x].min, columns[column_x].max);
    projection.adjust_y(columns[column_y].min, columns[column_y].max);

    info!("starting main loop");
    'mainloop: loop {
        // step 1: draw to texture
        texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
        texture.as_surface().draw(
            &vertex_buffer_points,
            &indices_points,
            &program_points,
            &uniform! {
                matrix: projection.get_matrix(),
                n:         n,
                pointsize: pointsize,
            },
            &params_points
        ).unwrap();

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
                        glutin::VirtualKeyCode::J => {
                            pointsize = f32::min(pointsize * POINTSIZE_CHANGE, POINTSIZE_MAX);
                        },
                        glutin::VirtualKeyCode::K => {
                            pointsize = f32::max(pointsize / POINTSIZE_CHANGE, POINTSIZE_MIN);
                        },
                        glutin::VirtualKeyCode::N => {
                            gamma = f32::min(gamma * GAMMA_CHANGE, GAMMA_MAX);
                        },
                        glutin::VirtualKeyCode::M => {
                            gamma = f32::max(gamma / GAMMA_CHANGE, GAMMA_MIN);
                        },
                        _ => ()
                    }
                },
                glutin::Event::Resized(w, h) => {
                    width = w;
                    height = h;
                    texture = build_renderable_texture(&display, width, height);
                }
                _ => ()
            }
        }
    }

    info!("shutting down");
}
