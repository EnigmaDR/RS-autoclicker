use iced::{
    executor, Application, Command, Element, Length, Settings,
    theme,
    widget::{button, column, row, text, slider, PickList},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Mutex,
};
use std::thread;
use std::time::Duration;
use enigo::{Enigo, MouseControllable, MouseButton};
use rdev::{listen, EventType, Key};

// ------------------- Hotkey Enum ------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hotkey {
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
}

impl Hotkey {
    const ALL: [Hotkey; 10] = [
        Hotkey::F1,
        Hotkey::F2,
        Hotkey::F3,
        Hotkey::F4,
        Hotkey::F5,
        Hotkey::F6,
        Hotkey::F7,
        Hotkey::F8,
        Hotkey::F9,
        Hotkey::F10,
    ];

    fn to_rdev_key(self) -> Key {
        match self {
            Hotkey::F1 => Key::F1,
            Hotkey::F2 => Key::F2,
            Hotkey::F3 => Key::F3,
            Hotkey::F4 => Key::F4,
            Hotkey::F5 => Key::F5,
            Hotkey::F6 => Key::F6,
            Hotkey::F7 => Key::F7,
            Hotkey::F8 => Key::F8,
            Hotkey::F9 => Key::F9,
            Hotkey::F10 => Key::F10,
        }
    }
}

impl std::fmt::Display for Hotkey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Default for Hotkey {
    fn default() -> Self {
        Hotkey::F6
    }
}
// --------------------------------------------------------


const DEFAULT_DELAY_MS: u32 = 800;
const DEFAULT_HOTKEY: Hotkey = Hotkey::F6;

fn main() -> iced::Result {
    let clicking_flag = Arc::new(AtomicBool::new(false));
    let delay_ms = Arc::new(AtomicUsize::new(DEFAULT_DELAY_MS as usize));
    let selected_hotkey = Arc::new(Mutex::new(DEFAULT_HOTKEY));
    let listener_handle = Arc::new(Mutex::new(None));

    spawn_clicker_loop(clicking_flag.clone(), delay_ms.clone());

    let hotkey_flag = clicking_flag.clone();
    let delay_for_hotkey = delay_ms.clone();
    let hotkey_arc = selected_hotkey.clone();
    let listener_handle_arc = listener_handle.clone();

    start_hotkey_listener(
        hotkey_flag,
        delay_for_hotkey,
        hotkey_arc,
        listener_handle_arc,
    );

    AutoClickerApp::run(Settings {
        flags: (clicking_flag, delay_ms, selected_hotkey, listener_handle),
        ..Default::default()
    })
}

fn spawn_clicker_loop(flag: Arc<AtomicBool>, delay: Arc<AtomicUsize>) {
    thread::spawn(move || {
        let mut enigo = Enigo::new();
        let mut last_time = std::time::Instant::now();
        loop {
            if flag.load(Ordering::Relaxed) {
                println!("Starting auto-clicker in 500ms delay...");
                std::thread::sleep(Duration::from_millis(1000));

                while flag.load(Ordering::Relaxed) {
                    enigo.mouse_click(MouseButton::Left);
                    println!("[AutoClicker] Clicked!");
                    let now = std::time::Instant::now();
                    let elapsed = now.duration_since(last_time);
                    println!(
                        "[AutoClicker] time since last click: {:?}",
                        elapsed
                    );
                    last_time = now;
                    let sleep_time = delay.load(Ordering::Relaxed);
                    std::thread::sleep(Duration::from_millis(sleep_time as u64));
                }
                println!("CLICKER THREAD STOPPED.");
            }

            std::thread::sleep(Duration::from_millis(50));
        }
    });
}

fn start_hotkey_listener(
    flag: Arc<AtomicBool>,
    delay: Arc<AtomicUsize>,
    selected_hotkey: Arc<Mutex<Hotkey>>,
    listener_handle_arc: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
) {
    let handle = thread::spawn(move || {
        println!("Hotkey listener started.");
        let hotkey = *selected_hotkey.lock().unwrap();

        if let Err(e) = listen(move |event| {
            if let EventType::KeyPress(key) = event.event_type {
                if key == hotkey.to_rdev_key() {
                    toggle_clicker(flag.clone());
                }
            }
        }) {
            println!("Error listening to keyboard events: {:?}", e);
        }
    });

    let mut lock = listener_handle_arc.lock().unwrap();
    if let Some(old_handle) = lock.take() {
        println!("Shutting down previous listener...");
        // Note: rdev's listener is blocking; there's no clean way to kill it except process kill
        // so we simply replace the thread handle and let the old one die if possible.
        // In a real production app you'd architect this differently!
        drop(old_handle);
    }
    *lock = Some(handle);
}

