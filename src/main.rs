#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// hide console window on Windows in release

#![allow(rustdoc::missing_crate_level_docs)]
// it's an example
use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use eframe::egui;
use reqwest;
use tokio::runtime::Builder;

#[derive(Clone, Serialize, Deserialize)]
struct Api {
    api_name: String,
    api_url: String,
    api_key: String,
    model: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct Message {
    sender: String,
    content: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct Session {
    id: usize,
    name: String,
    messages: Vec<Message>,
}

struct MyApp {
    api_name: String,
    api_url: String,
    api_key: String,
    model: String,
    api_list: Vec<Api>,
    selected_api_index: usize,
    input: String,
    sessions: Vec<Session>,
    current_session_index: usize,
    new_session_name: String,
    show_config_window: bool, // Flag to toggle configuration window
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            api_url: "".to_owned(),
            api_key: "".to_owned(),
            model: "".to_owned(),
            api_list: vec![],
            selected_api_index: 0,
            api_name: "".to_owned(),
            input: "".to_owned(),
            sessions: vec![Session { id: 0, name: "Default Session".to_owned(), messages: vec![] }],
            current_session_index: 0,
            new_session_name: "".to_owned(),
            show_config_window: false, // Initially hide configuration window
        }
    }
}

impl MyApp {
    // Function to save `api_list` to a JSON file
    fn save_api_list(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string(&self.api_list)?;
        fs::write(path, json)?;
        println!("Api list saved successfully to {}", path);
        Ok(())
    }

    // Function to load `api_list` from a JSON file
    fn load_api_list(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        if Path::new(path).exists() {
            let json = fs::read_to_string(path)?;
            self.api_list = serde_json::from_str(&json)?;
            println!("Api list loaded successfully from {}", path);
        } else {
            println!("Api list file {} not found", path);
        }
        Ok(())
    }

    // Function to save `sessions` to a JSON file
    fn save_sessions(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string(&self.sessions)?;
        fs::write(path, json)?;
        println!("Sessions saved successfully to {}", path);
        Ok(())
    }

    // Function to load `sessions` from a JSON file
    fn load_sessions(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        if Path::new(path).exists() {
            let json = fs::read_to_string(path)?;
            self.sessions = serde_json::from_str(&json)?;
            println!("Sessions loaded successfully from {}", path);
        } else {
            println!("Sessions file {} not found", path);
        }
        Ok(())
    }

    async fn send_message_to_api(&mut self, api: &Api) -> Result<String, String> {
        let client = reqwest::Client::new();
        let current_session = &self.sessions[self.current_session_index];
        let request_body = serde_json::json!({
            "model": api.model.clone(),
            "messages": current_session.messages.iter().map(|message| {
                serde_json::json!({
                    "role": if message.sender == "user" { "user" } else { "assistant" },
                    "content": message.content,
                })
            }).collect::<Vec<_>>()
        });

        let res = client.post(&api.api_url)
            .header("Authorization", format!("Bearer {}", api.api_key))
            .json(&request_body)
            .send()
            .await;

        match res {
            Ok(response) => {
                match response.json::<serde_json::Value>().await {
                    Ok(data) => {
                        if let Some(message) = data["choices"][0]["message"]["content"].as_str() {
                            Ok(message.to_owned())
                        } else {
                            Err("Invalid response format".to_owned())
                        }
                    },
                    Err(err) => Err(format!("Error reading response: {}", err)),
                }
            },
            Err(err) => Err(format!("Error sending request: {}", err)),
        }
    }

