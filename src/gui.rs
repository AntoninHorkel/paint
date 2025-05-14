use std::{clone::Clone, mem};

use egui::{
    Button,
    ClippedPrimitive,
    Color32,
    Context,
    DragValue,
    Image,
    ImageButton,
    RichText,
    ScrollArea,
    SidePanel,
    Sides,
    TexturesDelta,
    TopBottomPanel,
    Window as WindowWidget,
    color_picker::{self, Alpha},
    menu,
    viewport::ViewportId,
};
use egui_wgpu::{Renderer as EguiRenderer, ScreenDescriptor};
use egui_winit::State;
use ultraviolet::{Vec2, Vec4};
use wgpu::{CommandEncoder, LoadOp, Operations, RenderPassColorAttachment, RenderPassDescriptor, StoreOp, TextureView};
use winit::{event::WindowEvent, event_loop::ActiveEventLoop, window::Window};

use crate::{
    helpers::{Action, Position, Size},
    renderer::{CopyDirection, Renderer},
};

#[allow(clippy::struct_excessive_bools)]
pub struct Gui {
    context: Context,
    scale_factor: f32,
    state: State,
    screen_descriptor: ScreenDescriptor,
    egui_renderer: EguiRenderer,
    paint_jobs: Vec<ClippedPrimitive>,
    textures: TexturesDelta,
    pub using_cursor: bool,
    pub color: Color32,
    pub anti_aliasing: bool,
    pub anti_aliasing_scale: f32,
    pub dashed: bool,
    pub dash_length: f32,
    pub gap_length: f32,
    pub action: Action,
    pub zoom: f32,
    pub zoom_speed: f32,
    pub offset: Position<f32>,
    pub preview: bool,
    pub point_grab_tolerance: f32,
    side_panel_open: bool,
    settings_open: bool,
    enable_advanced_settings: bool,
}

