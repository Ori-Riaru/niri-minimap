mod config;
mod downloads;
mod minimap;

use clap::Parser;
use config::{Config, Widget};
use downloads::Downloads;
use minimap::Minimap;

fn main() {
    let config = Config::parse();

    match config.widget {
        Widget::Minimap => {
            let mut minimap = Minimap::new(config);
            minimap.track_events();
        }
        Widget::Downloads => {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let mut downloads = Downloads::new().await.unwrap();
                downloads.run().await.unwrap();
            });
        }
    }
}
