// Android/iOS need a library entrypoint. On desktop this crate intentionally
// stays empty: main.rs is the binary entrypoint and should only compile once.
#[cfg(mobile)]
#[path = "main.rs"]
mod app;

#[cfg(mobile)]
pub use app::run;
