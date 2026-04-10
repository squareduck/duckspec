//! duckboard — GUI for the duckspec framework, built with Iced 0.14.

use iced::widget::{button, column, text};
use iced::Element;

#[derive(Default)]
struct State {
    counter: u64,
}

#[derive(Debug, Clone)]
enum Message {
    Increment,
}

fn update(state: &mut State, message: Message) {
    match message {
        Message::Increment => state.counter += 1,
    }
}

fn view(state: &State) -> Element<'_, Message> {
    column![
        text("duckboard"),
        text(format!("Counter: {}", state.counter)),
        button("Increment").on_press(Message::Increment),
    ]
    .spacing(10)
    .padding(20)
    .into()
}

fn main() -> iced::Result {
    tracing_subscriber::fmt::init();
    tracing::info!("duckboard starting");

    iced::application(State::default, update, view)
        .title("duckboard")
        .run()
}