fn toggle_clicker(flag: Arc<AtomicBool>) {
    let current = flag.load(Ordering::Relaxed);
    if current {
        stop_clicker(flag);
    } else {
        start_clicker(flag);
    }
}

fn start_clicker(flag: Arc<AtomicBool>) {
    if !flag.load(Ordering::Relaxed) {
        flag.store(true, Ordering::Relaxed);
        println!("Clicker STARTED.");
    }
}

fn stop_clicker(flag: Arc<AtomicBool>) {
    if flag.load(Ordering::Relaxed) {
        flag.store(false, Ordering::Relaxed);
        println!("Clicker STOPPED.");
    }
}

#[derive(Debug, Clone)]
enum Message {
    StartClicker,
    StopClicker,
    SliderChanged(u32),
    HotkeyChanged(Hotkey),
}

struct AutoClickerApp {
    is_clicking: Arc<AtomicBool>,
    delay_ms: Arc<AtomicUsize>,
    slider_value: u32,
    is_toggling: bool,
    last_toggle: std::time::Instant,
    selected_hotkey: Arc<Mutex<Hotkey>>,
    listener_handle: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
}

impl Application for AutoClickerApp {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = theme::Theme;
    type Flags = (
        Arc<AtomicBool>,
        Arc<AtomicUsize>,
        Arc<Mutex<Hotkey>>,
        Arc<Mutex<Option<thread::JoinHandle<()>>>>,
    );

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Self {
                is_clicking: flags.0,
                delay_ms: flags.1,
                slider_value: DEFAULT_DELAY_MS,
                is_toggling: true,
                last_toggle: std::time::Instant::now() - std::time::Duration::from_secs(1),
                selected_hotkey: flags.2,
                listener_handle: flags.3,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Rust Auto Clicker")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        println!("Received message: {:?}", message);

        match message {
            Message::StartClicker => {
                if !self.is_clicking.load(Ordering::Relaxed) {
                    self.is_clicking.store(true, Ordering::Relaxed);
                    println!("Clicker STARTED.");
                }
            }
            Message::StopClicker => {
                if self.is_clicking.load(Ordering::Relaxed) {
                    self.is_clicking.store(false, Ordering::Relaxed);
                    println!("Clicker STOPPED.");
                }
            }
            Message::SliderChanged(value) => {
                self.slider_value = value;
                self.delay_ms.store(value as usize, Ordering::Relaxed);
                println!("Delay updated to {} ms", value);
            }
            Message::HotkeyChanged(hotkey) => {
                {
                    let mut lock = self.selected_hotkey.lock().unwrap();
                    *lock = hotkey;
                }
                println!("Hotkey changed to {:?}", hotkey);

                start_hotkey_listener(
                    self.is_clicking.clone(),
                    self.delay_ms.clone(),
                    self.selected_hotkey.clone(),
                    self.listener_handle.clone(),
                );
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<Self::Message> {
        let label = if self.is_clicking.load(Ordering::Relaxed) {
            "Auto Clicker is RUNNING"
        } else {
            "Auto Clicker is STOPPED"
        };

        let start_button = if self.is_clicking.load(Ordering::Relaxed) {
            button("Start")
        } else {
            button("Start").on_press(Message::StartClicker)
        };

        let stop_button = if self.is_clicking.load(Ordering::Relaxed) {
            button("Stop").on_press(Message::StopClicker)
        } else {
            button("Stop")
        };

        let hotkey_picklist = PickList::new(
            &Hotkey::ALL[..],
            Some(*self.selected_hotkey.lock().unwrap()),
            Message::HotkeyChanged,
        )
        .placeholder("Select Hotkey");

        let start_stop_row = row![start_button, stop_button].spacing(20);

        column![
            text(label),
            text(format!("Delay: {} ms", self.slider_value)),
            slider(10..=1000, self.slider_value, Message::SliderChanged).step(10u32),
            hotkey_picklist,
            start_stop_row,
        ]
        .spacing(20)
        .padding(20)
        .width(Length::Shrink)
        .into()
    }
}
