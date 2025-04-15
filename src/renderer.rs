use sdl2;
use gl;

pub struct Renderer {
  sdl_context: sdl2::Sdl,
  video_subsystem: sdl2::VideoSubsystem,
  window: sdl2::video::Window,
  gl_context: sdl2::video::GLContext,
}

impl Renderer {
  pub fn new() -> Self {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

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

    Self {
      sdl_context,
      video_subsystem,
      window,
      gl_context,
    }

  }
}
