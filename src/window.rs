use glfw::{Context, Glfw, GlfwReceiver, PWindow};
use log::*;
use wgpu::{Surface, SurfaceTargetUnsafe};

use crate::{
    ReadOnlyString,
    graphics::callback::{Proxy, TargetHandle},
};

#[derive(Debug)]
pub struct GlfwWindow {
    glfw: Glfw,
    /// The underlying GLFW window.
    pub window: PWindow,
    pub event_receiver: GlfwReceiver<(f64, glfw::WindowEvent)>,
    pub mouse_pos_proxy: Proxy<(f64, f64)>,
}

impl GlfwWindow {
    pub fn new(width: u32, height: u32, title: &str) -> anyhow::Result<Self> {
        let mut glfw = glfw::init(handle_glfw_error)
            .map_err(|e| anyhow::anyhow!("Failed to initialize GLFW: {}", e))?;

        let (mut window, event_receiver) = glfw
            .create_window(width, height, title, glfw::WindowMode::Windowed)
            .ok_or_else(|| anyhow::anyhow!("Failed to create GLFW window"))?;

        window.set_key_polling(true);
        window.make_current();

        let proxy = Proxy::new();

        let closure_proxy = proxy.clone();
        window.set_cursor_pos_callback(move |_, x, y| {
            let proxy = closure_proxy.clone();
            proxy.invoke((x, y));
        });

        Ok(GlfwWindow {
            glfw,
            mouse_pos_proxy: proxy,
            window,
            event_receiver,
        })
    }

    pub fn should_close(&self) -> bool {
        self.window.should_close()
    }

    pub fn poll_events(&mut self) {
        self.glfw.poll_events();
    }

    /// Gets the current size of the window.
    pub fn size(&self) -> (u32, u32) {
        let (width, height) = self.window.get_size();
        (width as u32, height as u32)
    }

    /// Sets the mouse cursor mode.
    pub fn set_mouse_mode(&self, mode: glfw::CursorMode) {
        // TODO: Now that `window` isn't in an Arc, we can call the actual safe method.
        // So this isn't allowed.. but we have a way around it.
        // Since glfw is a c library, we can just call the function directly.
        // This is fine since a. GlfwWindow: !Send and b. we know the pointer is valid.
        debug!("Setting mouse mode to {:?}", mode);
        unsafe {
            glfw::ffi::glfwSetInputMode(
                self.window.window_ptr(),
                glfw::ffi::GLFW_CURSOR,
                mode as i32,
            );
        }
    }

    pub fn get_mouse_mode(&self) -> glfw::CursorMode {
        self.window.get_cursor_mode()
    }

    #[must_use = "The returned TargetHandle must be kept alive to keep the callback registered."]
    pub fn register_mouse_pos_callback<F>(
        &self,
        label: Option<impl Into<ReadOnlyString>>,
        callback: F,
    ) -> TargetHandle<(f64, f64)>
    where
        F: FnMut((f64, f64)) + 'static,
    {
        self.mouse_pos_proxy
            .add_target(callback, label.map(|l| l.into()))
    }

    /// Toggles the mouse cursor mode between Normal and Disabled.
    pub fn toggle_mouse_mode(&self) {
        let current_mode = self.get_mouse_mode();
        let new_mode = match current_mode {
            glfw::CursorMode::Normal => glfw::CursorMode::Disabled,
            _ => glfw::CursorMode::Normal,
        };
        self.set_mouse_mode(new_mode);
    }

    /// Creates a WGPU surface for this window.
    /// The caller must ensure that the GlfwWindow outlives the Surface.
    pub unsafe fn create_surface(&self, instance: &wgpu::Instance) -> Surface<'static> {
        unsafe {
            instance
                .create_surface_unsafe(
                    SurfaceTargetUnsafe::from_window(&self.window)
                        .expect("failed to create surface"),
                )
                .expect("failed to create surface")
        }
    }
}

fn handle_glfw_error(error: glfw::Error, description: String) {
    error!("GLFW error {:?}: {}", error, description);
}