    fn current_session(&mut self) -> &mut Session {
        &mut self.sessions[self.current_session_index]
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Left panel for session management
        egui::SidePanel::left("left_panel").resizable(true).show(ctx, |ui| {
            ui.vertical(|ui| {
                // Header
                ui.heading("Sessions");
                ui.separator();
                // Session management UI
                ui.horizontal(|ui| {
                    if ui.button("New Session").clicked() {
                        let new_session_id = self.sessions.len();
                        let new_session_name = if self.new_session_name.is_empty() {
                            format!("Session {}", new_session_id + 1)
                        } else {
                            self.new_session_name.clone()
                        };
                        self.sessions.push(Session { id: new_session_id, name: new_session_name.clone(), messages: vec![] });
                        self.current_session_index = new_session_id;
                        self.input.clear();
                        self.new_session_name.clear();
                        if let Err(err) = self.save_sessions("sessions.json") {
                            println!("Error saving sessions: {:?}", err);
                        }
                    }
                });

                ui.separator();

                // Session switcher
                for (index, session) in self.sessions.iter().enumerate() {
                    if ui.button(&session.name).clicked() {
                        self.current_session_index = index;
                    }
                }
                if ui.button("Remove Current Session").clicked() {
                    if self.sessions.len() > 1 {
                        self.sessions.remove(self.current_session_index);
                        if self.current_session_index >= self.sessions.len() {
                            self.current_session_index = self.sessions.len() - 1;
                        }
                        if let Err(err) = self.save_sessions("sessions.json") {
                            println!("Error saving sessions: {:?}", err);
                        }
                    } else {
                        // Clear messages and rename the session to "Default Session"
                        self.current_session().messages.clear();
                        self.current_session().name = "Default Session".to_owned();
                        self.save_sessions("sessions.json").unwrap_or_else(|err| {
                            println!("Error saving sessions: {:?}", err);
                        });
                    }
                }
                if ui.button("remove all sessions").clicked(){
                    self.sessions.clear();
                    self.sessions.push(Session { id: 0, name: "Default Session".to_owned(), messages: vec![] });
                    self.current_session_index = 0;
                    self.save_sessions("sessions.json").unwrap_or_else(|err| {
                        println!("Error saving sessions: {:?}", err);
                    });
                }

            });
        });

        // Central panel for chat area and input area
        egui::CentralPanel::default().show(ctx, |ui| {
            let window_size = ctx.available_rect().size();
            let max_scroll_height = window_size.y - 75.0;
            ui.vertical(|ui| {
                // Header (current session name)
                ui.heading(&self.current_session().name);
                ui.separator();


                

                // Chat area with scrolling
                egui::ScrollArea::vertical().max_height(max_scroll_height).show(ui, |ui| {
                    ui.vertical(|ui| {
                        for message in &self.current_session().messages {
                            let text_color = match message.sender.as_str() {
                                "user" => egui::Color32::from_rgb(72, 219, 120), // Green
                                "API" => egui::Color32::from_rgb(66, 133, 244),  // Blue
                                "system" => egui::Color32::from_rgb(234, 67, 53), // Red
                                _ => egui::Color32::WHITE,
                            };

                            if message.content.contains("```") {
                                // Display in text editor if message contains "```"
                                let start_index = message.content.find("```").unwrap();
                                let end_index = message.content[start_index + 3..].find("```")
                                    .map(|i| start_index + 3 + i)
                                    .unwrap_or_else(|| message.content.len());

                                ui.horizontal(|ui| {
                                    ui.colored_label(text_color, &message.content[..start_index]);
                                    ui.end_row();
                                });

                                let mut text_editor_content = message.content[start_index + 3..end_index].to_owned();
                                ui.horizontal(|ui|{
                                    ui.add(egui::TextEdit::multiline(&mut text_editor_content).text_color(text_color));
                                    ui.end_row();
                                });

                                ui.horizontal(|ui|{
                                    ui.colored_label(text_color, &message.content[end_index+3..]);
                                    ui.end_row();
                                });
                            } else {
                                // Display in label if message does not contain "```"
                                ui.colored_label(text_color, message.content.clone());
                            }

                            ui.separator();
                        }
                    });
                });

                ui.separator();

                // Input area for sending messages
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.input)
                        .hint_text("Say Hello")
                        .text_color(egui::Color32::WHITE)
                    );

                    if ui.button("Send").clicked() {
                        if let Some(api) = self.api_list.get(self.selected_api_index) {
                            let input_text = self.input.clone();
                            let api_clone = api.clone();
                            let runtime = Builder::new_current_thread()
                                .enable_all()
                                .build()
                                .unwrap();

                            runtime.block_on(async {
                                self.current_session().messages.push(Message {
                                    sender: "user".to_owned(),
                                    content: input_text.clone(),
                                });

                                match self.send_message_to_api(&api_clone).await {
                                    Ok(message) => {
                                        self.current_session().messages.push(Message {
                                            sender: "API".to_owned(),
                                            content: message.clone(),
                                        });
                                        self.input = "".to_string();

                                        // Save sessions after sending
                                        if let Err(err) = self.save_sessions("sessions.json") {
                                            println!("Error saving sessions: {:?}", err);
                                        }
                                    }
                                    Err(err) => {
                                        self.current_session().messages.push(Message {
                                            sender: "system".to_owned(),
                                            content: format!("Error: {}", err),
                                        });
                                    }
                                }
                            });
                        }
                    }
                    if ui.button("Configure API").clicked() {
                        self.show_config_window = !self.show_config_window;
                    }
                });

            });
        });

        // Configuration window
        if self.show_config_window {
            egui::Window::new("Configuration Window")
                .resizable(true)
                .show(ctx, |ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.api_name).hint_text("API Name"));
                    ui.add(egui::TextEdit::singleline(&mut self.api_url).hint_text("API Url"));
                    ui.add(egui::TextEdit::singleline(&mut self.api_key).hint_text("API Key"));
                    ui.add(egui::TextEdit::singleline(&mut self.model).hint_text("Model (e.g., gpt-3.5-turbo)"));

                    if ui.button("Save").clicked() {
                        let api = Api {
                            api_url: self.api_url.clone(),
                            api_key: self.api_key.clone(),
                            model: self.model.clone(),
                            api_name: self.api_name.clone(),
                        };
                        self.api_list.push(api);

                        if let Err(err) = self.save_api_list("api_list.json") {
                            println!("Error saving api_list: {:?}", err);
                        } else {
                            println!("api_list saved successfully");
                        }
                    }

                    ui.separator();
                    ui.label("Choose an API to use");

                    egui::ComboBox::from_id_source("api_list")
                        .selected_text("Select an API")
                        .width(200.0)
                        .show_index(ui, &mut self.selected_api_index, self.api_list.len(), |i| {
                            if let Some(api) = self.api_list.get(i) {
                                format!("{} - {}", api.api_name, api.api_url)
                            } else {
                                "".to_string()
                            }
                        });
                });
        }
    }
}

fn main() -> eframe::Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    // Create the application instance
    let mut app = MyApp::default();

    // Attempt to load api_list.json and sessions.json
    if let Err(err) = app.load_api_list("api_list.json") {
        println!("Error loading api_list: {:?}", err);
    } else {
        println!("api_list loaded successfully");
    }

    if let Err(err) = app.load_sessions("sessions.json") {
        println!("Error loading sessions: {:?}", err);
    } else {
        println!("Sessions loaded successfully");
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]), // Adjust viewport size as needed
        ..Default::default()
    };

    // Run the application
    eframe::run_native(
        "My AI Chat App",
        options,
        Box::new(move |cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);

            // Return the application instance
            Ok(Box::new(app))
        }),
    )
}
