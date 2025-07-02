use iced::{
    executor, Application, Command, Element, Length, Settings, Subscription,
    theme,
    widget::{button, column, row, text, slider},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::thread;
use std::time::Duration;
use enigo::{Enigo, MouseControllable, MouseButton};
use rdev::{listen, EventType, Key};



const DEFAULT_DELAY_MS: u32 = 800;

fn main() -> iced::Result {
    let clicking_flag = Arc::new(AtomicBool::new(false));
    let delay_ms = Arc::new(AtomicUsize::new(DEFAULT_DELAY_MS as usize));

    spawn_clicker_loop(clicking_flag.clone(), delay_ms.clone());

    let hotkey_flag = clicking_flag.clone();
    let delay_for_hotkey = delay_ms.clone();

    thread::spawn(move || {
        if let Err(err) = hotkey_listener(hotkey_flag, delay_for_hotkey) {
            println!("Hotkey listener error: {:?}", err);
        }
    });

    AutoClickerApp::run(Settings {
        flags: (clicking_flag, delay_ms),
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

                // Optionally move mouse away
                // enigo.mouse_move_to(2000, 1000);

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


fn hotkey_listener(flag: Arc<AtomicBool>, _delay: Arc<AtomicUsize>) -> Result<(), rdev::ListenError> {
    listen(move |event| {
        if let EventType::KeyPress(key) = event.event_type {
            if key == Key::F6 {
                toggle_clicker(flag.clone());
            }
        }
    })
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
}



struct AutoClickerApp {
    is_clicking: Arc<AtomicBool>,
    delay_ms: Arc<AtomicUsize>,
    slider_value: u32,
    is_toggling: bool,
    last_toggle: std::time::Instant,
}

impl Application for AutoClickerApp {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = theme::Theme;
    type Flags = (Arc<AtomicBool>, Arc<AtomicUsize>);

fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
    (
        Self {
            is_clicking: flags.0,
            delay_ms: flags.1,
            slider_value: DEFAULT_DELAY_MS,
            is_toggling: true,
            last_toggle: std::time::Instant::now() - std::time::Duration::from_secs(1),
        },
        Command::none()
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

    let start_stop_row = row![
        start_button,
        stop_button
    ]
    .spacing(20);

    column![
        text(label),
        text(format!("Delay: {} ms", self.slider_value)),
        slider(10..=1000, self.slider_value, Message::SliderChanged)
            .step(10u32),
        start_stop_row,
    ]
    .spacing(20)
    .padding(20)
    .width(Length::Shrink)
    .into()
}


}
