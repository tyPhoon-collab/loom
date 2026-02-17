use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use miette::Result;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub enum Event {
    Key(KeyEvent),
    FileChange, // Simplified: just know that something changed
    Tick,
}

pub struct EventHandler {
    receiver: mpsc::Receiver<Event>,
    // Handler must be kept alive
    #[allow(dead_code)]
    watcher: RecommendedWatcher,
}

impl EventHandler {
    pub fn new(path: PathBuf, tick_rate: Duration) -> Result<Self> {
        let (sender, receiver) = mpsc::channel();

        // 1. Input Thread
        let input_sender = sender.clone();
        thread::spawn(move || {
            loop {
                if event::poll(tick_rate).unwrap() {
                    match event::read().unwrap() {
                        CrosstermEvent::Key(key) => {
                            input_sender.send(Event::Key(key)).unwrap();
                        }
                        _ => {}
                    }
                } else {
                    input_sender.send(Event::Tick).unwrap();
                }
            }
        });

        // 2. File Watcher
        let watcher_sender = sender.clone();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            match res {
                Ok(_) => {
                    // Debouncing is handled by logic or simple delay in loop?
                    // For now, raw events.
                    let _ = watcher_sender.send(Event::FileChange);
                }
                Err(e) => eprintln!("Watch error: {:?}", e),
            }
        }).expect("Watcher failed");

        watcher.watch(&path, RecursiveMode::NonRecursive).expect("Watch failed");

        Ok(Self {
            receiver,
            watcher,
        })
    }

    pub fn next(&self) -> Result<Event> {
        Ok(self.receiver.recv().unwrap())
    }
}
