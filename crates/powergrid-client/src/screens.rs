use iced::{
    widget::{button, column, container, row, scrollable, text, text_input},
    Element, Length,
};
use powergrid_core::{
    GameState,
    types::{Phase, PlayerColor, Resource},
};
use crate::app::Message;

// ---------------------------------------------------------------------------
// Connect screen
// ---------------------------------------------------------------------------

pub struct ConnectScreen {
    pub server_url: String,
    pub player_name: String,
    pub selected_color: PlayerColor,
    pub error: Option<String>,
}

impl ConnectScreen {
    pub fn new() -> Self {
        Self {
            server_url: "ws://localhost:3000/ws".to_string(),
            player_name: String::new(),
            selected_color: PlayerColor::Red,
            error: None,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let color_buttons = row![
            color_button(PlayerColor::Red, self.selected_color),
            color_button(PlayerColor::Blue, self.selected_color),
            color_button(PlayerColor::Green, self.selected_color),
            color_button(PlayerColor::Yellow, self.selected_color),
            color_button(PlayerColor::Purple, self.selected_color),
            color_button(PlayerColor::Black, self.selected_color),
        ]
        .spacing(8);

        let mut connect_btn = button("Connect");
        if !self.player_name.is_empty() {
            connect_btn = connect_btn.on_press(Message::Connect);
        }

        let mut col = column![
            text("Powergrid").size(32),
            text("Server URL"),
            text_input("ws://localhost:3000/ws", &self.server_url)
                .on_input(Message::ServerUrlChanged),
            text("Your Name"),
            text_input("Enter your name", &self.player_name)
                .on_input(Message::NameChanged),
            text("Color"),
            color_buttons,
            connect_btn,
        ]
        .spacing(12)
        .padding(40)
        .max_width(400);

        if let Some(err) = &self.error {
            col = col.push(text(err.as_str()));
        }

        container(col)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}

fn color_button(color: PlayerColor, selected: PlayerColor) -> Element<'static, Message> {
    let label = match color {
        PlayerColor::Red => "Red",
        PlayerColor::Blue => "Blue",
        PlayerColor::Green => "Green",
        PlayerColor::Yellow => "Yellow",
        PlayerColor::Purple => "Purple",
        PlayerColor::Black => "Black",
    };
    let btn = button(label).on_press(Message::ColorSelected(color));
    if color == selected {
        button(text(label).size(14))
            .on_press(Message::ColorSelected(color))
            .into()
    } else {
        btn.into()
    }
}

// ---------------------------------------------------------------------------
// Lobby screen
// ---------------------------------------------------------------------------

pub fn lobby_view(state: &GameState, is_host: bool) -> Element<'_, Message> {
    let players_list = state.players.iter().fold(
        column![text("Players:").size(18)].spacing(4),
        |col, p| col.push(text(format!("  {} ({:?})", p.name, p.color))),
    );

    let mut col = column![
        text("Powergrid — Lobby").size(28),
        players_list,
    ]
    .spacing(16)
    .padding(40);

    if is_host {
        let start_btn = if state.players.len() >= 2 {
            button("Start Game").on_press(Message::StartGame)
        } else {
            button("Start Game (need 2+ players)")
        };
        col = col.push(start_btn);
    } else {
        col = col.push(text("Waiting for host to start..."));
    }

    container(col)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

// ---------------------------------------------------------------------------
// Game screen
// ---------------------------------------------------------------------------

pub fn game_view(state: &GameState, my_id: uuid::Uuid) -> Element<'_, Message> {
    let phase_label = phase_description(&state.phase);

    let me = state.player(my_id);

    // Player status panel.
    let player_panel = state.players.iter().fold(
        column![text("Players").size(16)].spacing(4),
        |col, p| {
            let marker = if p.id == my_id { " ◀" } else { "" };
            col.push(text(format!(
                "{}{}: {} cities, ${}, plants: {}",
                p.name,
                marker,
                p.cities.len(),
                p.money,
                p.plants.iter().map(|pl| pl.number.to_string()).collect::<Vec<_>>().join(",")
            )))
        },
    );

    // Power plant market.
    let market = column![
        text("Power Plant Market").size(16),
        text("Actual:"),
        plants_row(&state.market.actual),
        text("Future:"),
        plants_row(&state.market.future),
    ]
    .spacing(4);

    // Resource market.
    let res = &state.resources;
    let resources = column![
        text("Resources").size(16),
        text(format!("Coal: {}  Oil: {}  Garbage: {}  Uranium: {}", res.coal, res.oil, res.garbage, res.uranium)),
    ]
    .spacing(4);

