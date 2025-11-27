use crate::config::Config;
use crate::cycle_state::CycleState;
use crate::window_manager::WindowManager;
use anyhow::{Context, Result};
use evdev::{Device, InputEventKind, Key};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct KeyboardListener {
    config: Config,
}

impl KeyboardListener {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Find keyboard device by looking for devices with standard keyboard keys
    fn find_keyboard_device(configured_path: Option<&str>) -> Result<Device> {
        if let Some(path_str) = configured_path {
            let path = Path::new(path_str);
            match Device::open(path) {
                Ok(device) => {
                    println!(
                        "Using configured keyboard device {} ({})",
                        device.name().unwrap_or("Unknown"),
                        path.display()
                    );
                    return Ok(device);
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to open configured keyboard device '{}': {}",
                        path_str, e
                    );
                    eprintln!("Falling back to automatic device detection...");
                }
            }
        }

        let devices_path = Path::new("/dev/input");
        for entry in std::fs::read_dir(devices_path)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(filename) = path.file_name() {
                if let Some(name) = filename.to_str() {
                    if name.starts_with("event") {
                        if let Ok(device) = Device::open(&path) {
                            if device.supported_keys().is_some_and(|keys| {
                                keys.contains(Key::KEY_TAB)
                                    || keys.contains(Key::KEY_LEFTSHIFT)
                                    || keys.contains(Key::KEY_Z)
                            }) {
                                println!(
                                    "Found keyboard device: {} ({})",
                                    device.name().unwrap_or("Unknown"),
                                    path.display()
                                );
                                return Ok(device);
                            }
                        }
                    }
                }
            }
        }

        anyhow::bail!("No keyboard device found in /dev/input")
    }

    /// Run the keyboard event listener in a background thread
    pub fn spawn(
        &self,
        wm: Arc<dyn WindowManager>,
        state: Arc<Mutex<CycleState>>,
    ) -> Result<std::thread::JoinHandle<()>> {
        if !self.config.enable_keyboard_buttons {
            anyhow::bail!("Keyboard buttons are disabled in config");
        }

        let forward_key = self.config.forward_key;
        let keyboard_device_path = self.config.keyboard_device_path.clone();

        let handle = std::thread::spawn(move || {
            match Self::run_listener(wm, state, forward_key, keyboard_device_path) {
                Ok(_) => println!("Keyboard listener stopped"),
                Err(e) => println!("Keyboard listener error: {}", e),
            }
        });

        Ok(handle)
    }

    fn run_listener(
        wm: Arc<dyn WindowManager>,
        state: Arc<Mutex<CycleState>>,
        forward_key: u16,
        keyboard_device_path: Option<String>,
    ) -> Result<()> {
        let mut device = Self::find_keyboard_device(keyboard_device_path.as_deref()).context(
            "Failed to find keyboard device. Make sure you have permission to read /dev/input/event*",
        )?;

        // DON'T grab the device - we only want to passively listen to events
        // Grabbing would prevent normal keyboard usage!

        println!("Listening for keyboard keys: forward={}", forward_key);

        loop {
            for event in device.fetch_events()? {
                if let InputEventKind::Key(key) = event.kind() {
                    let code = key.code();
                    //print(code);
                    if event.value() == 1 {
                        if code == forward_key {
                            println!("Forward key pressed");
                            if let Err(e) = Self::cycle_forward(&wm, &state) {
                                eprintln!("Failed to cycle forward: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    fn cycle_forward(wm: &Arc<dyn WindowManager>, state: &Arc<Mutex<CycleState>>) -> Result<()> {
        let mut state = state.lock().unwrap();

        // Sync with active window first
        if let Ok(active) = wm.get_active_window() {
            state.sync_with_active(active);
        }

        state.cycle_forward(&**wm)?;
        Ok(())
    }

    fn cycle_backward(wm: &Arc<dyn WindowManager>, state: &Arc<Mutex<CycleState>>) -> Result<()> {
        let mut state = state.lock().unwrap();

        // Sync with active window first
        if let Ok(active) = wm.get_active_window() {
            state.sync_with_active(active);
        }

        state.cycle_backward(&**wm)?;
        Ok(())
    }
}
