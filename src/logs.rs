use std::{
    fs::{File, OpenOptions},
    io::Write,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use eframe::egui::{self, Align2, Color32};
use time::{OffsetDateTime, UtcOffset, macros::format_description};

const LOG_OVERLAY_ERROR_WARN_LIFETIME: Duration = Duration::from_secs(5);
const LOG_OVERLAY_DEFAULT_LIFETIME: Duration = Duration::from_secs(2);

pub type LogQueue = Arc<Mutex<Vec<LogOverlayEntry>>>;

#[derive(Clone)]
pub struct LogOverlayEntry {
    level: log::Level,
    text: String,
    created_at: Instant,
}

struct OverlayLogger {
    entries: LogQueue,
    log_file: Mutex<File>,
}

const APP_LEVEL: log::Level = if cfg!(debug_assertions) { log::Level::Trace } else { log::Level::Info };

fn timestamp_string() -> String {
    let local_offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    let now_local = OffsetDateTime::now_utc().to_offset(local_offset);
    let fmt = format_description!(
        "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"
    );

    now_local
        .format(&fmt)
        .unwrap_or_else(|_| "1970-01-01 00:00:00.000".to_string())
}

fn crate_name_for_record(record: &log::Record) -> String {
    if let Some(module_path) = record.module_path() {
        return module_path
            .split("::")
            .next()
            .unwrap_or(module_path)
            .to_string();
    }

    let target = record.target();
    target.split("::").next().unwrap_or(target).to_string()
}

impl log::Log for OverlayLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        let target = metadata.target();
        let filter = if target.starts_with(env!("CARGO_CRATE_NAME")) { APP_LEVEL } else { log::Level::Warn };
        filter >= metadata.level()
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let timestamp = timestamp_string();
        let crate_name = crate_name_for_record(record);

        if let Ok(mut log_file) = self.log_file.lock() {
            let _ = writeln!(
                log_file,
                "{} [{}] [{}] {}",
                timestamp,
                record.level(),
                crate_name,
                record.args()
            );
        }

        let mut entries = self.entries.lock().expect("Log overlay mutex poisoned");
        entries.push(LogOverlayEntry {
            level: record.level(),
            text: format!("{}", record.args()),
            created_at: Instant::now(),
        });
    }

    fn flush(&self) {
        if let Ok(mut log_file) = self.log_file.lock() {
            let _ = log_file.flush();
        }
    }
}

pub fn init() -> LogQueue {
    let entries = Arc::new(Mutex::new(Vec::new()));
    let log_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("retrotex.log")
        .expect("Failed to open retrotex.log");

    log::set_boxed_logger(Box::new(OverlayLogger {
        entries: entries.clone(),
        log_file: Mutex::new(log_file),
    }))
    .expect("Failed to initialize logger");
    log::set_max_level(APP_LEVEL.to_level_filter());

    entries
}

fn icon_for_level(level: log::Level) -> egui::ImageSource<'static> {
    match level {
        log::Level::Error => egui::include_image!("../assets/ui/log_error.svg"),
        log::Level::Warn => egui::include_image!("../assets/ui/log_warning.svg"),
        log::Level::Info => egui::include_image!("../assets/ui/log_info.svg"),
        log::Level::Debug => egui::include_image!("../assets/ui/log_debug.svg"),
        log::Level::Trace => egui::include_image!("../assets/ui/log_trace.svg"),
    }
}

fn level_color(level: log::Level) -> Color32 {
    match level {
        log::Level::Error => Color32::from_rgb(250, 95, 85),
        log::Level::Warn => Color32::from_rgb(250, 185, 70),
        log::Level::Info => Color32::from_rgb(180, 220, 255),
        log::Level::Debug => Color32::from_rgb(190, 255, 180),
        log::Level::Trace => Color32::from_rgb(175, 175, 175),
    }
}

fn entry_lifetime(level: log::Level) -> Duration {
    match level {
        log::Level::Error | log::Level::Warn => LOG_OVERLAY_ERROR_WARN_LIFETIME,
        _ => LOG_OVERLAY_DEFAULT_LIFETIME,
    }
}

pub struct LogOverlay {
    queue: Arc<Mutex<Vec<LogOverlayEntry>>>,
    entries: Vec<LogOverlayEntry>,
}

impl LogOverlay {
    pub fn new(queue: Arc<Mutex<Vec<LogOverlayEntry>>>) -> Self {
        Self { queue, entries: Vec::new() }
    }

    pub fn num_entries(&self) -> usize {
        let entries = self.queue.lock().expect("Log overlay mutex poisoned");
        entries.len()
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        let now = Instant::now();

        {
            // Lock as briefly as possible. We'll deadlock if we log from this thread during that time
            let mut q = self.queue.lock().expect("Log overlay mutex poisoned");
            self.entries.append(&mut q);
        }

        self.entries.retain(| e | {
            let lifetime = entry_lifetime(e.level);
            now.duration_since(e.created_at) < lifetime
        });

        if !self.entries.is_empty() {
            egui::Area::new("log_overlay".into())
                .order(egui::Order::Foreground)
                .anchor(Align2::LEFT_BOTTOM, egui::vec2(12.0, -12.0))
                .interactable(false)
                .show(ctx, |ui| {
                    let background = ui.visuals().window_fill().to_opaque();
                    let background = Color32::from_rgba_unmultiplied(background.r(), background.g(), background.b(), 140);

                    egui::Frame::new()
                        .fill(background)
                        .corner_radius(6.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.set_max_width(500.0);
                            ui.spacing_mut().item_spacing.y = 2.0;
                            for entry in self.entries.iter() {
                                let color = level_color(entry.level);
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::Image::new(icon_for_level(entry.level))
                                            .fit_to_exact_size(egui::vec2(14.0, 14.0))
                                            .tint(color),
                                    );
                                    if matches!(entry.level, log::Level::Error | log::Level::Warn) {
                                        ui.colored_label(color, &entry.text);
                                    } else {
                                        ui.label(&entry.text);
                                    }
                                });
                            }
                        });
                });
        }
    }
}
