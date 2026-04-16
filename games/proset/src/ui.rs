use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::card::GameMode;
use crate::client::ProSetClient;

const DOT_COLORS: [Color; 6] = [
    Color::Red,
    Color::Yellow,
    Color::Green,
    Color::Cyan,
    Color::Blue,
    Color::Magenta,
];

/// Render the ProSet game area (header + cards). Returns the height consumed.
pub fn render(
    f: &mut Frame,
    game: &ProSetClient,
    area: Rect,
    room_code: &str,
    player_name: &str,
) {
    let card_rows = match game.mode {
        GameMode::TwelveCard => 3,
        GameMode::SevenCard => 2,
    };
    let card_area_height = (card_rows * 4 + 1) as u16;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),               // header/scores
            Constraint::Length(card_area_height), // cards
            Constraint::Min(0),                  // remaining space (caller uses for log)
        ])
        .split(area);

    // Header: scores, sorted by score descending (stable for ties).
    let mut sorted_scores: Vec<&crate::protocol::ScoreEntry> = game.scores.iter().collect();
    sorted_scores.sort_by(|a, b| b.score.cmp(&a.score));

    let score_spans: Vec<Span> = sorted_scores
        .iter()
        .map(|entry| {
            let style = if !entry.connected {
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::CROSSED_OUT)
            } else if entry.name == player_name {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Span::styled(format!(" {}:{} ", entry.name, entry.score), style)
        })
        .collect();

    let mode_name = game.mode.display_name();
    let timer = game.elapsed_display();
    let timer_style = if game.final_elapsed_ms.is_some() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let mut header_spans = score_spans;
    header_spans.push(Span::styled(format!(" [{timer}]"), timer_style));

    let header = Paragraph::new(Line::from(header_spans)).block(
        Block::default()
            .title(format!(
                " {mode_name} - Room {room_code} - {}/63 cards left ",
                game.deck_remaining,
            ))
            .borders(Borders::ALL),
    );
    f.render_widget(header, chunks[0]);

    // Cards
    render_cards(f, game, chunks[1]);
}

/// Get the total height needed for the ProSet game area.
pub fn game_area_height(mode: GameMode) -> u16 {
    let card_rows = match mode {
        GameMode::TwelveCard => 3,
        GameMode::SevenCard => 2,
    };
    3 + (card_rows * 4 + 1) as u16
}

fn render_cards(f: &mut Frame, game: &ProSetClient, area: Rect) {
    let card_w: u16 = 10;
    let card_h: u16 = 4;

    for (idx, &card_val) in game.cards.iter().enumerate() {
        if card_val == 0 {
            continue;
        }

        let (col, row) = match game.mode {
            GameMode::TwelveCard => (idx / 3, idx % 3),
            GameMode::SevenCard => {
                let positions = [(0, 0), (1, 0), (2, 0), (3, 0), (0, 1), (1, 1), (2, 1)];
                if idx < positions.len() {
                    positions[idx]
                } else {
                    continue;
                }
            }
        };

        let x = area.x + 1 + col as u16 * card_w;
        let y = area.y + row as u16 * card_h;

        if x + card_w > area.x + area.width || y + card_h > area.y + area.height {
            continue;
        }

        let card_area = Rect::new(x, y, card_w - 1, card_h);
        let selected = game.selected.contains(&idx);
        render_single_card(f, card_val, card_area, selected, &game.key_for_index(idx));
    }
}

fn render_single_card(f: &mut Frame, val: u8, area: Rect, selected: bool, key: &str) {
    let border_style = if selected {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style);
    f.render_widget(block, area);

    let pip_offsets = [(1, 0), (3, 0), (5, 0), (1, 1), (3, 1), (5, 1)];
    let inner_x = area.x + 1;
    let inner_y = area.y + 1;

    for (bit, &(dx, dy)) in pip_offsets.iter().enumerate() {
        if (val >> bit) & 1 == 1 {
            let px = inner_x + dx;
            let py = inner_y + dy;
            if px < area.x + area.width - 1 && py < area.y + area.height - 1 {
                let buf = f.buffer_mut();
                buf[(px, py)]
                    .set_char('\u{25CF}')
                    .set_fg(DOT_COLORS[bit]);
            }
        }
    }

    if !key.is_empty() {
        let kx = area.x + area.width - 2;
        let ky = area.y + area.height - 1;
        if kx < area.x + area.width && ky < area.y + area.height {
            let buf = f.buffer_mut();
            buf[(kx, ky)]
                .set_char(key.chars().next().unwrap_or(' '))
                .set_fg(Color::DarkGray);
        }
    }
}
