mod app;
mod connection;
mod screens;

use app::App;
use iced::application;

pub fn main() -> iced::Result {
    application("Powergrid", App::update, App::view)
        .subscription(App::subscription)
        .run_with(App::new)
}
