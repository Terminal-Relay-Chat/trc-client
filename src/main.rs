use ratatui::{
    DefaultTerminal, Frame, crossterm::{event::{self, Event, KeyCode, KeyEventKind}, terminal::{disable_raw_mode, enable_raw_mode}}, layout::{Constraint, Layout, Position}, style::{Color, Modifier, Style, Stylize}, text::{Line, Span, Text}, widgets::{Block, List, ListItem, Paragraph}
};
use color_eyre::Result;
use tokio::sync::Mutex;
use std::sync::Arc;


use crate::message::Message;

mod message;
mod networking;

#[tokio::main]
async fn main() -> Result<()> {
    enable_raw_mode()?;

    let target_ip: String = std::env::args().nth(1).expect("Can't call this binary without a target ip address");
    let secure_url: bool = match std::env::args().nth(2) {
        Some(content) => match content.to_lowercase().trim() {
            "false" => false,
            "true" => true,
            _ => panic!("2nd argument is if the api is secure or not. found an unexpected answer.")
        },
        None => true, // default to a secure api
    };

    let window = ratatui::init();
    let res = App::new().run(window, target_ip, secure_url).await;

    disable_raw_mode()?;
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
    messages: Arc<Mutex<Vec<Message>>>,
    active_channel: String,
    api_token: Option<String>
}

impl App {
    fn new() -> Self {
        Self {
            message_input: String::new(),
            input_mode: InputMode::Normal,
            messages: Arc::new(Mutex::new(Vec::new())),
            cursor_index: 0,
            active_channel: String::from("general"),
            api_token: None
        }
    }

    fn draw(&self, frame: &mut Frame, messages_snapshot: Vec<Message>) {
        let vertical = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(3),
        ]);

        let [tooltip_area, messages_area, input_area] = vertical.areas(frame.area());
        

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
            .block(Block::bordered().title(format!("Message #{} ", self.active_channel)));
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
        let messages: Vec<ListItem> = messages_snapshot
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let content = Line::from(Span::raw(format!("{i}: {}", m.content)));
                ListItem::new(content)
            })
            .collect();

        let messages = List::new(messages).block(Block::bordered()
            .title(format!("#{} ", self.active_channel)));

        frame.render_widget(messages, messages_area);



    }
    
    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.message_input.insert(index, new_char);
        self.move_cursor_right();
    }

    fn byte_index(&self) -> usize {
        self.message_input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.cursor_index)
            .unwrap_or(self.message_input.len())
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.cursor_index.saturating_sub(1);
        self.cursor_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.cursor_index.saturating_add(1);
        self.cursor_index = self.clamp_cursor(cursor_moved_right);
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.cursor_index != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.cursor_index;
            let from_left_to_current_index = current_index - 1;

            let before_char_to_delete = self.message_input.chars().take(from_left_to_current_index);
            let after_char_to_delete = self.message_input.chars().skip(current_index);

            self.message_input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }
    
    fn reset_cursor(&mut self) {
        self.cursor_index = 0;
    }
    
    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.message_input.chars().count())
    }

    fn submit_message(&mut self) {
        // self.messages.push(self.message_input.clone());
        self.message_input.clear();
        self.reset_cursor();
    }

    async fn run(mut self, mut terminal: DefaultTerminal, target_ip: String, secure: bool) -> Result<()> {
        print!("connecting to server...");
        let token = networking::get_token(&target_ip, &secure).await;

        self.messages.lock().await.push(Message { sender: String::new(), content: token });
        tokio::spawn(async {
            
        });

        loop {
            let messages_snapshot = self.messages.lock().await.clone();
            terminal.draw(|frame| self.draw(frame, messages_snapshot))?;

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
                        KeyCode::Enter => self.submit_message(), // submit message
                        KeyCode::Char(to_insert) => self.enter_char(to_insert), // append character to message
                        KeyCode::Backspace => self.delete_char(), // delete the last character
                        KeyCode::Left => self.move_cursor_left(), // move cursor left
                        KeyCode::Right => self.move_cursor_right(), // move cursor right
                        KeyCode::Esc => self.input_mode = InputMode::Normal,
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
    }
}
