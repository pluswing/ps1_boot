use std::ffi::CString;

use sdl2;
use gl;
use gl::types::{GLshort, GLubyte, GLuint, GLint, GLenum, GLsizei};
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
    panic!("Attribure \"{:?}\" not found in program", attr);
  }
  index as GLuint
}

pub fn find_program_uniform(program: GLuint, uniform: &str) -> GLint {
  let cstr = CString::new(uniform).unwrap();
  let index = unsafe {
    gl::GetUniformLocation(program, cstr.as_ptr())
  };
  if index < 0 {
    panic!("Uniform \"{:?}\" not found in program", uniform);
  }
  index as GLint
}

/*
pub fn check_for_errors() {
  let mut fatal = false;
  loop {
    let mut buffer = vec![0; 4096];

    let mut severity = 0;
    let mut source = 0;
    let mut message_size = 0;
    let mut mtype = 0;
    let mut id = 0;

    let count = unsafe {
      gl::GetDebugMessageLog(1, buffer.len() as GLsizei, &mut source, &mut mtype, &mut id, &mut severity, &mut message_size, buffer.as_mut_ptr() as * mut GLchar)
    };
    if count == 0 {
      break;
    }

    buffer.truncate(message_size as usize);
    let message = match str::from_utf8(&buffer) {
      Ok(m) => m,
      Err(e) => panic!("Go invalid message: {}", e)
    };
    let source = DebugSource::from_raw(source);
    let sevirity = DebugSrverity::from_raw(severity);
    let mtype = DebugType::from_raw(mtype);

    if severity.is_fatal() {
      fatal = true;
    }
  }
  if fatal {
    panic!("Fatal OpenGL error");
  }
}
*/

pub struct Renderer {
  video_subsystem: sdl2::VideoSubsystem,
  window: sdl2::video::Window,
  gl_context: sdl2::video::GLContext,

  vertex_shader: GLuint,
  fragment_shader: GLuint,
  program: GLuint,
  positions: Vec<Position>,
  colors: Vec<Color>,
  nvertices: u32,

  uniform_offset: GLint,
}

impl Renderer {
  pub fn new(video_subsystem: sdl2::VideoSubsystem) -> Self {
    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
    gl_attr.set_context_version(3, 3);
    gl_attr.set_context_flags().debug().set();

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

    let positions = vec![Position(0, 0); VERTEX_BUFFER_LEN as usize];
    let colors = vec![Color(0, 0, 0); VERTEX_BUFFER_LEN as usize];

    let uniform_offset = find_program_uniform(program, "offset");
    unsafe {
      gl::Uniform2i(uniform_offset, 0, 0);
    }

    Self {
      video_subsystem,
      window,
      gl_context,
      vertex_shader,
      fragment_shader,
      program,
      positions,
      colors,
      nvertices: 0,
      uniform_offset,
    }
  }

  pub fn push_triangle(&mut self, positions: [Position; 3], colors: [Color; 3]) {
    if self.nvertices + 3 > VERTEX_BUFFER_LEN {
      println!("Vertex attrivute buffers full, forcing draw");
      self.draw();
    }
    for i in 0..3 {
      self.positions[self.nvertices as usize] = positions[i];
      self.colors[self.nvertices as usize] = colors[i];
      self.nvertices = self.nvertices + 1;
    }
  }

  pub fn push_quad(&mut self, positions: [Position; 4], colors: [Color; 4]) {
    if self.nvertices + 6 > VERTEX_BUFFER_LEN {
      self.draw();
    }

    for i in 0..3 {
      self.positions[self.nvertices as usize] = positions[i];
      self.colors[self.nvertices as usize] = colors[i];
      self.nvertices = self.nvertices + 1;
    }

    for i in 1..4 {
      self.positions[self.nvertices as usize] = positions[i];
      self.colors[self.nvertices as usize] = colors[i];
      self.nvertices = self.nvertices + 1;
    }
  }

  pub fn draw(&mut self) {
    let mut position_vbo: gl::types::GLuint = 0;
    unsafe {
        gl::GenBuffers(1, &mut position_vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, position_vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,                                                       // target
            (self.nvertices as usize * std::mem::size_of::<Position>()) as gl::types::GLsizeiptr, // size of data in bytes
            self.positions.as_ptr() as *const gl::types::GLvoid, // pointer to data
            gl::STATIC_DRAW,                               // usage
        );
        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
    }

    let mut color_vbo: gl::types::GLuint = 0;
    unsafe {
        gl::GenBuffers(1, &mut color_vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, color_vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,                                                       // target
            (self.nvertices as usize * std::mem::size_of::<Color>()) as gl::types::GLsizeiptr, // size of data in bytes
            self.colors.as_ptr() as *const gl::types::GLvoid, // pointer to data
            gl::STATIC_DRAW,                               // usage
        );
        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
    }

    let mut vao = 0;
    unsafe {
      gl::GenVertexArrays(1, &mut vao);
      gl::BindVertexArray(vao);
    }

    unsafe {
      gl::BindBuffer(gl::ARRAY_BUFFER, position_vbo);
      let index = find_program_attrib(self.program, "vertex_position");
      gl::EnableVertexAttribArray(index);
      gl::VertexAttribIPointer(index, 2, gl::SHORT, 0, ptr::null());
    }

    unsafe {
      gl::BindBuffer(gl::ARRAY_BUFFER, color_vbo);
      let index = find_program_attrib(self.program, "vertex_color");
      gl::EnableVertexAttribArray(index);
      gl::VertexAttribIPointer(index, 3, gl::UNSIGNED_BYTE, 0, ptr::null());
    }

    unsafe {
      gl::BindBuffer(gl::ARRAY_BUFFER, 0);
      gl::BindVertexArray(0);
    }

    unsafe {
      gl::UseProgram(self.program);
    }

    unsafe {
      gl::BindVertexArray(vao);
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

  pub fn set_draw_offset(&mut self, x: i16, y: i16) {
    self.draw();
    unsafe {
      gl::Uniform2i(self.uniform_offset, x as GLint, y as GLint);
    }
  }

  pub fn display(&mut self) {
    self.draw();
    self.window.gl_swap_window();
  }
}

impl Drop for Renderer {
  fn drop(&mut self) {
      unsafe {
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

const VERTEX_BUFFER_LEN: u32 = 64 * 1024;