impl Gui {
    pub fn new(event_loop: &ActiveEventLoop, renderer: &Renderer) -> Self {
        let context = Context::default();
        context.set_zoom_factor(1.1);
        egui_extras::install_image_loaders(&context);
        let scale_factor = renderer.window.scale_factor() as f32;
        let state = State::new(
            context.clone(),
            ViewportId::ROOT,
            event_loop,
            Some(scale_factor),
            renderer.window.theme(),
            Some(renderer.device.limits().max_texture_dimension_2d as usize),
        );
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [renderer.window_size.width, renderer.window_size.height],
            pixels_per_point: scale_factor * context.zoom_factor(),
        };
        let egui_renderer = EguiRenderer::new(&renderer.device, renderer.texture_format, None, 1, false);
        let textures = TexturesDelta::default();
        Self {
            context,
            scale_factor,
            state,
            screen_descriptor,
            egui_renderer,
            paint_jobs: vec![],
            textures,
            using_cursor: false,
            color: Color32::BLACK,
            anti_aliasing: true,
            anti_aliasing_scale: 10.0,
            dashed: false,
            dash_length: 50.0,
            gap_length: 25.0,
            action: Action::DrawLine,
            zoom: 80.0,
            zoom_speed: 100.0,
            offset: Position::new(0.0, 0.0),
            preview: true,
            point_grab_tolerance: 10.0,
            side_panel_open: true,
            settings_open: false,
            enable_advanced_settings: false,
        }
    }

    pub fn prepare(&mut self, renderer: &mut Renderer) {
        let mut options = self.context.options(Clone::clone);
        let output = self.context.run(self.state.take_egui_input(&renderer.window), |context| {
            TopBottomPanel::top("bar").show(context, |ui| {
                menu::bar(ui, |ui| {
                    ui.horizontal_centered(|ui| {
                        if ui
                            .add(Button::image_and_text(egui::include_image!("icons/settings.svg"), "UI Settings"))
                            .clicked()
                        {
                            self.settings_open = true;
                        }
                        ui.separator();
                        for color in [
                            Color32::BLACK,
                            Color32::WHITE,
                            Color32::RED,
                            Color32::GREEN,
                            Color32::BLUE,
                            Color32::YELLOW,
                            Color32::PURPLE,
                        ] {
                            if ui
                                .add(
                                    Button::new("")
                                        .fill(color)
                                        .min_size(egui::vec2(30.0, 10.0))
                                        .selected(self.color == color),
                                )
                                .clicked()
                                && self.color != color
                            {
                                self.color = color;
                                if self.action != Action::Erase {
                                    renderer.compute_uniform_buffer_object.color =
                                        Vec4::from(self.color.to_normalized_gamma_f32());
                                    renderer.compute_uniform_buffer_object_changed = true;
                                }
                            }
                        }
                        ui.separator();
                        ui.label("Stroke");
                        if ui
                            .add(
                                DragValue::new(&mut renderer.compute_uniform_buffer_object.stroke)
                                    .suffix("px")
                                    .range(1.0..=f32::NAN)
                                    .speed(0.25),
                            )
                            .changed()
                        {
                            renderer.compute_uniform_buffer_object_changed = true;
                        }
                        ui.separator();
                        if ui.checkbox(&mut self.anti_aliasing, "Anti-aliasing").changed() {
                            if self.anti_aliasing {
                                if (renderer.compute_uniform_buffer_object.anti_aliasing_scale
                                    - self.anti_aliasing_scale)
                                    .abs()
                                    > f32::EPSILON
                                {
                                    renderer.compute_uniform_buffer_object.anti_aliasing_scale =
                                        self.anti_aliasing_scale * 0.01;
                                    renderer.compute_uniform_buffer_object_changed = true;
                                }
                            } else {
                                renderer.compute_uniform_buffer_object.anti_aliasing_scale = 0.0;
                                renderer.compute_uniform_buffer_object_changed = true;
                            }
                        }
                        ui.separator();
                        if ui.checkbox(&mut self.dashed, "Dashed").changed() {
                            if self.dashed {
                                if (renderer.compute_uniform_buffer_object.dash_length - self.dash_length).abs()
                                    > f32::EPSILON
                                {
                                    renderer.compute_uniform_buffer_object.dash_length = self.dash_length;
                                    renderer.compute_uniform_buffer_object_changed = true;
                                }
                                if (renderer.compute_uniform_buffer_object.gap_length - self.gap_length).abs()
                                    > f32::EPSILON
                                {
                                    renderer.compute_uniform_buffer_object.gap_length = self.gap_length;
                                    renderer.compute_uniform_buffer_object_changed = true;
                                }
                            } else {
                                renderer.compute_uniform_buffer_object.dash_length = 0.0;
                                renderer.compute_uniform_buffer_object.gap_length = 0.0;
                                renderer.compute_uniform_buffer_object_changed = true;
                            }
                        }
                        ui.separator();
                        ui.label("Zoom");
                        if ui.add(DragValue::new(&mut self.zoom).suffix("%").range(1.0..=f32::NAN).speed(1.0)).changed()
                        {
                            // TODO: Move grabbed point.
                            renderer.scale_texture(self.zoom);
                            renderer.vertex_uniform_buffer_object_changed = true;
                            renderer.window.request_redraw();
                        }
                        ui.separator();
                        ui.label("Offset");
                        ui.add_space(12.0);
                        ui.label("x:");
                        let offset_x_changed =
                            ui.add(DragValue::new(&mut self.offset.x).suffix("%").speed(1.0)).changed();
                        ui.add_space(6.0);
                        ui.label("y:");
                        let offset_y_changed =
                            ui.add(DragValue::new(&mut self.offset.y).suffix("%").speed(1.0)).changed();
                        if offset_x_changed || offset_y_changed {
                            // TODO: Move grabbed point.
                            // TODO: Trait.
                            renderer.vertex_uniform_buffer_object.offset =
                                Vec2::new(self.offset.x, self.offset.y) * 0.01;
                            renderer.vertex_uniform_buffer_object_changed = true;
                            renderer.window.request_redraw();
                        }
                        ui.separator();
                        for (action, image, text) in [
                            (Action::DrawLine, egui::include_image!("icons/light_krita_tool_line.svg"), "Draw line"),
                            (
                                Action::DrawRectangle,
                                egui::include_image!("icons/light_krita_tool_rectangle.svg"),
                                "Draw rectangle",
                            ),
                            (
                                Action::DrawCircle,
                                egui::include_image!("icons/light_krita_tool_ellipse.svg"),
                                "Draw circle",
                            ),
                            // (Action::DrawEllipse, egui::include_image!("icons/ellipse.svg"), "Draw ellipse"),
                            (
                                Action::DrawPolygon,
                                egui::include_image!("icons/light_krita_tool_polygon.svg"),
                                "Draw polygon",
                            ),
                            (Action::Erase, egui::include_image!("icons/eraser.svg"), "Erase"),
                            (
                                Action::Fill,
                                egui::include_image!("icons/light_krita_tool_color_fill.svg"),
                                "Fill shapes or areas",
                            ),
                            // (Action::CutRectangle, egui::include_image!("icons/light_tool_rect_selection.svg"), "Cut
                            // rectangle"),
                        ] {
                            if ui
                                .add(ImageButton::new(Image::new(image)).selected(self.action == action))
                                .on_hover_text(text)
                                .clicked()
                            {
                                self.action = action;
                                if self.action == Action::Erase {
                                    renderer.compute_uniform_buffer_object.color = Vec4::one();
                                    renderer.compute_uniform_buffer_object_changed = true;
                                } else {
                                    let color = Vec4::from(self.color.to_normalized_gamma_f32());
                                    if renderer.compute_uniform_buffer_object.color != color {
                                        renderer.compute_uniform_buffer_object.color = color;
                                        renderer.compute_uniform_buffer_object_changed = true;
                                    }
                                }
                                if renderer.compute_uniform_buffer_object.action != action as u32 {
                                    renderer.compute_uniform_buffer_object.action = action as u32;
                                    renderer.compute_uniform_buffer_object_changed = true;
                                }
                                if renderer.fragment_uniform_buffer_object.action != action as u32 {
                                    renderer.fragment_uniform_buffer_object.action = action as u32;
                                    renderer.fragment_uniform_buffer_object_changed = true;
                                }
                            }
                        }
                    });
                });
            });
            SidePanel::left("side panel").resizable(false).show_animated(context, self.side_panel_open, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(5.0);
                    ui.spacing_mut().slider_width = 244.0;
                    if color_picker::color_picker_color32(ui, &mut self.color, Alpha::OnlyBlend)
                        && self.action != Action::Erase
                    {
                        renderer.compute_uniform_buffer_object.color = Vec4::from(self.color.to_normalized_gamma_f32());
                        renderer.compute_uniform_buffer_object_changed = true;
                    }
                    ui.separator();
                    Sides::new().show(
                        ui,
                        |ui| ui.label("Anti-aliasing scale"),
                        |ui| {
                            if ui
                                .add(
                                    DragValue::new(&mut self.anti_aliasing_scale)
                                        .suffix("%")
                                        .range(0.0..=100.0)
                                        .speed(1.0),
                                )
                                .changed()
                                && self.anti_aliasing
                            {
                                renderer.compute_uniform_buffer_object.anti_aliasing_scale =
                                    self.anti_aliasing_scale * 0.01;
                                renderer.compute_uniform_buffer_object_changed = true;
                            }
                        },
                    );
                    ui.separator();
                    Sides::new().show(
                        ui,
                        |ui| ui.label("Dash"),
                        |ui| {
                            if ui
                                .add(
                                    DragValue::new(&mut self.dash_length).suffix("px").range(1.0..=f32::NAN).speed(1.0),
                                )
                                .changed()
                                && self.dashed
                            {
                                renderer.compute_uniform_buffer_object.dash_length = self.dash_length;
                                renderer.compute_uniform_buffer_object_changed = true;
                            }
                        },
                    );
                    Sides::new().show(
                        ui,
                        |ui| ui.label("Gap"),
                        |ui| {
                            if ui
                                .add(DragValue::new(&mut self.gap_length).suffix("px").range(1.0..=f32::NAN).speed(1.0))
                                .changed()
                                && self.dashed
                            {
                                renderer.compute_uniform_buffer_object.gap_length = self.gap_length;
                                renderer.compute_uniform_buffer_object_changed = true;
                            }
                        },
                    );
                    ui.separator();
                    if ui.checkbox(&mut self.preview, "Preview").changed() {
                        renderer.fragment_uniform_buffer_object.preview = u32::from(self.preview);
                        renderer.fragment_uniform_buffer_object_changed = true;
                        if self.preview {
                            renderer.draw();
                        } else {
                            renderer.copy_texture(CopyDirection::BackToFront);
                        }
                        renderer.window.request_redraw();
                    }
                    ui.separator();
                    Sides::new().show(
                        ui,
                        |ui| ui.label("Zoom speed"),
                        |ui| {
                            ui.add(DragValue::new(&mut self.zoom_speed).suffix("%").range(1.0..=f32::NAN).speed(1.0));
                        },
                    );
                    ui.separator();
                    Sides::new().show(
                        ui,
                        |ui| ui.label("Grab tolerance"),
                        |ui| {
                            ui.add(
                                DragValue::new(&mut self.point_grab_tolerance)
                                    .suffix("px")
                                    .range(0.0..=f32::NAN)
                                    .speed(1.0),
                            );
                        },
                    );
                });
            });
            WindowWidget::new("UI Settings").open(&mut self.settings_open).show(context, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.vertical(|ui| {
                            ui.set_min_width(300.0);
                            ui.label(RichText::new("General").heading().size(24.0));
                            let zoom = context.zoom_factor();
                            ui.horizontal(|ui| {
                                ui.label(format!("Zoom: {}%", (zoom * 100.0).round()));
                                if ui
                                    .add(Button::image_and_text(egui::include_image!("icons/zoom_in.svg"), "Zoom-in"))
                                    .clicked()
                                {
                                    context.set_zoom_factor(zoom + 0.05);
                                }
                                if ui
                                    .add(Button::image_and_text(egui::include_image!("icons/zoom_out.svg"), "Zoom-out"))
                                    .clicked()
                                {
                                    context.set_zoom_factor(zoom - 0.05);
                                }
                            });
                            ui.checkbox(&mut self.side_panel_open, "Open side panel");
                            ui.checkbox(&mut self.enable_advanced_settings, "Enable advanced settings");
                            if self.enable_advanced_settings {
                                ui.separator();
                                ui.label(RichText::new("Advanced").heading().size(24.0));
                                options.ui(ui);
                            }
                        });
                    });
                });
            });
        });
        if renderer.compute_uniform_buffer_object_changed {
            renderer.copy_texture(CopyDirection::BackToFront);
            renderer.draw();
            renderer.window.request_redraw();
        }
        options.zoom_factor = self.context.zoom_factor();
        self.context.options_mut(|o| *o = options);
        self.textures.append(output.textures_delta);
        self.state.handle_platform_output(&renderer.window, output.platform_output);
        self.screen_descriptor.pixels_per_point = self.scale_factor * self.context.zoom_factor();
        self.paint_jobs = self.context.tessellate(output.shapes, self.screen_descriptor.pixels_per_point);
    }

    pub fn render(&mut self, encoder: &mut CommandEncoder, current_texture_view: &TextureView, renderer: &Renderer) {
        for (id, image_delta) in &self.textures.set {
            self.egui_renderer.update_texture(&renderer.device, &renderer.queue, *id, image_delta);
        }
        self.egui_renderer.update_buffers(
            &renderer.device,
            &renderer.queue,
            encoder,
            &self.paint_jobs,
            &self.screen_descriptor,
        );
        {
            let mut render_pass = encoder
                .begin_render_pass(&RenderPassDescriptor {
                    label: Some("egui render pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: current_texture_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                })
                .forget_lifetime();
            self.egui_renderer.render(&mut render_pass, &self.paint_jobs, &self.screen_descriptor);
        }
        let textures = mem::take(&mut self.textures);
        for id in &textures.free {
            self.egui_renderer.free_texture(id);
        }
    }

    pub fn handle_event(&mut self, window: &Window, event: &WindowEvent) {
        if self.state.on_window_event(window, event).repaint {
            window.request_redraw();
        }
        // Note that context.is_using_pointer returns `false` if the pointer is just hovering over an egui area.
        self.using_cursor = self.context.wants_pointer_input();
    }

    pub const fn resize(&mut self, size: Size<u32>) {
        if size.width > 0 && size.height > 0 {
            self.screen_descriptor.size_in_pixels = [size.width, size.height];
        }
    }

    pub const fn scale_factor(&mut self, scale_factor: f64) {
        #[allow(clippy::cast_possible_truncation)]
        {
            self.scale_factor = scale_factor as f32;
        }
    }
}
