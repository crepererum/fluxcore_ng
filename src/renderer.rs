extern crate glium;
extern crate log;

use cfg;

use glium::{DisplayBuild, Surface};
use glium::backend::Facade;
use glium::backend::glutin_backend::GlutinFacade;
use glium::glutin;

use data;
use data::{Column, Point};

use res;

use std::thread;
use std::time::{Duration, Instant};


#[derive(Clone, Copy)]
struct TextureVertex {
    position:   [f32; 2],
    tex_coords: [f32; 2],
}

implement_vertex!(Point, position);
implement_vertex!(TextureVertex, position, tex_coords);


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
        let factor_x = cfg::SCROLL_BASE.powf(dx);
        self.scale_x = f32::max(cfg::SCALE_MIN, self.scale_x * factor_x);
        self.delta_x += (scale_x_old - self.scale_x) * (posx_relative - self.delta_x) / scale_x_old;
    }

    fn scroll_y(&mut self, dy: f32, posy: u32, height: u32) {
        let posy_relative = -(2.0 * (posy as f32) / (height as f32) - 1.0);
        let scale_y_old = self.scale_y;
        let factor_y = cfg::SCROLL_BASE.powf(dy);
        self.scale_y = f32::max(cfg::SCALE_MIN, self.scale_y * factor_y);
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

pub struct Renderer {
    width: u32,
    height: u32,
    columns: Vec<Column>,
    column_x: usize,
    column_y: usize,
    column_z: usize,
    n: usize,
    m: usize,
    display: Box<GlutinFacade>,
    gamma: f32,
    pointsize: f32,
    showborder: bool,
    projection: Projection,
    mouse_x: u32,
    mouse_y: u32,
    mouse_down: bool,
    last_frame: Instant,
    redraw: bool,
    lowres: bool,
    lowres_start: Instant,
    vertex_buffer_points: glium::VertexBuffer<Point>,
    vertex_buffer_texture: glium::VertexBuffer<TextureVertex>,
    indices_points: glium::index::NoIndices,
    indices_texture: glium::index::NoIndices,
    texture_lowres: glium::Texture2d,
    texture_std: glium::Texture2d,
    program_points: glium::Program,
    program_texture: glium::Program,
}

impl Renderer {
    pub fn new(width: u32, height: u32, columns: Vec<Column>, fname: String) -> Renderer {
        info!("set up OpenGL stuff");

        let m = columns.len();
        let column_x: usize = 0;
        let column_y: usize = 1;
        let column_z: usize = if m > 2 { 2 } else { 1 };
        let points = data::points_from_columns(&columns, column_x, column_y, column_z);

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
            .with_title(format!("fluxcore_ng - {}", fname))
            .build_glium()
            .unwrap();

        let source_code_points = glium::program::ProgramCreationInput::SourceCode {
            fragment_shader: res::FRAGMENT_SHADER_POINTS_SRC,
            geometry_shader: None,
            outputs_srgb: false,
            tessellation_control_shader: None,
            tessellation_evaluation_shader: None,
            transform_feedback_varyings: None,
            uses_point_size: true,
            vertex_shader: res::VERTEX_SHADER_POINTS_SRC,
        };

        let mut projection = Projection::new();
        projection.adjust_x(columns[column_x].min, columns[column_x].max);
        projection.adjust_y(columns[column_y].min, columns[column_y].max);
        projection.adjust_z(columns[column_z].min, columns[column_z].max);


        let vertex_buffer_points  = glium::VertexBuffer::new(&display, &points).unwrap();
        let vertex_buffer_texture = glium::VertexBuffer::new(&display, &vertices_texture).unwrap();
        let texture_std           = build_renderable_texture(&display, width, height);
        let texture_lowres        = build_renderable_texture(&display, ((width as f32) * cfg::LOWRES_FACTOR) as u32, ((height as f32) * cfg::LOWRES_FACTOR) as u32);
        let program_points        = glium::Program::new(&display, source_code_points).unwrap();
        let program_texture       = glium::Program::from_source(&display, res::VERTEX_SHADER_TEXTURE_SRC, res::FRAGMENT_SHADER_TEXTURE_SRC, None).unwrap();

        Renderer {
            width: width,
            height: height,
            columns: columns,
            column_x: column_x,
            column_y: column_y,
            column_z: column_z,
            n: points.len(),
            m: m,
            display: Box::new(display),
            gamma: cfg::GAMMA_DEFAULT,
            pointsize: cfg::POINTSIZE_DEFAULT,
            showborder: cfg::SHOWBORDER_DEFAULT,
            projection: projection,
            mouse_x: 0,
            mouse_y: 0,
            mouse_down: false,
            last_frame: Instant::now(),
            redraw: true,
            lowres: false,
            lowres_start: Instant::now(),
            vertex_buffer_points: vertex_buffer_points,
            vertex_buffer_texture: vertex_buffer_texture,
            indices_points: glium::index::NoIndices(glium::index::PrimitiveType::Points),
            indices_texture: glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
            texture_std: texture_std,
            texture_lowres: texture_lowres,
            program_points: program_points,
            program_texture: program_texture,
        }
    }

    pub fn run_forever(&mut self) {
        info!("starting main loop");
        'mainloop: loop {
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

            // step 1: draw to texture if requested
            if self.redraw {
                self.texture_lowres.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
                self.texture_lowres.as_surface().draw(
                    &self.vertex_buffer_points,
                    &self.indices_points,
                    &self.program_points,
                    &uniform! {
                        matrix: self.projection.get_matrix(),
                        inv_n:     1.0 / (self.n as f32),
                        pointsize: self.pointsize * cfg::LOWRES_FACTOR,
                        showborder: if self.showborder { 1f32 } else { 0f32 },
                    },
                    &params_points
                ).unwrap();

                self.redraw = false;
                self.lowres = true;
                self.lowres_start = Instant::now();
            }
            let lowres_now   = Instant::now();
            let lowres_delta = lowres_now.duration_since(self.lowres_start);
            if self.lowres && lowres_delta > Duration::from_millis(cfg::LOWRES_MILLIS) {
                self.texture_std.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
                self.texture_std.as_surface().draw(
                    &self.vertex_buffer_points,
                    &self.indices_points,
                    &self.program_points,
                    &uniform! {
                        matrix: self.projection.get_matrix(),
                        inv_n:     1.0 / (self.n as f32),
                        pointsize: self.pointsize,
                        showborder: if self.showborder { 1f32 } else { 0f32 },
                    },
                    &params_points
                ).unwrap();
                self.lowres = false;
            }

            // step 2: draw texture to screen
            let mut target = self.display.draw();
            {
                let sampler = glium::uniforms::Sampler::new(if self.lowres { &self.texture_lowres } else {&self.texture_std})
                    .wrap_function(glium::uniforms::SamplerWrapFunction::Clamp);
                target.draw(
                    &self.vertex_buffer_texture,
                    &self.indices_texture,
                    &self.program_texture,
                    &uniform! {
                        inv_gamma: (1.0 / self.gamma) as f32,
                        tex:       sampler,
                    },
                    &Default::default()
                ).unwrap();
            }
            target.finish().unwrap();

            // step 3: handle events
            let mut rebuild_points = false;
            for ev in self.display.poll_events() {
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
                                self.showborder = !self.showborder;
                                self.redraw = true;
                            },
                            glutin::VirtualKeyCode::J => {
                                self.pointsize = f32::min(self.pointsize * cfg::POINTSIZE_CHANGE, cfg::POINTSIZE_MAX);
                                self.redraw = true;
                            },
                            glutin::VirtualKeyCode::K => {
                                self.pointsize = f32::max(self.pointsize / cfg::POINTSIZE_CHANGE, cfg::POINTSIZE_MIN);
                                self.redraw = true;
                            },
                            glutin::VirtualKeyCode::N => {
                                self.gamma = f32::min(self.gamma * cfg::GAMMA_CHANGE, cfg::GAMMA_MAX);
                                self.redraw = true;
                            },
                            glutin::VirtualKeyCode::M => {
                                self.gamma = f32::max(self.gamma / cfg::GAMMA_CHANGE, cfg::GAMMA_MIN);
                                self.redraw = true;
                            },
                            glutin::VirtualKeyCode::R => {
                                self.projection.adjust_x(self.columns[self.column_x].min, self.columns[self.column_x].max);
                                self.projection.adjust_y(self.columns[self.column_y].min, self.columns[self.column_y].max);
                                self.projection.adjust_z(self.columns[self.column_z].min, self.columns[self.column_z].max);
                                self.gamma      = cfg::GAMMA_DEFAULT;
                                self.pointsize  = cfg::POINTSIZE_DEFAULT;
                                self.showborder = cfg::SHOWBORDER_DEFAULT;
                                self.redraw = true;
                            },
                            glutin::VirtualKeyCode::Left => {
                                if self.column_x == 0 {
                                    self.column_x = self.m as usize;
                                }
                                self.column_x -= 1;
                                rebuild_points = true;
                                self.projection.adjust_x(self.columns[self.column_x].min, self.columns[self.column_x].max);
                                self.redraw = true;
                            },
                            glutin::VirtualKeyCode::Right => {
                                self.column_x += 1;
                                if self.column_x >= self.m {
                                    self.column_x = 0;
                                }
                                rebuild_points = true;
                                self.projection.adjust_x(self.columns[self.column_x].min, self.columns[self.column_x].max);
                                self.redraw = true;
                            },
                            glutin::VirtualKeyCode::Up => {
                                if self.column_y == 0 {
                                    self.column_y = self.m as usize;
                                }
                                self.column_y -= 1;
                                rebuild_points = true;
                                self.projection.adjust_y(self.columns[self.column_y].min, self.columns[self.column_y].max);
                                self.redraw = true;
                            },
                            glutin::VirtualKeyCode::Down => {
                                self.column_y += 1;
                                if self.column_y >= self.m {
                                    self.column_y = 0;
                                }
                                rebuild_points = true;
                                self.projection.adjust_y(self.columns[self.column_y].min, self.columns[self.column_y].max);
                                self.redraw = true;
                            },
                            glutin::VirtualKeyCode::PageUp => {
                                if self.column_z == 0 {
                                    self.column_z = self.m as usize;
                                }
                                self.column_z -= 1;
                                rebuild_points = true;
                                self.projection.adjust_z(self.columns[self.column_z].min, self.columns[self.column_z].max);
                                self.redraw = true;
                            },
                            glutin::VirtualKeyCode::PageDown => {
                                self.column_z += 1;
                                if self.column_z >= self.m {
                                    self.column_z = 0;
                                }
                                rebuild_points = true;
                                self.projection.adjust_z(self.columns[self.column_z].min, self.columns[self.column_z].max);
                                self.redraw = true;
                            },
                            _ => ()
                        }
                    },
                    glutin::Event::MouseInput(glutin::ElementState::Pressed, glutin::MouseButton::Left) => {
                        self.mouse_down = true;
                    },
                    glutin::Event::MouseInput(glutin::ElementState::Released, glutin::MouseButton::Left) => {
                        self.mouse_down = false;
                    },
                    glutin::Event::MouseMoved(posx, posy) => {
                        if self.mouse_down {
                            let dx = posx - (self.mouse_x as i32);
                            let dy = posy - (self.mouse_y as i32);
                            self.projection.move_x(dx, self.width);
                            self.projection.move_y(dy, self.height);
                            self.redraw = true;
                        }
                        self.mouse_x = posx as u32;
                        self.mouse_y = posy as u32;
                    },
                    glutin::Event::MouseWheel(glutin::MouseScrollDelta::LineDelta(dx, dy), glutin::TouchPhase::Moved) => {
                        self.projection.scroll_x(dx, self.mouse_x, self.width);
                        self.projection.scroll_y(dy, self.mouse_y, self.height);
                        self.redraw = true;
                    },
                    glutin::Event::Resized(w, h) => {
                        self.width  = w;
                        self.height = h;
                        self.texture_std    = build_renderable_texture(&*self.display, self.width, self.height);
                        self.texture_lowres = build_renderable_texture(&*self.display, ((self.width as f32) * cfg::LOWRES_FACTOR) as u32, ((self.height as f32) * cfg::LOWRES_FACTOR) as u32);
                        self.redraw = true;
                    },
                    _ => ()
                }
            }

            // step 4: update geometry
            if rebuild_points {
                let points                = data::points_from_columns(&self.columns, self.column_x, self.column_y, self.column_z);
                self.vertex_buffer_points = glium::VertexBuffer::new(&*self.display, &points).unwrap();
            }

            // step 5: throttle FPS
            let this_frame    = Instant::now();
            let frame_delta   = this_frame.duration_since(self.last_frame);
            let desired_delta = Duration::from_millis(cfg::FRAME_MILLIS);
            if frame_delta < desired_delta {
                thread::sleep(desired_delta - frame_delta);
            }

            self.last_frame = Instant::now();
        }
    }
}
