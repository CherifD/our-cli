mod agent;
mod app;
mod chat;
mod constants;
mod memory;
mod output;

pub fn run() -> anyhow::Result<()> {
    app::run()
}
