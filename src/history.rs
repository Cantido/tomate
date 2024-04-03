use std::fs::read_to_string;
use std::path::PathBuf;

use anyhow::{Context, Result};
use prettytable::{color, format, Attr, Cell, Row, Table};
use serde::{Deserialize, Serialize};

use crate::Pomodoro;
use crate::time::TimeDeltaExt;

#[derive(Debug, Deserialize, Serialize)]
pub struct History {
    pomodoros: Vec<Pomodoro>,
}

impl History {
    pub fn load(path: &PathBuf) -> Result<Self> {
        let history_str = read_to_string(path)
            .with_context(|| "Failed to read history file")?;
        toml::from_str(&history_str)
            .with_context(|| "Failed to parse history file")
    }

    pub fn print_std(&self) {
        let mut table = Table::new();

        table.set_titles(Row::new(vec![
            Cell::new("Date Started").with_style(Attr::Underline(true)),
            Cell::new("Duration").with_style(Attr::Underline(true)),
            Cell::new("Tags").with_style(Attr::Underline(true)),
            Cell::new("Description").with_style(Attr::Underline(true)),
        ]));

        for pom in self.pomodoros.iter() {
            let date = pom.timer().starts_at().format("%d %b %R").to_string();
            let dur = &pom.timer().duration().to_human();
            let tags = pom.tags().clone().unwrap_or(&["-".to_string()]).join(",");
            let desc = pom.description().clone().unwrap_or("-");

            table.add_row(Row::new(vec![
                Cell::new(&date).with_style(Attr::ForegroundColor(color::BLUE)),
                Cell::new(&dur)
                    .style_spec("r")
                    .with_style(Attr::ForegroundColor(color::CYAN)),
                Cell::new(&tags),
                Cell::new(&desc),
            ]));
        }
        table.set_format(*format::consts::FORMAT_CLEAN);
        table.printstd();
    }
}

