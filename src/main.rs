use ratatui::{
    DefaultTerminal, Frame, crossterm::event::{self, Event, KeyCode, KeyEventKind}, layout::{Constraint, Layout, Position}, style::{Color, Modifier, Style, Stylize}, text::{Line, Span, Text}, widgets::{Block, List, ListItem, Paragraph}
};
use color_eyre::Result;


use crate::message::Message;

mod message;

fn main() -> Result<()> {
    let window = ratatui::init();
    let res = App::new().run(window);
    res
}

enum InputMode {
    Normal,
    Messaging,
    // trolling,
}

struct App {
    message_input: String,
    cursor_index: usize,
    input_mode: InputMode,
    messages: Vec<Message>,
    active_channel: String
}

impl App {
    const fn new() -> Self {
        Self {
            message_input: String::new(),
            input_mode: InputMode::Normal,
            messages: Vec::new(),
            cursor_index: 0,
            active_channel: String::new()
        }
    }

    fn draw(&self, frame: &mut Frame) {
        let vertical = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(1)
        ]);

        let [tooltip_area, input_area, messages_area] = vertical.areas(frame.area());
        

        /* render the tooltip */

        // stylize and add content
        let (tooltip, style) = match self.input_mode {
            InputMode::Normal => (
                vec![
                    "Press ".into(),
                    "i".bold(),
                    " to type a message or command.".into(),
                    " or ".into(),
                    "q".bold(),
                    " to quit.".into()
                ],
                Style::default().add_modifier(Modifier::RAPID_BLINK),
            ),
            InputMode::Messaging => (
                vec![
                "Press ".into(),
                "Escape".bold(),
                " to stop editing, and ".into(),
                "Enter".bold(),
                " to send ".into()
                ],
                Style::default()
            )
        };
        
        let tooltip_message = {
            let text = Text::from(Line::from(tooltip)).patch_style(style);
            Paragraph::new(text)
        };
        frame.render_widget(tooltip_message, tooltip_area);
        
        /* render the user input box */
        let input = Paragraph::new(self.message_input.as_str())
            .style(match self.input_mode {
                InputMode::Normal => Style::default(),
                InputMode::Messaging => Style::default().fg(Color::Yellow),
            })
            .block(Block::bordered().title(format!("Message #{}", self.active_channel)));
        frame.render_widget(input, input_area);

        /* render the cursor if editing */
        match self.input_mode {
            InputMode::Normal => {},

            #[allow(clippy::cast_possible_truncation)]
            InputMode::Messaging => frame.set_cursor_position(Position::new(
                    input_area.x + self.cursor_index as u16 + 1, 
                    input_area.y + 1
                )),
        }
        

        /* render the messages */
        let messages: Vec<ListItem> = self
            .messages
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let content = Line::from(Span::raw(format!("{i}: {}", m.content)));
                ListItem::new(content)
            })
            .collect();

        let messages = List::new(messages).block(Block::bordered()
            .title(format!("#{}", self.active_channel)));

        frame.render_widget(messages, messages_area);



    }
       
    fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;

            if let Event::Key(key) = event::read()? {
                match self.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('i') => {
                            self.input_mode = InputMode::Messaging;
                        },
                        KeyCode::Char('q') => {
                            return Ok(());
                        },
                        _ => {}
                    },
                    InputMode::Messaging if key.kind == KeyEventKind::Press => match key.code {
                        KeyCode::Enter => todo!(), // submit message
                        KeyCode::Char(to_insert) => todo!(), // append character to message
                        KeyCode::Backspace => todo!(), // delete the last character
                        KeyCode::Left => todo!(), // move cursor left
                        KeyCode::Right => todo!(), // move cursor right
                        KeyCode::Esc => self.input_mode = InputMode::Normal,
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
    }
}
