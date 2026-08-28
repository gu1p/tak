mod framing;
mod model;
mod output;
mod output_io;
mod reduction;
mod render;
mod render_selection;
mod render_style;
mod render_text;
mod state_types;
mod terminal;

pub(in crate::cli) use output::RunVisualizationObserver;

#[cfg(test)]
mod tests;
