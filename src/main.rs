mod camera;
mod dense_storage;
mod material;
mod mesh;
mod mesh_object;
mod scene;
mod shader_types;
mod state;
mod transform;

use std::{sync::Arc, time::Instant};

use glam::Vec3;
use material::Material;
use mesh_object::MeshObject;
use scene::Scene;
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
    last_time: Option<Instant>,
    frame_count: u32,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );

        let mut scene = Scene::default();

        let sphere = scene
            .load_mesh("assets/sphere.obj")
            .expect("The sphere obj should exist");
        let cube = scene
            .load_mesh("assets/cube.obj")
            .expect("The cube obj should exist");
        let blue_mat = scene.insert_material(Material {
            albedo: Vec3::new(66.0, 135.0, 245.0) / 255.0,
            ..Default::default()
        });
        let white_emissive_mat = scene.insert_material(Material {
            emissive: Vec3::new(1.0, 1.0, 1.0),
            emissive_strength: 3.0,
            ..Default::default()
        });
        let gray_mat = scene.insert_material(Material {
            albedo: Vec3::new(127.0, 127.0, 127.0) / 255.0,
            ..Default::default()
        });

        scene.insert_mesh_object(MeshObject {
            mesh: sphere,
            material: blue_mat,
            transform: transform::Transform {
                translation: Vec3::new(1.0, -0.5, -3.0),
                ..Default::default()
            },
        });

        scene.insert_mesh_object(MeshObject {
            mesh: cube,
            material: white_emissive_mat,
            transform: transform::Transform {
                translation: Vec3::new(0.0, 1.5, -3.0),
                ..Default::default()
            },
        });

        scene.insert_mesh_object(MeshObject {
            mesh: cube,
            material: gray_mat,
            transform: transform::Transform {
                translation: Vec3::new(0.0, -1.5, -3.0),
                scale: Vec3::new(10.0, 1.0, 10.0),
                ..Default::default()
            },
        });

        let state = pollster::block_on(State::new(window.clone(), scene));
        self.state = Some(state);

        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(state) = &mut self.state else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                let last_time = self.last_time.get_or_insert_with(Instant::now);
                let elapsed_secs = last_time.elapsed().as_secs_f32();

                if elapsed_secs >= 1.0 {
                    println!("{} fps", self.frame_count as f32 / elapsed_secs);
                    self.last_time = Some(Instant::now());
                    self.frame_count = 0;
                }

                self.frame_count += 1;

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
