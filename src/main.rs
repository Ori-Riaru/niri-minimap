mod config;
mod minimap;

use clap::Parser;
use config::{Config, Widget};
use minimap::Minimap;

fn main() {
    let config = Config::parse();

    match config.widget {
        Widget::Minimap => {
            let mut minimap = Minimap::new(config);
            minimap.track_events();
        }
    }
}
