//! Equations as MathML (W3C MathML Core / Presentation MathML) via `nodalia-math`.

pub use nodalia_math::mathml::{NS, UNICODEMATH};

pub fn to_mathml(nodes: &[letra_doc::math::MNode], display: bool, linear: &str) -> String {
    let linear = if linear.is_empty() { letra_doc::math::to_linear(nodes) } else { linear.to_string() };
    linear_to_mathml(&linear, display)
}

pub fn linear_to_mathml(linear: &str, display: bool) -> String {
    let tree = nodalia_math::math::parse_linear(linear);
    nodalia_math::mathml::to_mathml(&tree, display, linear)
}

pub fn parse(html_or_math: &str) -> Option<Vec<letra_doc::math::MNode>> {
    let ml = nodalia_math::mathml::parse(html_or_math)?;
    let tree = nodalia_math::mathml::to_tree(&ml);
    let linear = nodalia_math::math::to_linear(&tree);
    Some(letra_doc::math::parse_linear(&linear))
}
