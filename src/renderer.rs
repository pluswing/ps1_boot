use std::ffi::CString;
use std::{mem::size_of, slice};

use sdl2;
use gl;
use gl::types::{GLshort, GLubyte, GLuint, GLsizeiptr, GLint, GLenum, GLsizei};
use std::ptr;

pub fn compile_shader(src: &str, shader_type: GLenum) -> GLuint {
  let shader;
  unsafe {
    shader = gl::CreateShader(shader_type);
    let c_str = CString::new(src.as_bytes()).unwrap();
    gl::ShaderSource(shader, 1, &c_str.as_ptr(), ptr::null());
    gl::CompileShader(shader);
    let mut status = gl::FALSE as GLint;
    gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);
    if status != (gl::TRUE as GLint) {
      panic!("Shader compilation failed!");
    }
  }
  shader
}

pub fn link_program(shaders: &[GLuint]) -> GLuint {
  let program;

  unsafe {
    program = gl::CreateProgram();
    for &shader in shaders {
      gl::AttachShader(program, shader);
    }
    gl::LinkProgram(program);
    let mut status = gl::FALSE as GLint;
    gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);
    if status != (gl::TRUE as GLint) {
      panic!("OpenGL program linking failed!");
    }
  }
  program
}

pub fn find_program_attrib(program: GLuint, attr: &str) -> GLuint {
  let cstr = CString::new(attr).unwrap();
  let index = unsafe {
    gl::GetAttribLocation(program, cstr.as_ptr())
  };
  if index < 0 {
    panic!("Attribure \"{:?}\" not found int program", attr);
  }
  index as GLuint
}

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

    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
    gl_attr.set_context_version(3, 3);

    let window = video_subsystem.window("PSX", 1024, 512)
        .opengl()
        .position_centered()
        .build()
        .unwrap();

    let gl_context = window.gl_create_context().unwrap();

    let _gl = gl::load_with(|s| video_subsystem.gl_get_proc_address(s) as *const std::os::raw::c_void);

    unsafe {
      // gl::Viewport(0, 0, 1024, 512);
      gl::ClearColor(0., 0., 0., 1.0);
      gl::Clear(gl::COLOR_BUFFER_BIT);
    }

    window.gl_swap_window();

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

    println!("init args positions");
    let positions = Buffer::<Position>::new();

    unsafe {
      println!("find");
      let index = find_program_attrib(program, "vertex_position");
      println!("found");
      gl::EnableVertexAttribArray(index);
      println!("en");
      gl::VertexAttribIPointer(index, 2, gl::SHORT, 0, ptr::null());
    }

    println!("init args color");
    let colors = Buffer::<Color>::new();

    unsafe {
      let index = find_program_attrib(program, "vertex_color");
      gl::EnableVertexAttribArray(index);
      gl::VertexAttribIPointer(index, 3, gl::UNSIGNED_BYTE, 0, ptr::null());
    }
    println!("done renderer init");

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

  pub fn push_triangle(&mut self, positions: [Position; 3], colors: [Color; 3]) {
    if self.nvertices + 3 > VERTEX_BUFFER_LEN {
      println!("Vertex attrivute buffers full, forcing draw");
      self.draw();
    }
    println!("push triangle");
    for i in 0..3 {
      self.positions.set(self.nvertices, positions[i]);
      self.colors.set(self.nvertices,colors[i]);
      self.nvertices = self.nvertices + 1;
    }
  }

  pub fn draw(&mut self) {
    unsafe {
      println!("barrier");
      // gl::MemoryBarrier(gl::CLIENT_MAPPED_BUFFER_BARRIER_BIT);
      println!("draw arrays {}", self.nvertices);
      gl::DrawArrays(gl::TRIANGLES, 0, self.nvertices as GLsizei);
    }

    unsafe {
      let sync = gl::FenceSync(gl::SYNC_GPU_COMMANDS_COMPLETE, 0);
      loop {
        let r = gl::ClientWaitSync(
          sync,
          gl::SYNC_FLUSH_COMMANDS_BIT,
          10000000
        );
        if r == gl::ALREADY_SIGNALED || r == gl::CONDITION_SATISFIED {
          break;
        }
      }
    }
    self.nvertices = 0;
  }

  pub fn display(&mut self) {
    self.draw();

    let mut event_pump = self.sdl_context.event_pump().unwrap();
    for event in event_pump.poll_iter() {
      match event {
        sdl2::event::Event::Quit {..} => panic!("exit!"),
        _ => {},
      }
    }

    self.window.gl_swap_window();
  }
}

impl Drop for Renderer {
  fn drop(&mut self) {
      unsafe {
        gl::DeleteVertexArrays(1, &self.vertex_array_object);
        gl::DeleteShader(self.vertex_shader);
        gl::DeleteShader(self.fragment_shader);
        gl::DeleteProgram(self.program);
      }
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

    println!("buffer new");
    unsafe {
      gl::GenBuffers(1, &mut object);
      gl::BindBuffer(gl::ARRAY_BUFFER, object);
      println!("element size");
      let element_size = size_of::<T>() as GLsizeiptr;
      let buffer_size = element_size * VERTEX_BUFFER_LEN as GLsizeiptr;
      let access = gl::MAP_WRITE_BIT | gl::MAP_PERSISTENT_BIT;
      println!("storage");
      gl::BufferData(gl::ARRAY_BUFFER, buffer_size, ptr::null(), access);

      println!("map");
      memory = gl::MapBufferRange(gl::ARRAY_BUFFER, 0, buffer_size, access) as *mut T;
      println!("from");
      let s = slice::from_raw_parts_mut(memory, VERTEX_BUFFER_LEN as usize);
      println!("init");
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
      // let p = self.map.offset(index as isize);
      // *p = val;

      let s = slice::from_raw_parts_mut(self.map, VERTEX_BUFFER_LEN as usize);
      let mut i = 0;
      for x in s.iter_mut() {
        if i == index {
          *x = val;
          break;
        }
        i = i + 1;
      }
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
