use crate::{lattice::{Lattice, LatticeType, LatticeInitialState}, spin::Spin};


pub struct IsingApp {
    size: usize,
    fps: f32,
    last_epoch: std::time::Instant,
    lattice_type: LatticeType,
    initial_state: LatticeInitialState,
    lattice: Lattice,
    lattice_texture: Option<egui::TextureHandle>,
    paused: bool,
    file_save_handle: Option<std::thread::JoinHandle<Option<std::path::PathBuf>>>,
    alert: Option<Alert>,
}

enum Alert {
    Success(String),
    Error(String),
}

impl Default for IsingApp {
    fn default() -> Self {
        Self {
            size: 32,
            fps: 10.0,
            last_epoch: std::time::Instant::now(),
            initial_state: LatticeInitialState::Random,
            lattice_type: LatticeType::Ferromagnetic,
            lattice: Lattice::new_random(32, 1.0, 0.0, LatticeType::Ferromagnetic),
            lattice_texture: None,
            paused: false,
            file_save_handle: None,
            alert: None,
        }
    }
}

impl IsingApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals {
            dark_mode: true,
            ..Default::default()
        });

        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert("font_awesome".into(), egui::FontData::from_static(include_bytes!("../fonts/font_awesome_solid.otf")));
        fonts.families
            .entry(egui::FontFamily::Name("icons".into())).or_default()
            .insert(0, "font_awesome".into());

        cc.egui_ctx.set_fonts(fonts);

        Default::default()
    }
}

