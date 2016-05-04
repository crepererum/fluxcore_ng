extern crate clap;
extern crate csv;
extern crate env_logger;
#[macro_use] extern crate glium;
#[macro_use] extern crate log;

use clap::{Arg, App};
use glium::{DisplayBuild, Surface};
use glium::glutin;
use std::f32;


#[derive(Clone, Copy)]
struct Point {
    position: [f32; 2],
}
implement_vertex!(Point, position);


static VERTEX_SHADER_SRC:   &'static str = include_str!("../res/shader.points.vertex.glsl");
static FRAGMENT_SHADER_SRC: &'static str = include_str!("../res/shader.points.fragment.glsl");

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
    let points = points_from_columns(&columns, 0, 1);

    info!("set up OpenGL stuff");
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

    let vertex_buffer = glium::VertexBuffer::new(&display, &points).unwrap();
    let indices = glium::index::NoIndices(glium::index::PrimitiveType::Points);

    let source_code = glium::program::ProgramCreationInput::SourceCode {
        fragment_shader: FRAGMENT_SHADER_SRC,
        geometry_shader: None,
        outputs_srgb: false,
        tessellation_control_shader: None,
        tessellation_evaluation_shader: None,
        transform_feedback_varyings: None,
        uses_point_size: true,
        vertex_shader: VERTEX_SHADER_SRC,
    };
    let program = glium::Program::new(&display, source_code).unwrap();

    let params = glium::DrawParameters {
        blend: glium::Blend::alpha_blending(),
        .. Default::default()
    };

    let mut pointsize: f32 = POINTSIZE_DEFAULT;

    info!("starting main loop");
    'mainloop: loop {
        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);
        target.draw(
            &vertex_buffer,
            &indices,
            &program,
            &uniform! {
                pointsize: pointsize,
            },
            &params
        ).unwrap();
        target.finish().unwrap();

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
                        _ => ()
                    }
                },
                glutin::Event::Resized(w, h) => {
                    width = w;
                    height = h;
                }
                _ => ()
            }
        }
    }

    info!("shutting down");
}
