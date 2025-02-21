pub use parser::parse;
pub use templater::template;

mod ast;
mod parser;
mod splitter;
mod templater;

pub const TEMPLATE_REFERENCE_PATTERN: &str = r"\{\{([:?!@]{1})([a-zA-Z][_a-zA-Z0-9.]+)\}\}";