impl eframe::App for IsingApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        // save image
        if self.file_save_handle.is_some() && self.file_save_handle.as_ref().expect("").is_finished() {
            match self.file_save_handle.take().expect("").join() {
                Ok(path) => if let Some(path) = path {
                    let (data, size) = self.lattice.as_image_raw();
                    let size = size as u32;
                    
                    self.alert = match image::save_buffer_with_format(path, &data, size, size, image::ColorType::Rgb8, image::ImageFormat::Png) {
                        Ok(_) => Some(Alert::Success("Image saved succesfully.".into())),
                        Err(err) => Some(Alert::Error(format!("Failed to save image: {}", err.to_string()))),
                    };
                },
                Err(_) => {
                    self.alert = Some(Alert::Error("Failed to open file save dialogue.".into()));
                },
            }
        }

        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Ising Model");

                ui.add_space(4.0);

                egui::CollapsingHeader::new("Lattice").default_open(true).show(ui, |ui| {
                    ui.label("Size");
                    ui.add(egui::Slider::new(&mut self.size, 1..=256));
                    
                    {
                        let p_antiferro = if let LatticeType::SpinGlass { p_antiferro } = &self.lattice_type {
                            *p_antiferro
                        } else {
                            0.5
                        };

                        ui.radio_value(&mut self.lattice_type, LatticeType::Ferromagnetic, "Ferromagnetic");
                        ui.radio_value(&mut self.lattice_type, LatticeType::Antiferromagnetic, "Antiferromagnetic");
                        ui.radio_value(&mut self.lattice_type, LatticeType::SpinGlass { p_antiferro }, "Spin Glass");

                        if let LatticeType::SpinGlass { p_antiferro } = &mut self.lattice_type {
                            ui.label("p Antiferromagnetic");
                            ui.add(egui::Slider::new(p_antiferro, 0.0..=1.0));
                        }
                    }
                    
                    ui.label("Lattice Initial State");
                    ui.radio_value(&mut self.initial_state, LatticeInitialState::Random, "Random");
                    ui.radio_value(&mut self.initial_state, LatticeInitialState::AllUp, "All Spin Up");
                    ui.radio_value(&mut self.initial_state, LatticeInitialState::AllDown, "All Spin Down");
        
                    if ui.button("Regenerate Lattice").clicked() {
                        self.lattice = match self.initial_state {
                            LatticeInitialState::Random => Lattice::new_random(self.size, self.lattice.temperature, self.lattice.magnetic_field, self.lattice_type),
                            LatticeInitialState::AllUp => Lattice::new_uniform(self.size, self.lattice.temperature, self.lattice.magnetic_field, Spin::Up, self.lattice_type),
                            LatticeInitialState::AllDown => Lattice::new_uniform(self.size, self.lattice.temperature, self.lattice.magnetic_field, Spin::Down, self.lattice_type),
                        };
                    }
                });

                ui.add_space(4.0);
                
                egui::CollapsingHeader::new("Environment").default_open(true).show(ui, |ui| {
                    ui.label("Temperature");
                    ui.add(egui::Slider::new(&mut self.lattice.temperature, 0.0..=10.0));
                    
                    ui.label("Magnetic Field");
                    ui.add(egui::Slider::new(&mut self.lattice.magnetic_field, -5.0..=5.0));
                });

                ui.add_space(4.0);
                
                egui::CollapsingHeader::new("Simulation").default_open(true).show(ui, |ui| {
                    ui.label("Iterations per Second");
                    ui.add(egui::Slider::new(&mut self.fps, 1.0..=60.0));
                    if ui.button("Save Image").clicked() {
                        self.file_save_handle = Some(std::thread::spawn(|| {
                            rfd::FileDialog::new()
                                .add_filter("PNG", &["png"])
                                .set_file_name("lattice.png")
                                .set_title("Save Lattice Image")
                                .save_file()
                        }));
                        self.paused = true;
                    }
                });

                ui.add_space(8.0);
            })
        });
        
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(alert) = &self.alert {
                let mut alert_closed = false;
                
                let (title, text) = match alert {
                    Alert::Success(text) => ("Success", text.as_str()),
                    Alert::Error(text) => ("Error", text.as_str()),
                };
                egui::Window::new(title).collapsible(false).show(ctx, |ui| {
                    ui.label(text);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        if ui.button("OK").clicked() {
                            alert_closed = true;
                        }
                    });
                });

                if alert_closed {
                    self.alert = None;
                }
            }

            ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                let (pause_text, play_text) = {
                    let font_id = egui::FontId::new(14.0, egui::FontFamily::Name("icons".into()));
                    (egui::RichText::new("\u{f04c}").font(font_id.clone()), egui::RichText::new("\u{f04b}").font(font_id))
                };
                
                if ui.add_enabled(!self.paused, egui::Button::new(pause_text).small()).clicked() {
                    self.paused = true;
                }
                if ui.add_enabled(self.paused, egui::Button::new(play_text).small()).clicked() {
                    self.paused = false;
                }

                ui.add_space(8.0);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    ui.label(format!("Magnetisation: {:.4}", self.lattice.magnetisation()));
                    ui.label(format!("Heat capacity: {:.2}", self.lattice.heat_capacity()));
                });
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                ui.label(egui::RichText::new("Spin Down").color(egui::Color32::from_rgb(255, 64, 64)));
                ui.label(egui::RichText::new("Spin Up").color(egui::Color32::from_rgb(96, 96, 255)));
                ui.label("Key:");
            });

            ui.add_space(8.0);

            if !self.paused && std::time::Instant::now() - self.last_epoch > std::time::Duration::from_secs_f32(1.0/self.fps) {
                let start = std::time::Instant::now();
                self.lattice.epoch();
                println!("Epoch time: {:.5}", (std::time::Instant::now() - start).as_secs_f32());
                // force redraw
                self.lattice_texture = None;
                self.last_epoch = std::time::Instant::now();
            }
            
            let available_space = ui.available_size().x.min(ui.available_size().y);
            
            let texture: &egui::TextureHandle = self.lattice_texture.get_or_insert_with(|| {
                let start = std::time::Instant::now();
                let tex = ui.ctx().load_texture("lattice-texture", self.lattice.as_image(available_space as usize), Default::default());
                println!("Texture time: {:.5}", (std::time::Instant::now() - start).as_secs_f32());
                tex
            });
            
            ui.image(texture, egui::Vec2::new(available_space, available_space));
        });

        ctx.request_repaint_after(std::time::Duration::from_secs_f32(1.0/self.fps));
    }
}
