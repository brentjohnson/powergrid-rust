mod app;
mod connection;
mod screens;

use app::App;
use iced::application;

pub fn main() -> iced::Result {
    application(App::new, App::update, App::view)
        .title("Powergrid")
        .subscription(App::subscription)
        .run()
}
