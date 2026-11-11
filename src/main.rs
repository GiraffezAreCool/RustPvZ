/*
Hello it is GiraffezAreCool. This is my attempt to make PvZ in rust and Vulkan
This is mt first time using rust and vulkan so please help
Why am i doing this you may ask? cause i'd be fun idk.

NO GEN AI
uhhh yea
*/

mod renderer;
use renderer::Engine;
use winit;
use winit::application::ApplicationHandler;
#[derive(Default)]
struct App {
    engine: Option<Engine>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.engine = Some(Engine::new(event_loop).unwrap())
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let Some(engine) = self.engine.as_mut() {
            engine.window_event(event_loop, window_id, event);
        }
    }
}

fn main() {
    let mut app = App::default();

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    event_loop.run_app(&mut app).unwrap();
}
