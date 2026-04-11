mod app;
mod connection;
mod screens;

use iced::application;
use app::App;

pub fn main() -> iced::Result {
    application("Powergrid", App::update, App::view)
        .subscription(App::subscription)
        .run_with(App::new)
}
