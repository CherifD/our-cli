mod agent;
mod app;
mod chat;
mod memory;
mod output;

pub fn run() -> anyhow::Result<()> {
    app::run()
}
