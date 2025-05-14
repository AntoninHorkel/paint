#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

mod gui;
mod helpers;
mod renderer;
use std::sync::Arc;

use gui::Gui;
use helpers::{Action, Position, Size, abs_max};
use renderer::{CopyDirection, Renderer};
use ultraviolet::{Vec2, Vec4};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalPosition},
    event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, KeyCode, NamedKey, PhysicalKey},
    window::{Window, WindowId},
};

#[derive(Default)]
enum State {
    #[default]
    Init,
    AddPoints,
    EditPoints,
}

#[derive(Default)]
struct App {
    renderer: Option<Renderer>,
    gui: Option<Gui>,
    absolute_position: Position<f32>,
    position: Position<f32>,
    state: State,
    grabbed_point_idx: Option<usize>,
    grab_position: Option<Position<f32>>,
    grab_offset: Option<Vec2>,
}

// TODO: Clean-up and remove unwrap().
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("Paint")
            .with_inner_size(LogicalSize::new(1366, 768))
            .with_min_inner_size(LogicalSize::new(320, 240))
            .with_position(PhysicalPosition::new(0, 0))
            .with_maximized(true);
        let window = Arc::new(event_loop.create_window(window_attributes).expect("Failed to create window"));
        let renderer = pollster::block_on(Renderer::new(window.clone(), Size::new(1500, 1000)))
            .expect("Failed to create rendering backend");
        let gui = Gui::new(event_loop, &renderer);
        self.renderer = Some(renderer);
        self.gui = Some(gui);
        // TODO: Clean-up.
        #[allow(clippy::unwrap_used)]
        let renderer = self.renderer.as_mut().unwrap();
        #[allow(clippy::unwrap_used)]
        let gui = self.gui.as_mut().unwrap();
        renderer.storage_buffer_object_changed = true;
        renderer.compute_uniform_buffer_object.color = Vec4::one();
        renderer.compute_uniform_buffer_object.action = Action::Init as u32;
        renderer.compute_uniform_buffer_object.stroke = 10.0;
        renderer.compute_uniform_buffer_object.anti_aliasing_scale =
            if gui.anti_aliasing { gui.anti_aliasing_scale * 0.01 } else { 0.0 };
        renderer.compute_uniform_buffer_object.dash_length = if gui.dashed { gui.dash_length } else { 0.0 };
        renderer.compute_uniform_buffer_object.gap_length = if gui.dashed { gui.gap_length } else { 0.0 };
        renderer.compute_uniform_buffer_object_changed = true;
        renderer.draw();
        renderer.copy_texture(CopyDirection::FrontToBack);
        renderer.compute_uniform_buffer_object.color = Vec4::from(gui.color.to_normalized_gamma_f32());
        renderer.compute_uniform_buffer_object.action = gui.action as u32;
        renderer.compute_uniform_buffer_object_changed = true;
        renderer.draw(); // TODO: Why is this required?
        renderer.scale_texture(gui.zoom);
        renderer.vertex_uniform_buffer_object.offset = Vec2::new(gui.offset.x, gui.offset.y) * 0.01;
        renderer.vertex_uniform_buffer_object_changed = true;
        renderer.fragment_uniform_buffer_object.grid_scale = Vec2::zero(); // TODO: Change when GUI widget gets added.
        renderer.fragment_uniform_buffer_object.action = gui.action as u32;
        renderer.fragment_uniform_buffer_object.preview = u32::from(gui.preview);
        renderer.fragment_uniform_buffer_object_changed = true;
        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        #[allow(clippy::unwrap_used)]
        let renderer = self.renderer.as_mut().unwrap();
        #[allow(clippy::unwrap_used)]
        let gui = self.gui.as_mut().unwrap();
        gui.handle_event(&renderer.window, &event);
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                gui.prepare(renderer); // TODO: Is it necessaty to call this every redraw request event?
                renderer
                    .render_with(|encoder, current_texture_view, renderer| {
                        gui.render(encoder, current_texture_view, renderer);
                    })
                    .expect("Failed to render");
            }
            WindowEvent::Resized(size) => {
                renderer.resize_window(size.into(), gui.zoom, gui.offset);
                gui.resize(size.into());
            }
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                // TODO: Implement a trait to convert Position<T> to Position<U>.
                self.absolute_position = Position::new(position.x as f32, position.y as f32);
                self.position = renderer.cursor_absolute_to_relative(self.absolute_position);
                if let Some(grab_position) = self.grab_position {
                    if let Some(grab_offset) = self.grab_offset {
                        // TODO: Simplify
                        let x = if renderer.vertex_uniform_buffer_object.scale.x
                            < renderer.vertex_uniform_buffer_object.scale.y
                        {
                            renderer.texture_size.width
                        } else {
                            renderer.texture_size.height
                        } as f32;
                        renderer.vertex_uniform_buffer_object.offset = grab_offset;
                        renderer.vertex_uniform_buffer_object.offset.x += (self.absolute_position.x - grab_position.x)
                            * 0.001
                            / (renderer.texture_size.width as f32 / x)
                            / renderer.vertex_uniform_buffer_object.scale.x;
                        renderer.vertex_uniform_buffer_object.offset.y += (grab_position.y - self.absolute_position.y)
                            * 0.001
                            / (renderer.texture_size.height as f32 / x)
                            / renderer.vertex_uniform_buffer_object.scale.y;
                        renderer.vertex_uniform_buffer_object_changed = true;
                        gui.offset.x = renderer.vertex_uniform_buffer_object.offset.x * 100.0;
                        gui.offset.y = renderer.vertex_uniform_buffer_object.offset.y * 100.0;
                        renderer.window.request_redraw();
                    }
                }
                match self.state {
                    State::AddPoints => {
                        if !gui.using_cursor {
                            renderer.storage_buffer_object.points.pop();
                            renderer.storage_buffer_object.points.push(Vec2::new(self.position.x, self.position.y));
                            renderer.storage_buffer_object_changed = true;
                            if gui.preview {
                                renderer.copy_texture(CopyDirection::BackToFront);
                                renderer.draw();
                            }
                            renderer.window.request_redraw();
                        }
                    }
                    State::EditPoints => {
                        if let Some(grabbed_point_idx) = self.grabbed_point_idx {
                            if !gui.using_cursor {
                                renderer.storage_buffer_object.points[grabbed_point_idx] =
                                    Vec2::new(self.position.x, self.position.y);
                                renderer.storage_buffer_object_changed = true;
                                if gui.preview {
                                    renderer.copy_texture(CopyDirection::BackToFront);
                                    renderer.draw();
                                }
                                renderer.window.request_redraw();
                            }
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                match button {
                    MouseButton::Left => {
                        if state == ElementState::Pressed {
                            if gui.using_cursor {
                                return;
                            }
                            if gui.action == Action::Fill {
                                // TODO: Implement a trait to convert Position<T> to Position<U>.
                                renderer.fill(
                                    Position::new(self.position.x as u32, self.position.y as u32),
                                    gui.color.to_array(),
                                );
                                renderer.copy_texture(CopyDirection::FrontToBack);
                                renderer.window.request_redraw();
                                return;
                            }
                            match self.state {
                                State::Init => {
                                    renderer.storage_buffer_object.points.clear();
                                    renderer
                                        .storage_buffer_object
                                        .points
                                        .push(Vec2::new(self.position.x, self.position.y));
                                    renderer
                                        .storage_buffer_object
                                        .points
                                        .push(Vec2::new(self.position.x, self.position.y));
                                    renderer.storage_buffer_object.length = 2;
                                    renderer.storage_buffer_object_changed = true;
                                    self.state = State::AddPoints;
                                }
                                State::AddPoints => match gui.action {
                                    Action::DrawLine
                                    | Action::DrawRectangle
                                    | Action::DrawCircle
                                    | Action::DrawEllipse
                                    | Action::CutRectangle => {
                                        self.state = State::EditPoints;
                                    }
                                    Action::Erase => {
                                        renderer.storage_buffer_object.points.swap_remove(0);
                                        renderer
                                            .storage_buffer_object
                                            .points
                                            .push(Vec2::new(self.position.x, self.position.y));
                                        renderer.storage_buffer_object_changed = true;
                                    }
                                    Action::DrawPolygon => {
                                        renderer
                                            .storage_buffer_object
                                            .points
                                            .push(Vec2::new(self.position.x, self.position.y));
                                        renderer.storage_buffer_object.length += 1;
                                        renderer.storage_buffer_object_changed = true;
                                    }
                                    _ => {}
                                },
                                State::EditPoints => match self.grabbed_point_idx {
                                    Some(_) => self.grabbed_point_idx = None,
                                    None => {
                                        self.grabbed_point_idx = renderer
                                            .storage_buffer_object
                                            .points
                                            .iter()
                                            .enumerate()
                                            .filter_map(|(idx, point)| {
                                                let dx = (point.x - self.position.x).abs();
                                                let dy = (point.y - self.position.y).abs();
                                                if dx < gui.point_grab_tolerance && dy < gui.point_grab_tolerance {
                                                    Some((idx, dx + dy))
                                                } else {
                                                    None
                                                }
                                            })
                                            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                                            .map(|(idx, _)| idx);
                                    }
                                },
                            }
                            // renderer.window.request_redraw();
                        }
                    }
                    MouseButton::Right => match state {
                        ElementState::Pressed => {
                            self.grab_position = if gui.using_cursor { None } else { Some(self.absolute_position) };
                            self.grab_offset = if gui.using_cursor {
                                None
                            } else {
                                Some(renderer.vertex_uniform_buffer_object.offset)
                            };
                        }
                        ElementState::Released => {
                            self.grab_position = None;
                            self.grab_offset = None;
                        }
                    },
                    _ => {}
                }
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: false,
            } => match event {
                KeyEvent {
                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                    logical_key: Key::Named(NamedKey::Escape),
                    text: _,
                    location: _,
                    state: ElementState::Pressed,
                    repeat: false,
                    ..
                } => {
                    renderer.storage_buffer_object.points.clear();
                    renderer.storage_buffer_object.length = 0;
                    renderer.storage_buffer_object_changed = true;
                    renderer.copy_texture(CopyDirection::BackToFront);
                    renderer.draw();
                    renderer.copy_texture(CopyDirection::FrontToBack);
                    renderer.window.request_redraw();
                    self.state = State::Init;
                }
                KeyEvent {
                    physical_key: PhysicalKey::Code(KeyCode::Enter),
                    logical_key: Key::Named(NamedKey::Enter),
                    text: _,
                    location: _,
                    state: ElementState::Pressed,
                    repeat: false,
                    ..
                } => match gui.action {
                    Action::DrawLine
                    | Action::DrawRectangle
                    | Action::DrawCircle
                    | Action::DrawEllipse
                    | Action::CutRectangle => match self.state {
                        State::AddPoints | State::EditPoints => {
                            renderer.draw();
                            renderer.copy_texture(CopyDirection::FrontToBack);
                            renderer.storage_buffer_object.points.clear();
                            renderer.storage_buffer_object.length = 0;
                            renderer.storage_buffer_object_changed = true;
                            renderer.window.request_redraw();
                            self.state = State::Init;
                        }
                        _ => {}
                    },
                    Action::DrawPolygon => match self.state {
                        State::AddPoints => self.state = State::EditPoints,
                        State::EditPoints => {
                            renderer.draw();
                            renderer.copy_texture(CopyDirection::FrontToBack);
                            renderer.storage_buffer_object.points.clear();
                            renderer.storage_buffer_object.length = 0;
                            renderer.storage_buffer_object_changed = true;
                            renderer.window.request_redraw();
                            self.state = State::Init;
                        }
                        _ => {}
                    },
                    _ => {}
                },
                KeyEvent {
                    physical_key: PhysicalKey::Code(KeyCode::Delete),
                    logical_key: Key::Named(NamedKey::Delete),
                    text: _,
                    location: _,
                    state: ElementState::Pressed,
                    repeat: false,
                    ..
                } => match self.state {
                    State::AddPoints | State::EditPoints => match gui.action {
                        Action::DrawLine
                        | Action::DrawRectangle
                        | Action::DrawCircle
                        | Action::DrawEllipse
                        | Action::CutRectangle => {
                            self.grabbed_point_idx = None;
                            renderer.storage_buffer_object.points.clear();
                            renderer.storage_buffer_object.length = 0;
                            renderer.storage_buffer_object_changed = true;
                            renderer.copy_texture(CopyDirection::BackToFront);
                            renderer.draw();
                            renderer.copy_texture(CopyDirection::FrontToBack);
                            renderer.window.request_redraw();
                            self.state = State::Init;
                        }
                        Action::DrawPolygon => {
                            if let Some(grabbed_point_idx) = self.grabbed_point_idx {
                                if grabbed_point_idx == 2 {
                                    self.grabbed_point_idx = None;
                                    renderer.storage_buffer_object.points.clear();
                                    renderer.storage_buffer_object.length = 0;
                                    renderer.storage_buffer_object_changed = true;
                                    renderer.copy_texture(CopyDirection::BackToFront);
                                    renderer.draw();
                                    renderer.copy_texture(CopyDirection::FrontToBack);
                                    renderer.window.request_redraw();
                                    self.state = State::Init;
                                } else {
                                    self.grabbed_point_idx = Some(grabbed_point_idx - 1);
                                    renderer.storage_buffer_object.points.remove(grabbed_point_idx);
                                    renderer.storage_buffer_object.length -= 1;
                                    renderer.storage_buffer_object_changed = true;
                                    if gui.preview {
                                        renderer.copy_texture(CopyDirection::BackToFront);
                                        renderer.draw();
                                    }
                                    renderer.window.request_redraw();
                                }
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                },
                _ => {}
            },
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                inner_size_writer: _,
            } => gui.scale_factor(scale_factor),
            WindowEvent::MouseWheel {
                device_id: _,
                delta,
                phase: TouchPhase::Moved,
            } => {
                if !gui.using_cursor {
                    #[allow(clippy::cast_possible_truncation)]
                    {
                        gui.zoom += match delta {
                            MouseScrollDelta::LineDelta(x, y) => abs_max(x, y),
                            MouseScrollDelta::PixelDelta(PhysicalPosition {
                                x,
                                y,
                            }) => abs_max(x, y) as f32,
                        } * gui.zoom_speed
                            * 0.01;
                    }
                    gui.zoom = gui.zoom.max(1.0);
                    renderer.scale_texture(gui.zoom);
                    renderer.vertex_uniform_buffer_object_changed = true;
                    self.position = renderer.cursor_absolute_to_relative(self.absolute_position);
                    if matches!(self.state, State::AddPoints) && !gui.using_cursor {
                        renderer.storage_buffer_object.points.pop();
                        renderer.storage_buffer_object.points.push(Vec2::new(self.position.x, self.position.y));
                        renderer.storage_buffer_object_changed = true;
                        if gui.preview {
                            renderer.copy_texture(CopyDirection::BackToFront);
                            renderer.draw();
                        }
                    }
                    renderer.window.request_redraw();
                }
            }
            WindowEvent::PanGesture {
                device_id: _,
                delta,
                phase: TouchPhase::Moved,
            } => {
                if !gui.using_cursor {
                    gui.zoom += abs_max(delta.x, delta.y) * gui.zoom_speed * 0.01;
                    gui.zoom = gui.zoom.max(1.0);
                    renderer.scale_texture(gui.zoom);
                    renderer.vertex_uniform_buffer_object_changed = true;
                    self.position = renderer.cursor_absolute_to_relative(self.absolute_position);
                    if matches!(self.state, State::AddPoints) && !gui.using_cursor {
                        renderer.storage_buffer_object.points.pop();
                        renderer.storage_buffer_object.points.push(Vec2::new(self.position.x, self.position.y));
                        renderer.storage_buffer_object_changed = true;
                        if gui.preview {
                            renderer.copy_texture(CopyDirection::BackToFront);
                            renderer.draw();
                        }
                    }
                    renderer.window.request_redraw();
                }
            }
            WindowEvent::PinchGesture {
                device_id: _,
                delta,
                phase: TouchPhase::Moved,
            } => {
                if !gui.using_cursor {
                    #[allow(clippy::cast_possible_truncation)]
                    {
                        gui.zoom += delta as f32 * gui.zoom_speed * 0.01;
                    }
                    gui.zoom = gui.zoom.max(1.0);
                    renderer.scale_texture(gui.zoom);
                    renderer.vertex_uniform_buffer_object_changed = true;
                    self.position = renderer.cursor_absolute_to_relative(self.absolute_position);
                    if matches!(self.state, State::AddPoints) && !gui.using_cursor {
                        renderer.storage_buffer_object.points.pop();
                        renderer.storage_buffer_object.points.push(Vec2::new(self.position.x, self.position.y));
                        renderer.storage_buffer_object_changed = true;
                        if gui.preview {
                            renderer.copy_texture(CopyDirection::BackToFront);
                            renderer.draw();
                        }
                    }
                    renderer.window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().expect("Failed to build EventLoop");
    // event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::default();
    event_loop.run_app(&mut app).expect("Failed to run app");
}
