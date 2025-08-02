// app.txt
use crossterm::event::{self, Event, KeyCode};

use ratatui::prelude::*;
use rodio::{Decoder, OutputStream, Sink};
use std::{
    error::Error,
    fs,
    io::{self, BufReader},
    path::{Path, PathBuf},
    time::Duration,
};
use crate::ui::ui;
use rand::Rng;

pub enum InputMode {
    Normal,
    Editing,
}

pub struct App {
    pub input: String,
    pub input_mode: InputMode,
    pub playlist: Vec<PathBuf>,
    pub search_results: Vec<PathBuf>,
    pub _stream: OutputStream,
    pub sink: Sink,
    pub current_song_path: Option<PathBuf>,
    pub selected_song_index: Option<usize>,
    pub is_playing: bool,
    pub waveform: Vec<u64>,
}

impl App {
    pub fn new() -> Result<App, Box<dyn Error>> {
        let (stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;
        let playlist = Self::load_playlist("music")?;
        let selected_song_index = if playlist.is_empty() { None } else { Some(0) };

        Ok(App {
            input: String::new(),
            input_mode: InputMode::Normal,
            playlist,
            search_results: vec![],
            _stream: stream,
            sink,
            current_song_path: None,
            selected_song_index,
            is_playing: false,
            waveform: vec![0; 100],
        })
    }

    fn load_playlist<P: AsRef<Path>>(path: P) -> Result<Vec<PathBuf>, io::Error> {
        let mut playlist = Vec::new();
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                playlist.extend(Self::load_playlist(&path)?);
            } else if path.is_file() {
                if let Some(extension) = path.extension() {
                    if let Some(extension_str) = extension.to_str() {
                        match extension_str {
                            "mp3" | "flac" | "wav" => {
                                playlist.push(path.canonicalize()?);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        Ok(playlist)
    }

    pub fn play_pause(&mut self) {
        if self.sink.is_paused() {
            self.sink.play();
            self.is_playing = true;
        } else {
            self.sink.pause();
            self.is_playing = false;
        }
    }

    pub fn is_seekable(&self) -> bool {
        !self.sink.empty()
    }

    pub fn seek_forward(&mut self) {
        if self.is_seekable() {
            let current_pos = self.sink.get_pos();
            let new_pos = current_pos + Duration::from_secs(5);
            if let Err(e) = self.sink.try_seek(new_pos) {
                if !e.to_string().contains("end of stream") {
                    eprintln!("Error seeking forward: {}", e);
                }
            }
        }
    }

    pub fn seek_backward(&mut self) {
        if self.is_seekable() {
            let current_pos = self.sink.get_pos();
            let new_pos = if current_pos > Duration::from_secs(5) {
                current_pos - Duration::from_secs(5)
            } else {
                Duration::ZERO
            };
            if let Err(e) = self.sink.try_seek(new_pos) {
                if !e.to_string().contains("end of stream") {
                    eprintln!("Error seeking backward: {}", e);
                }
            }
        }
    }

    pub fn play_selected_song(&mut self) -> Result<(), Box<dyn Error>> {
        let song_to_play = if self.input.is_empty() {
            self.selected_song_index.and_then(|i| self.playlist.get(i).cloned())
        } else {
            self.selected_song_index.and_then(|i| self.search_results.get(i).cloned())
        };

        if let Some(song_path) = song_to_play {
            self.play_song_by_path(&song_path)?;
        }

        Ok(())
    }

    fn play_song_by_path(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        self.sink.stop();
        self.sink.clear();

        let file = BufReader::new(fs::File::open(path)?);
        let source = Decoder::new(file)?;

        self.sink.append(source);
        self.sink.play();
        self.current_song_path = Some(path.to_path_buf());
        self.is_playing = true;

        Ok(())
    }

    pub fn next_song(&mut self) -> Result<(), Box<dyn Error>> {
        let songs_to_play = if self.input.is_empty() {
            &self.playlist
        } else {
            &self.search_results
        };

        if songs_to_play.is_empty() {
            return Ok(());
        }

        let current_index = self
            .current_song_path
            .as_ref()
            .and_then(|p| songs_to_play.iter().position(|s| s == p));

        let next_index = match current_index {
            Some(i) => (i + 1) % songs_to_play.len(),
            None => 0,
        };

        let next_path = songs_to_play[next_index].clone();
        self.play_song_by_path(&next_path)?;
        self.selected_song_index = Some(next_index);

        Ok(())
    }

    pub fn previous_song(&mut self) -> Result<(), Box<dyn Error>> {
        let songs_to_play = if self.input.is_empty() {
            &self.playlist
        } else {
            &self.search_results
        };

        if songs_to_play.is_empty() {
            return Ok(());
        }

        let current_index = self
            .current_song_path
            .as_ref()
            .and_then(|p| songs_to_play.iter().position(|s| s == p));

        let prev_index = match current_index {
            Some(i) => {
                if i == 0 {
                    songs_to_play.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };

        let prev_path = songs_to_play[prev_index].clone();
        self.play_song_by_path(&prev_path)?;
        self.selected_song_index = Some(prev_index);

        Ok(())
    }

    pub fn select_next(&mut self) {
        let songs_to_play = if self.input.is_empty() {
            &self.playlist
        } else {
            &self.search_results
        };

        let len = songs_to_play.len();
        if len == 0 {
            return;
        }

        let i = self.selected_song_index.map_or(0, |i| (i + 1) % len);
        self.selected_song_index = Some(i);
    }

    pub fn select_previous(&mut self) {
        let songs_to_play = if self.input.is_empty() {
            &self.playlist
        } else {
            &self.search_results
        };

        let len = songs_to_play.len();
        if len == 0 {
            return;
        }

        let i = self
            .selected_song_index
            .map_or(0, |i| if i == 0 { len - 1 } else { i - 1 });
        self.selected_song_index = Some(i);
    }

    pub fn tick(&mut self) {
        if self.is_playing && self.sink.empty() {
            let _ = self.next_song();
        }
        self.update_waveform();
    }

    fn update_waveform(&mut self) {
        let mut rng = rand::thread_rng();
        let new_waveform: Vec<u64> = (0..100).map(|_| rng.gen_range(0..100)).collect();
        self.waveform = new_waveform;
    }
}

pub fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;
        app.tick();

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match app.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('e') => app.input_mode = InputMode::Editing,
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('p') => app.play_pause(),
                        KeyCode::Char('n') => {
                            let _ = app.next_song();
                        }
                        KeyCode::Char('b') => {
                            let _ = app.previous_song();
                        }
                        KeyCode::Char(' ') => {
                            let _ = app.play_selected_song();
                        }
                        KeyCode::Down => app.select_next(),
                        KeyCode::Up => app.select_previous(),
                        KeyCode::Left => app.seek_backward(),
                        KeyCode::Right => app.seek_forward(),
                        KeyCode::Char('c') => {
                            app.input.clear();
                            app.search_results.clear();
                            app.selected_song_index =
                                if app.playlist.is_empty() { None } else { Some(0) };
                        }
                        _ => {}
                    },
                    InputMode::Editing => match key.code {
                        KeyCode::Enter => {
                            app.search_results = app
                                .playlist
                                .iter()
                                .filter(|p| {
                                    p.to_str()
                                        .unwrap_or("")
                                        .to_lowercase()
                                        .contains(&app.input.to_lowercase())
                                })
                                .cloned()
                                .collect();
                            app.selected_song_index = app.search_results.get(0).map(|_| 0);
                            app.input_mode = InputMode::Normal;
                        }
                        KeyCode::Char(c) => app.input.push(c),
                        KeyCode::Backspace => {
                            app.input.pop();
                        }
                        KeyCode::Esc => app.input_mode = InputMode::Normal,
                        _ => {}
                    },
                }
            }
        }
    }
}

