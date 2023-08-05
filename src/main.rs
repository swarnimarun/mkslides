use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyEventState},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::{
    io::{self, Stdout},
    time::Duration,
};
mod slide;
use slide::{mkslides, render_slide, Slides};

use anyhow::{Context, Result};

fn main() -> Result<()> {
    let mdfile = std::env::args()
        .nth(1)
        .expect("please provide a markdown file to render as slides");
    let mut terminal = setup_terminal()?;
    run(mkslides(mdfile)?, &mut terminal)?;
    restore_terminal(&mut terminal)?;
    Ok(())
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
    Ok(terminal.show_cursor()?)
}

fn run(mut slides: Slides, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    Ok(loop {
        terminal.draw(render_slide(
            slides.current().context("slides current failes")?,
        ))?;
        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match (key.code, key.kind) {
                    (KeyCode::Char('q'), KeyEventKind::Release) => break,
                    (KeyCode::Char('h'), KeyEventKind::Release) => slides.prev(),
                    (KeyCode::Char('l'), KeyEventKind::Release) => slides.next(),
                    _ => {}
                }
            }
        }
    })
}
