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


struct WindowDims {
    width: u32,
    height: u32,
}

impl WindowDims {
    fn to_lowres(&self) -> WindowDims {
        WindowDims {
            width: ((self.width as f32) * cfg::LOWRES_FACTOR) as u32,
            height: ((self.height as f32) * cfg::LOWRES_FACTOR) as u32,
        }
    }
}

fn build_renderable_texture<F>(facade: &F, window_dims: &WindowDims) -> glium::Texture2d where F: Facade {
    glium::Texture2d::empty_with_format(
        facade,
        glium::texture::UncompressedFloatFormat::F32F32F32F32,
        glium::texture::MipmapsOption::NoMipmap,
        window_dims.width,
        window_dims.height
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

struct ColumnState {
    x: usize,
    y: usize,
    z: usize,
}

impl ColumnState {
    fn new(m: usize) -> ColumnState {
        ColumnState {
            x: 0,
            y: 1,
            z: if m > 2 { 2 } else { 1 },
        }
    }

    fn x_prev(&mut self, m: usize) {
        if self.x == 0 {
            self.x = m;
        }
        self.x -= 1;
    }

    fn y_prev(&mut self, m: usize) {
        if self.y == 0 {
            self.y = m;
        }
        self.y -= 1;
    }

    fn z_prev(&mut self, m: usize) {
        if self.z == 0 {
            self.z = m;
        }
        self.z -= 1;
    }

    fn x_next(&mut self, m: usize) {
        self.x += 1;
        if self.x >= m {
            self.x = 0;
        }
    }

    fn y_next(&mut self, m: usize) {
        self.y += 1;
        if self.y >= m {
            self.y = 0;
        }
    }

    fn z_next(&mut self, m: usize) {
        self.z += 1;
        if self.z >= m {
            self.z = 0;
        }
    }
}

struct MouseState {
    x: u32,
    y: u32,
    down: bool,
}

impl MouseState {
    fn new() -> MouseState {
        MouseState {
            x: 0,
            y: 0,
            down: false,
        }
    }
}

struct UserState {
    gamma: f32,
    pointsize: f32,
    showborder: bool,
}

impl UserState {
    fn new() -> UserState {
        UserState {
            gamma:      cfg::GAMMA_DEFAULT,
            pointsize:  cfg::POINTSIZE_DEFAULT,
            showborder: cfg::SHOWBORDER_DEFAULT,
        }
    }

    fn reset(&mut self) {
        self.gamma      = cfg::GAMMA_DEFAULT;
        self.pointsize  = cfg::POINTSIZE_DEFAULT;
        self.showborder = cfg::SHOWBORDER_DEFAULT;
    }

    fn showborder_toggle(&mut self) {
        self.showborder = !self.showborder;
    }

    fn pointsize_increase(&mut self) {
        self.pointsize = f32::min(self.pointsize * cfg::POINTSIZE_CHANGE, cfg::POINTSIZE_MAX);
    }

    fn pointsize_decrease(&mut self) {
        self.pointsize = f32::max(self.pointsize / cfg::POINTSIZE_CHANGE, cfg::POINTSIZE_MIN);
    }

    fn gamma_increase(&mut self) {
        self.gamma = f32::min(self.gamma * cfg::GAMMA_CHANGE, cfg::GAMMA_MAX);

    }

    fn gamma_decrease(&mut self) {
        self.gamma = f32::max(self.gamma / cfg::GAMMA_CHANGE, cfg::GAMMA_MIN);
    }
}

pub struct Renderer {
    window_dims: WindowDims,
    columns: Vec<Column>,
    column_state: ColumnState,
    n: usize,
    m: usize,
    display: GlutinFacade,
    user_state: UserState,
    projection: Projection,
    mouse_state: MouseState,
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

        let window_dims = WindowDims{width: width, height: height};

        let m = columns.len();
        let column_state = ColumnState::new(m);
        let points = data::points_from_columns(&columns, column_state.x, column_state.y, column_state.z);

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
        projection.adjust_x(columns[column_state.x].min, columns[column_state.x].max);
        projection.adjust_y(columns[column_state.y].min, columns[column_state.y].max);
        projection.adjust_z(columns[column_state.z].min, columns[column_state.z].max);


        let vertex_buffer_points  = glium::VertexBuffer::new(&display, &points).unwrap();
        let vertex_buffer_texture = glium::VertexBuffer::new(&display, &vertices_texture).unwrap();
        let texture_std           = build_renderable_texture(&display, &window_dims);
        let texture_lowres        = build_renderable_texture(&display, &window_dims.to_lowres());
        let program_points        = glium::Program::new(&display, source_code_points).unwrap();
        let program_texture       = glium::Program::from_source(&display, res::VERTEX_SHADER_TEXTURE_SRC, res::FRAGMENT_SHADER_TEXTURE_SRC, None).unwrap();

        Renderer {
            window_dims: window_dims,
            columns: columns,
            column_state: column_state,
            n: points.len(),
            m: m,
            display: display,
            user_state: UserState::new(),
            projection: projection,
            mouse_state: MouseState::new(),
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
        loop {
            let next = self.run_once();
            if !next {
                break;
            }
        }
    }

    pub fn run_once(&mut self) -> bool {
        self.render_to_textures();
        self.render_to_screen();

        let mut rebuild_points = false;
        let mut exit = false;
        self.handle_events(&mut rebuild_points, &mut exit);
        if exit {
            return false;
        }

        if rebuild_points {
            self.update_geometry();
        }

        self.throttle();

        true
    }

    fn render_to_textures(&mut self) {
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

        if self.redraw {
            self.texture_lowres.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
            self.texture_lowres.as_surface().draw(
                &self.vertex_buffer_points,
                &self.indices_points,
                &self.program_points,
                &uniform! {
                    matrix: self.projection.get_matrix(),
                    inv_n:     1.0 / (self.n as f32),
                    pointsize: self.user_state.pointsize * cfg::LOWRES_FACTOR,
                    showborder: if self.user_state.showborder { 1f32 } else { 0f32 },
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
                    pointsize: self.user_state.pointsize,
                    showborder: if self.user_state.showborder { 1f32 } else { 0f32 },
                },
                &params_points
            ).unwrap();
            self.lowres = false;
        }
    }

    fn render_to_screen(&mut self) {
        let mut target = self.display.draw();
        {
            let sampler = glium::uniforms::Sampler::new(if self.lowres { &self.texture_lowres } else {&self.texture_std})
                .wrap_function(glium::uniforms::SamplerWrapFunction::Clamp);
            target.draw(
                &self.vertex_buffer_texture,
                &self.indices_texture,
                &self.program_texture,
                &uniform! {
                    inv_gamma: (1.0 / self.user_state.gamma) as f32,
                    tex:       sampler,
                },
                &Default::default()
            ).unwrap();
        }
        target.finish().unwrap();
    }

    fn handle_events(&mut self, rebuild_points: &mut bool, exit: &mut bool) {
        for ev in self.display.poll_events() {
            match ev {
                glutin::Event::Closed => {
                    *exit = true;
                    return;
                },
                glutin::Event::KeyboardInput(glutin::ElementState::Pressed, _, Some(code)) => {
                    match code {
                        glutin::VirtualKeyCode::Escape => {
                            *exit = true;
                            return;
                        }
                        glutin::VirtualKeyCode::B => {
                            self.user_state.showborder_toggle();
                            self.redraw = true;
                        },
                        glutin::VirtualKeyCode::J => {
                            self.user_state.pointsize_increase();
                            self.redraw = true;
                        },
                        glutin::VirtualKeyCode::K => {
                            self.user_state.pointsize_decrease();
                            self.redraw = true;
                        },
                        glutin::VirtualKeyCode::N => {
                            self.user_state.gamma_increase();
                            self.redraw = true;
                        },
                        glutin::VirtualKeyCode::M => {
                            self.user_state.gamma_decrease();
                            self.redraw = true;
                        },
                        glutin::VirtualKeyCode::Q => {
                            *exit = true;
                            return;
                        }
                        glutin::VirtualKeyCode::R => {
                            self.projection.adjust_x(self.columns[self.column_state.x].min, self.columns[self.column_state.x].max);
                            self.projection.adjust_y(self.columns[self.column_state.y].min, self.columns[self.column_state.y].max);
                            self.projection.adjust_z(self.columns[self.column_state.z].min, self.columns[self.column_state.z].max);
                            self.user_state.reset();
                            self.redraw = true;
                        },
                        glutin::VirtualKeyCode::Left => {
                            self.column_state.x_prev(self.m);
                            *rebuild_points = true;
                            self.projection.adjust_x(self.columns[self.column_state.x].min, self.columns[self.column_state.x].max);
                            self.redraw = true;
                        },
                        glutin::VirtualKeyCode::Right => {
                            self.column_state.x_next(self.m);
                            *rebuild_points = true;
                            self.projection.adjust_x(self.columns[self.column_state.x].min, self.columns[self.column_state.x].max);
                            self.redraw = true;
                        },
                        glutin::VirtualKeyCode::Up => {
                            self.column_state.y_prev(self.m);
                            *rebuild_points = true;
                            self.projection.adjust_y(self.columns[self.column_state.y].min, self.columns[self.column_state.y].max);
                            self.redraw = true;
                        },
                        glutin::VirtualKeyCode::Down => {
                            self.column_state.y_next(self.m);
                            *rebuild_points = true;
                            self.projection.adjust_y(self.columns[self.column_state.y].min, self.columns[self.column_state.y].max);
                            self.redraw = true;
                        },
                        glutin::VirtualKeyCode::PageUp => {
                            self.column_state.z_prev(self.m);
                            *rebuild_points = true;
                            self.projection.adjust_z(self.columns[self.column_state.z].min, self.columns[self.column_state.z].max);
                            self.redraw = true;
                        },
                        glutin::VirtualKeyCode::PageDown => {
                            self.column_state.z_next(self.m);
                            *rebuild_points = true;
                            self.projection.adjust_z(self.columns[self.column_state.z].min, self.columns[self.column_state.z].max);
                            self.redraw = true;
                        },
                        _ => ()
                    }
                },
                glutin::Event::MouseInput(glutin::ElementState::Pressed, glutin::MouseButton::Left) => {
                    self.mouse_state.down = true;
                },
                glutin::Event::MouseInput(glutin::ElementState::Released, glutin::MouseButton::Left) => {
                    self.mouse_state.down = false;
                },
                glutin::Event::MouseMoved(posx, posy) => {
                    if self.mouse_state.down {
                        let dx = posx - (self.mouse_state.x as i32);
                        let dy = posy - (self.mouse_state.y as i32);
                        self.projection.move_x(dx, self.window_dims.width);
                        self.projection.move_y(dy, self.window_dims.height);
                        self.redraw = true;
                    }
                    self.mouse_state.x = posx as u32;
                    self.mouse_state.y = posy as u32;
                },
                glutin::Event::MouseWheel(glutin::MouseScrollDelta::LineDelta(dx, dy), glutin::TouchPhase::Moved) => {
                    self.projection.scroll_x(dx, self.mouse_state.x, self.window_dims.width);
                    self.projection.scroll_y(dy, self.mouse_state.y, self.window_dims.height);
                    self.redraw = true;
                },
                glutin::Event::Resized(w, h) => {
                    self.window_dims.width = w;
                    self.window_dims.height = h;
                    self.texture_std    = build_renderable_texture(&self.display, &self.window_dims);
                    self.texture_lowres = build_renderable_texture(&self.display, &self.window_dims.to_lowres());
                    self.redraw = true;
                },
                _ => ()
            }
        }
    }

    fn update_geometry(&mut self) {
        let points = data::points_from_columns(
            &self.columns,
            self.column_state.x,
            self.column_state.y,
            self.column_state.z
        );
        self.vertex_buffer_points = glium::VertexBuffer::new(&self.display, &points).unwrap();
    }

    fn throttle(&mut self) {
        let this_frame    = Instant::now();
        let frame_delta   = this_frame.duration_since(self.last_frame);
        let desired_delta = Duration::from_millis(cfg::FRAME_MILLIS);
        if frame_delta < desired_delta {
            thread::sleep(desired_delta - frame_delta);
        }

        self.last_frame = Instant::now();
    }
}
