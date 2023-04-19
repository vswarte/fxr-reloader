use rfd::FileDialog;
use image::ImageFormat;
use std::path::PathBuf;
use iced::window::icon::Icon;
use iced::window::Settings as WindowSettings;
use iced::widget::{button, column, row, pick_list, vertical_space, text};
use iced::{Application, Command, Element, Settings, Theme, Length, Subscription, Color, executor};

pub(crate) mod game;
use crate::game::{get_running_games, GameProcess};

const MAX_LOG_ENTRIES: usize = 10;

pub fn main() -> iced::Result {
    let icon = Icon::from_file_data(include_bytes!("icon.png"), Some(ImageFormat::Png)).unwrap();

    ReloaderGUI::run(Settings {
        window: WindowSettings {
            size: (300, 400),
            resizable: false,
            icon: Some(icon),
            ..WindowSettings::default()
        },
        ..Settings::default()
    })
}

struct ReloaderGUI {
    selected_game: Option<GameProcess>,
    last_files: Option<Vec<PathBuf>>,
    log_entries: Vec<String>,
}

impl ReloaderGUI {
    // It's probably OK calling unwrap directly as we only have the on_press registered when
    // self.selected_game is Some(T)
    fn patch_fxr_files(&self, files: Vec<PathBuf>) {
        let process_id = self.selected_game.as_ref().unwrap().pid;

        for file in files.into_iter() {
            game::call_fxr_patch(process_id, file).unwrap();
        }
    }

    fn add_log_entry(&mut self, entry: String) {
        self.log_entries.push(entry);

        if self.log_entries.len() > MAX_LOG_ENTRIES {
            self.log_entries.remove(0);
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    SelectedGameChanged(GameProcess),
    ReloadFXRPressed,
    ReloadLastFilesPressed,
    UpdateLog,
}

impl Application for ReloaderGUI {
    type Executor = executor::Default;
    type Flags = ();
    type Message = Message;
    type Theme = Theme;

    fn new(_flags: ()) -> (Self, Command<Self::Message>) {
        (Self {
            selected_game: get_running_games()
                .first()
                .map(|x| x.clone()),
            last_files: None,
            log_entries: vec![
                String::from("Made by chainfailure"),
                String::from("Version 0.2.1"),
            ],
        }, Command::none())
    }

    fn title(&self) -> String {
        String::from("FXR Reloader")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::SelectedGameChanged(game) => {
                self.selected_game = Some(game);
            }
            Message::ReloadFXRPressed => {
                let selection = prompt_fxr_files();
                if let Some(selected_files) = selection {
                    self.patch_fxr_files(selected_files.clone());
                    self.last_files = Some(selected_files);
                    self.add_log_entry(String::from("Reloaded FXR definition(s)"));
                }
            }
            Message::ReloadLastFilesPressed => {
                self.patch_fxr_files(self.last_files.as_ref().unwrap().clone());
                self.add_log_entry(String::from("Reloaded last patched FXR(s)"));
            }
            Message::UpdateLog => {
                // self.add_log_entry(String::from("Blah"));
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let game_processes = get_running_games();

        let mut reload_fxr_button = button("Reload FXR");
        if self.selected_game.is_some() {
            reload_fxr_button = reload_fxr_button
                .on_press(Message::ReloadFXRPressed);
        }

        let log_entries = self.log_entries.join("\n");

        let mut reload_last_button = button("Reload Last Reloaded FXR");
        if self.last_files.is_some() {
            reload_last_button = reload_last_button.on_press(Message::ReloadLastFilesPressed);
        }

        column![
            "Game process",
            pick_list(game_processes, self.selected_game.clone(), move |x| {
                Message::SelectedGameChanged(x)
            })
                .placeholder("Select the running game you want to patch FXRs in")
                .width(Length::Fill),

            vertical_space(20),

            row![
                reload_fxr_button.width(Length::Fill),
            ],

            vertical_space(10),

            row! [
                reload_last_button.width(Length::Fill),
            ],

            vertical_space(Length::Fill),

            text(log_entries)
                .style(Color::from([0.8, 0.8, 0.8]))
                .size(18),
        ]
            .padding(20)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(500)).map(|_| {
            Message::UpdateLog
        })
    }
}

fn prompt_fxr_files() -> Option<Vec<PathBuf>> {
    FileDialog::new()
        .add_filter("FXR Files", &["fxr"])
        .set_directory("/")
        .pick_files()
}
