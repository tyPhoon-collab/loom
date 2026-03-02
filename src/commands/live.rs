use loom::interface::app::App;
use loom::interface::tui;
use miette::Result;
use std::path::PathBuf;

pub fn handle_live(input: PathBuf, port: usize, config_status: String) -> Result<()> {
    let mut app = App::new(input, port, config_status)?;
    let mut terminal = tui::init()?;
    let res = app.run(&mut terminal);

    if let Err(e) = res {
        eprintln!("Error: {:?}", e);
    }
    Ok(())
}
