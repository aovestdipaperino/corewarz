//! Drawing a battle.
//!
//! [`render`] is a pure function of the battle and the display state onto a
//! frame. It never mutates either, and it never touches the terminal directly.

use corewarz_asm::disassemble_instruction;
use corewarz_core::{Mars, WarriorId};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier as TextModifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::state::VisualState;

/// The colour assigned to each warrior, by index.
const WARRIOR_COLORS: [Color; 6] = [
    Color::Cyan,
    Color::Magenta,
    Color::Green,
    Color::Yellow,
    Color::Blue,
    Color::Red,
];

/// The display colour for `warrior`.
#[must_use]
pub fn warrior_color(warrior: WarriorId) -> Color {
    WARRIOR_COLORS[warrior.0 as usize % WARRIOR_COLORS.len()]
}

/// Draws the whole interface.
pub fn render(frame: &mut Frame, mars: &Mars, state: &VisualState) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),
            Constraint::Length(8),
            Constraint::Length(3),
        ])
        .split(frame.area());
    render_core(frame, areas[0], mars, state);
    render_sources(frame, areas[1], mars);
    render_status(frame, areas[2], mars, state);
}

fn render_core(frame: &mut Frame, area: Rect, mars: &Mars, state: &VisualState) {
    let block = Block::default().borders(Borders::ALL).title(" core ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let width = u32::from(inner.width).max(1);
    let height = u32::from(inner.height).max(1);
    let capacity = width * height;
    let per_char = VisualState::cells_per_cell(mars.core().size(), capacity);

    let mut lines = Vec::with_capacity(height as usize);
    for row in 0..height {
        let mut spans = Vec::with_capacity(width as usize);
        for column in 0..width {
            let first = (row * width + column) * per_char;
            if first >= mars.core().size() {
                spans.push(Span::raw(" "));
                continue;
            }
            // A run is drawn with the colour of its owner and the heat of its
            // hottest cell, so activity is never lost to aggregation.
            let mut owner = None;
            let mut heat = 0u8;
            let mut executing = false;
            for offset in 0..per_char {
                let addr = first + offset;
                if addr >= mars.core().size() {
                    break;
                }
                owner = mars.core().owner(addr).or(owner);
                heat = heat.max(state.heat(addr));
                executing |= (0..mars.warriors().len())
                    .any(|i| state.exec_position(WarriorId(i as u8)) == Some(addr));
            }
            spans.push(cell_span(owner, heat, executing));
        }
        lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

fn cell_span(owner: Option<WarriorId>, heat: u8, executing: bool) -> Span<'static> {
    let (glyph, color) = match owner {
        None => (if heat > 0 { "\u{b7}" } else { " " }, Color::DarkGray),
        Some(w) => (
            if heat > 128 { "\u{2588}" } else { "\u{2593}" },
            warrior_color(w),
        ),
    };
    let mut style = Style::default().fg(color);
    if executing {
        style = style.add_modifier(TextModifier::REVERSED);
    }
    Span::styled(glyph, style)
}

fn render_sources(frame: &mut Frame, area: Rect, mars: &Mars) {
    let block = Block::default().borders(Borders::ALL).title(" execution ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = Vec::new();
    for (index, warrior) in mars.warriors().iter().enumerate() {
        let id = WarriorId(index as u8);
        let color = warrior_color(id);
        match mars.process_pc(id) {
            None => lines.push(Line::from(Span::styled(
                format!("{:<12} dead", warrior.name),
                Style::default().fg(color),
            ))),
            Some(pc) => {
                // Two cells of context either side of the program counter.
                for delta in -2i64..=2 {
                    let addr = mars.core().fold(i64::from(pc) + delta);
                    let text = format!(
                        "{:<12} {addr:>5}  {}",
                        if delta == 0 {
                            warrior.name.as_str()
                        } else {
                            ""
                        },
                        disassemble_instruction(mars.core().get(addr))
                    );
                    let mut style = Style::default().fg(color);
                    if delta == 0 {
                        style = style.add_modifier(TextModifier::BOLD);
                    }
                    lines.push(Line::from(Span::styled(text, style)));
                }
            }
        }
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_status(frame: &mut Frame, area: Rect, mars: &Mars, state: &VisualState) {
    let mut spans = vec![Span::raw(format!("cycle {:>7}   ", mars.cycle()))];
    for (index, warrior) in mars.warriors().iter().enumerate() {
        let id = WarriorId(index as u8);
        spans.push(Span::styled(
            format!(
                "{}: {} proc / {} dead   ",
                warrior.name,
                mars.process_count(id),
                state.deaths(id)
            ),
            Style::default().fg(warrior_color(id)),
        ));
    }
    let block = Block::default().borders(Borders::ALL).title(" status ");
    frame.render_widget(Paragraph::new(Line::from(spans)).block(block), area);
}
