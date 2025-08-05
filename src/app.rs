// app.txt
use crossterm::event::{self, Event, KeyCode};
use ratatui::prelude::*;
use ringbuf::Consumer;
use rodio::{Decoder, OutputStream, Sink, Source};
use std::{
    error::Error,
    fs,
    io::{self, BufReader},
    path::{Path, PathBuf},
    sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex},
    thread,
    time::Duration,
};
use crate::ui::ui;

struct RingBufferSource {
    consumer: Consumer<f32, Arc<ringbuf::HeapRb<f32>>>,
    channels: u16,
    sample_rate: u32,
}

impl RingBufferSource {
    fn new(consumer: Consumer<f32, Arc<ringbuf::HeapRb<f32>>>, channels: u16, sample_rate: u32) -> Self {
        Self { consumer, channels, sample_rate }
    }
}

impl Iterator for RingBufferSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        self.consumer.pop()
    }
}

impl Source for RingBufferSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

pub enum InputMode {
    Normal,
    Editing,
}

pub struct App {
    pub input: String,
    pub input_mode: InputMode,
    pub playlist: Vec<PathBuf>,
    pub search_results: Vec<PathBuf>,
    pub _stream: Option<OutputStream>,
    pub sink: Option<Sink>,
    pub current_song_path: Option<PathBuf>,
    pub selected_song_index: Option<usize>,
    pub is_playing: bool,
    pub spectrogram_data: Arc<Mutex<Vec<f32>>>,
    pub audio_thread_handle: Option<thread::JoinHandle<()>>,
    pub stop_audio_thread: Arc<AtomicBool>,
}

impl App {
    pub fn new() -> Result<App, Box<dyn Error>> {
        let (_stream, stream_handle) = match OutputStream::try_default() {
            Ok((stream, handle)) => (Some(stream), Some(handle)),
            Err(_) => (None, None),
        };
        let sink = stream_handle.as_ref().map(|h| Sink::try_new(h).unwrap());
        let playlist = Self::load_playlist("music")?;
        let selected_song_index = if playlist.is_empty() { None } else { Some(0) };
        let spectrogram_data = Arc::new(Mutex::new(vec![0.0; 512]));

        Ok(App {
            input: String::new(),
            input_mode: InputMode::Normal,
            playlist,
            search_results: vec![],
            _stream,
            sink,
            current_song_path: None,
            selected_song_index,
            is_playing: false,
            spectrogram_data,
            audio_thread_handle: None,
            stop_audio_thread: Arc::new(AtomicBool::new(false)),
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
        if let Some(sink) = &self.sink {
            if sink.is_paused() {
                sink.play();
                self.is_playing = true;
            } else {
                sink.pause();
                self.is_playing = false;
            }
        }
    }

    pub fn is_seekable(&self) -> bool {
        self.sink.as_ref().map_or(false, |s| !s.empty())
    }

    pub fn seek_forward(&mut self) {
        if self.is_seekable() {
            if let Some(sink) = &self.sink {
                let current_pos = sink.get_pos();
                let new_pos = current_pos + Duration::from_secs(5);
                if let Err(e) = sink.try_seek(new_pos) {
                    if !e.to_string().contains("end of stream") {
                        eprintln!("Error seeking forward: {}", e);
                    }
                }
            }
        }
    }

    pub fn seek_backward(&mut self) {
        if self.is_seekable() {
            if let Some(sink) = &self.sink {
                let current_pos = sink.get_pos();
                let new_pos = if current_pos > Duration::from_secs(5) {
                    current_pos - Duration::from_secs(5)
                } else {
                    Duration::ZERO
                };
                if let Err(e) = sink.try_seek(new_pos) {
                    if !e.to_string().contains("end of stream") {
                        eprintln!("Error seeking backward: {}", e);
                    }
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
        if let Some(sink) = &self.sink {
            sink.stop();
            sink.clear();

            self.stop_audio_thread.store(true, Ordering::SeqCst);
            if let Some(handle) = self.audio_thread_handle.take() {
                handle.join().unwrap();
            }
            self.stop_audio_thread.store(false, Ordering::SeqCst);

            let file = BufReader::new(fs::File::open(path)?);
            let source = Decoder::new(file)?;
            let channels = source.channels();
            let sample_rate = source.sample_rate();

            let playback_rb = ringbuf::HeapRb::<f32>::new(sample_rate as usize * 5);
            let (mut playback_prod, playback_cons) = playback_rb.split();

            let spectrogram_rb = ringbuf::HeapRb::<f32>::new(sample_rate as usize * 5);
            let (mut spectrogram_prod, mut spectrogram_cons) = spectrogram_rb.split();

            let stop_audio_thread = self.stop_audio_thread.clone();
            let audio_thread_handle = thread::spawn(move || {
                let mut source = source.convert_samples::<f32>();
                while !stop_audio_thread.load(Ordering::SeqCst) {
                    if let Some(sample) = source.next() {
                        while playback_prod.is_full() {
                            thread::sleep(Duration::from_millis(1));
                        }
                        let _ = playback_prod.push(sample);
                        let _ = spectrogram_prod.push(sample);
                    } else {
                        break;
                    }
                }
            });
            self.audio_thread_handle = Some(audio_thread_handle);

            let spectrogram_data = self.spectrogram_data.clone();
            thread::spawn(move || {
                let fft_size = 1024;
                let window = apodize::hanning_iter(fft_size).map(|f| f as f32).collect::<Vec<_>>();
                let mut planner = rustfft::FftPlanner::new();
                let fft = planner.plan_fft_forward(fft_size);
                let mut buffer: Vec<f32> = Vec::with_capacity(fft_size);

                loop {
                    // Collect samples at a fixed rate regardless of UI updates
                    while buffer.len() < fft_size && spectrogram_cons.len() > 0 {
                        if let Some(sample) = spectrogram_cons.pop() {
                            buffer.push(sample);
                        }
                    }

                    // Process FFT when we have enough samples
                    if buffer.len() >= fft_size {
                        let mut complex_buffer: Vec<_> = buffer
                            .drain(..fft_size)
                            .zip(window.iter())
                            .map(|(s, w)| rustfft::num_complex::Complex::new(s * w, 0.0))
                            .collect();

                        fft.process(&mut complex_buffer);

                        let mut spectrogram_data = spectrogram_data.lock().unwrap();
                        *spectrogram_data = complex_buffer[..fft_size / 2]
                            .iter()
                            .map(|c| (c.norm_sqr().sqrt() * 2.0 / fft_size as f32).log10() * 20.0)
                            .map(|v| if v.is_nan() || v.is_infinite() { 0.0 } else { v })
                            .collect();
                    }
                    
                    // Consistent update rate - 30 FPS for smooth visualization
                    thread::sleep(Duration::from_millis(16));
                }
            });

            let source = RingBufferSource::new(playback_cons, channels, sample_rate);
            sink.append(source);
            sink.play();
            self.current_song_path = Some(path.to_path_buf());
            self.is_playing = true;
        }

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
}

pub fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    let tick_rate = Duration::from_millis(16); // ~60 FPS for smooth UI
    
    loop {
        // Non-blocking event poll with short timeout
        if event::poll(tick_rate)? {
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
        
        // Always redraw the UI at consistent intervals for smooth visualizer
        terminal.draw(|f| ui(f, &mut app))?;
    }
}
