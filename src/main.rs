mod shader_types;
mod state;

use std::sync::Arc;

use glam::Vec3;
use shader_types::{Material, RawSceneComponents};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

use crate::state::State;

#[derive(Default)]
struct App {
    state: Option<State>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );

        let mut raw_scene = RawSceneComponents::default();

        raw_scene
            .insert_obj(
                "assets/sphere.obj",
                Material {
                    albedo: Vec3::new(66.0, 135.0, 245.0) / 255.0,
                    ..Default::default()
                },
            )
            .expect("The sphere obj should exist");
        raw_scene
            .insert_obj(
                "assets/cube.obj",
                Material {
                    emissive: Vec3::new(1.0, 1.0, 1.0),
                    emissive_strength: 3.0,
                    ..Default::default()
                },
            )
            .expect("The cube obj should exist");
        raw_scene
            .insert_obj(
                "assets/cube.obj",
                Material {
                    albedo: Vec3::new(127.0, 127.0, 127.0) / 255.0,
                    ..Default::default()
                },
            )
            .expect("The cube obj should exist");

        let state = pollster::block_on(State::new(window.clone(), &raw_scene));
        self.state = Some(state);

        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let state = self.state.as_mut().unwrap();
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                state.render();
                state.get_window().request_redraw();
            }
            WindowEvent::Resized(size) => {
                state.resize(size);
            }
            _ => (),
        }
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
