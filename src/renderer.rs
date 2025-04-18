use std::{mem::size_of, slice};

use sdl2;
use gl;
use gl::types::{GLshort, GLubyte, GLuint, GLsizeiptr};
use std::ptr;

pub struct Renderer {
  sdl_context: sdl2::Sdl,
  video_subsystem: sdl2::VideoSubsystem,
  window: sdl2::video::Window,
  gl_context: sdl2::video::GLContext,

  vertex_shader: GLuint,
  fragment_shader: GLuint,
  program: GLuint,
  vertex_array_object: GLuint,
  positions: Buffer<Position>,
  colors: Buffer<Color>,
  nvertices: u32,
}

impl Renderer {
  pub fn new() -> Self {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    // TODO
    // sdl2::video::gl_set_attrbute(GLContextMajorVersion, 3);
    // sdl2::video::gl_set_attrbute(GLContextMinorVersion, 3);

    let window = video_subsystem.window("PSX", 1024, 512)
        .opengl()
        .position_centered()
        .build()
        .unwrap();

    let gl_context = window.gl_create_context().unwrap();

    let _gl = gl::load_with(|s| video_subsystem.gl_get_proc_address(s) as *const std::os::raw::c_void);

    unsafe {
      gl::ClearColor(0., 0., 0., 1.0);
      gl::Clear(gl::COLOR_BUFFER_BIT);
    }

    window.gl_swap_window();

    // main.rsに持っていく。
    let mut event_pump = sdl_context.event_pump().unwrap();
    'main: loop {
        for event in event_pump.poll_iter() {
            match event {
                sdl2::event::Event::Quit {..} => break 'main,
                _ => {},
            }
        }

        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        window.gl_swap_window();
    }

    let vs_src = include_str!("shader/vertex.glsl");
    let fs_src = include_str!("shader/fragment.glsl");

    let vertex_shader = compile_shader(vs_src, gl::VERTEX_SHADER);
    let fragment_shader = compile_shader(fs_src, gl::FRAGMENT_SHADER);

    let program = link_program(&[vertex_shader, fragment_shader]);

    unsafe {
      gl::UseProgram(program);
    }

    let mut vao = 0;
    unsafe {
      gl::GenVertexArrays(1, &mut vao);
      gl::BindVertexArray(vao);
    }

    let positions = Buffer::new();

    unsafe {
      let index = find_program_attrib(program, "vertex_position");
      gl::EnableVertexAttribArray(index);
      gl::VertexAttribIPointer(index, 2, gl::SHORT, 0, ptr::null());
    }

    let colors = Buffer::new();

    unsafe {
      let index = find_program_attrib(program, "vertex_color");
      gl::EnableVertexAttribArray(index);
      gl::VertexAttribIPointer(index, 3, gl::UNSIGNED_BYTE, 0, ptr::null());
    }

    Self {
      sdl_context,
      video_subsystem,
      window,
      gl_context,
      vertex_shader,
      fragment_shader,
      program,
      vertex_array_object: vao,
      positions,
      colors,
      nvertices: 0,
    }
  }

  pub fn push_triangle(&mut self, positions: &[Position], colors: &[Color]) {
    // TODO
  }
}

#[derive(Copy, Clone, Default, Debug)]
pub struct Position(pub GLshort, pub GLshort);

impl Position {
  pub fn from_gp0(val: u32) -> Self {
    let x = val as i16;
    let y = (val >> 16) as i16;

    Self(x as GLshort, y as GLshort)
  }
}

#[derive(Copy, Clone, Default, Debug)]
pub struct Color(pub GLubyte, pub GLubyte, pub GLubyte);

impl Color {
  pub fn from_gp0(val: u32) -> Self {
    let r = val as u8;
    let g = (val >> 8) as u8;
    let b = (val >> 16) as u8;
    Self(r as GLubyte, g as GLubyte, b as GLubyte)
  }
}

pub struct Buffer<T> {
  object: GLuint,
  map: *mut T,
}

impl <T: Copy + Default> Buffer<T> {
  pub fn new() -> Buffer<T> {
    let mut object = 0;
    let mut memory;

    unsafe {
      gl::GenBuffers(1, &mut object);
      gl::BindBuffer(gl::ARRAY_BUFFER, object);
      let element_size = size_of::<T>() as GLsizeiptr;
      let buffer_size = element_size * VERTEX_BUFFER_LEN as GLsizeiptr;
      let access = gl::MAP_WRITE_BIT | gl::MAP_PERSISTENT_BIT;
      gl::BufferStorage(gl::ARRAY_BUFFER, buffer_size, ptr::null(), access);
      memory = gl::MapBufferRange(gl::ARRAY_BUFFER, 0, buffer_size, access) as *mut T;
      let s = slice::from_raw_parts_mut(memory, VERTEX_BUFFER_LEN as usize);
      for x in s.iter_mut() {
        *x = Default::default();
      }
    }
    Self {
      object,
      map: memory,
    }
  }

  pub fn set(&mut self, index: u32, val: T) {
    if index >= VERTEX_BUFFER_LEN {
      panic!("buffer overflow!");
    }

    unsafe {
      let p = self.map.offset(index as isize);
      *p = val;
    }
  }
}

impl<T> Drop for Buffer<T> {
  fn drop(&mut self) {
    unsafe {
      gl::BindBuffer(gl::ARRAY_BUFFER, self.object);
      gl::UnmapBuffer(gl::ARRAY_BUFFER);
      gl::DeleteBuffers(1, &self.object);
    }
  }
}

const VERTEX_BUFFER_LEN: u32 = 64 * 1024;
