//! Print the design tokens as a CSS custom-property sheet (feature 028 — FR-030).
//!
//! Deliberately trivial: it prints what `tokens::css::stylesheet()` returns and decides nothing.
//! Every property the sheet has to hold is asserted by unit tests over that function, which is only
//! worth anything if this binary cannot introduce a property of its own. `site/build.sh` redirects
//! it into `site/theme/css/tokens.css`, which is generated on every build and committed never.
//!
//!     cargo run -p micold-core --bin micold-tokens-css > site/theme/css/tokens.css

fn main() {
    print!("{}", micold_core::tokens::css::stylesheet());
}
