use clap::Parser;

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum Orientation {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum Widget {
    Minimap,
}

#[derive(Parser, Debug)]
#[clap(author, version, about)]
pub struct Config {
    #[arg(default_value = "minimap")]
    pub widget: Widget,
    /// Only display windows for this output (niri msg outputs to list outputs)
    #[arg(short, long, default_value = None)]
    pub output: Option<Vec<String>>,
    /// Target window height in pixels
    #[arg(short('H'), long, default_value_t = 46)]
    pub height: u64,
    /// <Not yet implemented> Bar orientation
    #[arg(short('O'), long, default_value = "vertical")]
    pub orientation: Orientation,
    /// Factor at which window should be stretched/squished along the bars direction
    #[arg(short, long, default_value_t = 1.25)]
    pub aspect_ratio: f64,
}