    // My resources + actions.
    let my_panel: Element<Message> = if let Some(me) = me {
        let res = &me.resources;
        column![
            text(format!("You: {} | ${}", me.name, me.money)).size(16),
            text(format!(
                "Resources — Coal: {}  Oil: {}  Garbage: {}  Uranium: {}",
                res.coal, res.oil, res.garbage, res.uranium
            )),
            action_panel(state, my_id),
        ]
        .spacing(8)
        .into()
    } else {
        text("Spectating").into()
    };

    // Event log.
    let log = state.event_log.iter().rev().take(10).fold(
        column![text("Log").size(14)].spacing(2),
        |col, entry| col.push(text(entry.as_str()).size(12)),
    );

    let left = column![
        text(format!("Round {} — {}", state.round, phase_label)).size(20),
        player_panel,
        market,
        resources,
    ]
    .spacing(16)
    .width(Length::FillPortion(3));

    let right = column![my_panel, scrollable(log)]
        .spacing(16)
        .width(Length::FillPortion(2));

    container(row![left, right].spacing(24).padding(24))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn plants_row(plants: &[powergrid_core::types::PowerPlant]) -> Element<'static, Message> {
    plants.iter().fold(row![].spacing(8), |r, p| {
        r.push(
            button(text(format!("#{} {:?} {}→{}", p.number, p.kind, p.cost, p.cities)).size(11))
                .on_press(Message::SelectPlant(p.number))
        )
    })
    .into()
}

fn action_panel(state: &GameState, my_id: uuid::Uuid) -> Element<'_, Message> {
    match &state.phase {
        Phase::Auction { current_bidder_idx, active_bid, .. } => {
            let my_turn = state.player_order.get(*current_bidder_idx) == Some(&my_id);
            if let Some(bid) = active_bid {
                let is_my_bid_turn = bid.remaining_bidders.first() == Some(&my_id);
                if is_my_bid_turn {
                    column![
                        text(format!("Active bid on plant #{}: ${}", bid.plant_number, bid.amount)),
                        text("Enter amount in text field and press Bid, or Pass"),
                        row![
                            button("Pass Bid").on_press(Message::PassAuction),
                        ].spacing(8),
                    ].spacing(8).into()
                } else {
                    text(format!("Bidding on plant #{} — waiting...", bid.plant_number)).into()
                }
            } else if my_turn {
                column![
                    text("Your turn — select a plant from the market above, or:"),
                    button("Pass").on_press(Message::PassAuction),
                ].spacing(8).into()
            } else {
                text("Waiting for other players to bid...").into()
            }
        }
        Phase::BuyResources { remaining } => {
            if remaining.first() == Some(&my_id) {
                column![
                    text("Buy resources (click to buy 1 unit):"),
                    row![
                        button("Coal").on_press(Message::BuyResource(Resource::Coal)),
                        button("Oil").on_press(Message::BuyResource(Resource::Oil)),
                        button("Garbage").on_press(Message::BuyResource(Resource::Garbage)),
                        button("Uranium").on_press(Message::BuyResource(Resource::Uranium)),
                        button("Done").on_press(Message::DoneBuying),
                    ].spacing(8),
                ].spacing(8).into()
            } else {
                text("Waiting for other players to buy resources...").into()
            }
        }
        Phase::BuildCities { remaining } => {
            if remaining.first() == Some(&my_id) {
                column![
                    text("Build cities — enter city ID below:"),
                    row![
                        button("Done Building").on_press(Message::DoneBuilding),
                    ].spacing(8),
                ].spacing(8).into()
            } else {
                text("Waiting for other players to build...").into()
            }
        }
        Phase::Bureaucracy { remaining } => {
            if remaining.first() == Some(&my_id) {
                column![
                    text("Power your cities — press to fire all plants you can:"),
                    button("Power Cities").on_press(Message::PowerCities),
                ].spacing(8).into()
            } else {
                text("Waiting for other players...").into()
            }
        }
        Phase::GameOver { winner } => {
            let name = state.player(*winner).map(|p| p.name.as_str()).unwrap_or("Unknown");
            text(format!("Game Over! {} wins!", name)).size(24).into()
        }
        _ => text("").into(),
    }
}

fn phase_description(phase: &Phase) -> &'static str {
    match phase {
        Phase::Lobby => "Lobby",
        Phase::PlayerOrder => "Determining Player Order",
        Phase::Auction { .. } => "Auction",
        Phase::BuyResources { .. } => "Buy Resources",
        Phase::BuildCities { .. } => "Build Cities",
        Phase::Bureaucracy { .. } => "Bureaucracy",
        Phase::GameOver { .. } => "Game Over",
    }
}
