use sdl2;
use sdl2::video::{OPENGL, WindowPos};

pub struct Renderer {
  sdl_context: sdl2::sdl::Sdl,
  window: sdl2::video::Window,
  gl_context: sdl2::video::GLContext,
}

impl Renderer {
  pub fn new() -> Self {
    // TODO
  }
}
