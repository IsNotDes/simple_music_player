// src/ui.rs
use crate::app::{App, InputMode};
// --- MODIFIED IMPORT ---
// Add Bar and BarGroup to the imports
use ratatui::{
    prelude::*,
    widgets::{Bar, BarChart, BarGroup, Block, Borders, List, ListItem, ListState, Paragraph},
};
// --- END MODIFIED IMPORT ---

pub fn ui(f: &mut Frame, app: &mut App) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(f.area());
    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_chunks[0]);
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(top_chunks[0]);

    let items_to_display = if app.input.is_empty() {
        &app.playlist
    } else {
        &app.search_results
    };
    let playlist_items: Vec<ListItem> = items_to_display
        .iter()
        .map(|p| ListItem::new(p.file_name().unwrap_or_default().to_string_lossy()))
        .collect();
    let mut playlist_state = ListState::default();
    playlist_state.select(app.selected_song_index);
    let playlist = List::new(playlist_items)
        .block(Block::default().title("Playlist").borders(Borders::ALL))
        .highlight_symbol("> ")
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).bg(Color::Blue));
    f.render_stateful_widget(playlist, left_chunks[0], &mut playlist_state);

    let input = Paragraph::new(app.input.as_str())
        .style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Style::default().fg(Color::Yellow),
        })
        .block(Block::default().title("Search").borders(Borders::ALL));
    f.render_widget(input, left_chunks[1]);

    let spectrogram_data = app.spectrogram_data.lock().unwrap();
    let max_display_height = 500.0; // Max height for the bars
    let min_db = -100.0; // Minimum decibel value to display
    let max_db = 100.0;   // Maximum decibel value to display

    // --- MODIFIED PART ---
    // Create Bars without explicitly setting labels or value styles
    let bars: Vec<Bar> = spectrogram_data
        .iter()
        .map(|&v| {
            // Clamp the value to the display range
            let clamped_v = v.max(min_db).min(max_db);
            // Normalize to 0-1 range
            let normalized_v = (clamped_v - min_db) / (max_db - min_db);
            // Scale to u64 for display
            let height = (normalized_v * max_display_height) as u64;
            // Create a Bar with only the value.
            // Not calling .label() or .value_style() should use defaults that render nothing.
            Bar::default().value(height)
        })
        .collect();

    // Create a single BarGroup containing all the bars, without a group label
    let bargroup = BarGroup::default().bars(&bars);

    // Create the BarChart using the BarGroup
    let barchart = BarChart::default()
        .block(Block::default().title("Visualizer").borders(Borders::ALL))
        .data(bargroup) // Pass the BarGroup
        .bar_width(1)
        .bar_gap(0);
    // --- END MODIFIED PART ---

    f.render_widget(barchart, top_chunks[1]);

    let playback_status = if app.is_playing { "Playing" } else { "Paused" };
    let current_song = app.current_song_path
        .as_ref()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("No song");
    let status_text = format!("Status: {} | Song: {}", playback_status, current_song);
    let status_block = Block::default().title("Playback").borders(Borders::ALL);
    let status_paragraph = Paragraph::new(status_text).block(status_block);
    f.render_widget(status_paragraph, main_chunks[1]);
}
