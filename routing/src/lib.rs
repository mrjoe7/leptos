#![no_std]

#[macro_use]
extern crate alloc;

//mod generate_route_list;
pub mod location;
pub mod matching;
pub mod params;
mod path_segment;
pub use path_segment::*;
//pub mod matching;
//cfg(feature = "reaccy")]
//pub mod reactive;
//mod render_mode;
//pub mod route;
//pub mod router;
//mod static_render;
//pub use generate_route_list::*;
//pub use render_mode::*;
