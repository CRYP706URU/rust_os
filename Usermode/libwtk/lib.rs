//
//
//
//! Tifflin window toolkit
#![feature(zero_one)]
#![feature(const_fn)]
#![cfg_attr(nightly, feature(unboxed_closures,fn_traits))]

extern crate async;
extern crate byteorder;

#[macro_use]
extern crate macros;

#[macro_use]
extern crate syscalls;

pub mod geom;
pub mod surface;

mod window;
mod layout;
mod static_layout;
mod input;
mod text;

pub mod menu;

pub mod image;
pub mod scrollbar;

pub mod decorator;

pub use surface::Colour;

/// Re-export GUI events for users of the library
pub use syscalls::gui::Event as InputEvent;
pub use syscalls::gui::KeyCode as KeyCode;
pub use window::Modifier as ModifierKey;

/// Common trait for window elements
pub trait Element
{
	/// Called when focus changes to/from this element
	fn focus_change(&self, _have: bool) {
	}
	/// Called when an event fires. Keyboard events are controlled by focus, mouse via the render tree
	fn handle_event(&self, _ev: ::InputEvent, _win: &mut ::window::WindowTrait) -> bool {
		false
	}

	/// Redraw this element into the provided surface view
	// MEMO: Cannot take &mut, because that requires `root: &mut` in Window, which precludes passing &mut Window to Element::handle_event
	fn render(&self, surface: ::surface::SurfaceView, force: bool);

	/// Update size-based information (should be called before a render with a new size, and may be expensive)
	fn resize(&self, _w: u32, _h: u32);

	/// Fetch child element at the given position.
	/// Returns the child element and the offset of the child.
	fn element_at_pos(&self, x: u32, y: u32) -> (&::Element, (u32,u32)); //{ (self, (0,0)) }
}
/// Object safe
impl<'a, T: 'a + Element> Element for &'a T
{
	fn focus_change(&self, have: bool) { (*self).focus_change(have) }
	fn handle_event(&self, ev: ::InputEvent, win: &mut ::window::WindowTrait) -> bool { (*self).handle_event(ev, win) }

	fn render(&self, surface: ::surface::SurfaceView, force: bool) { (*self).render(surface, force) }
	fn resize(&self, w: u32, h: u32) { (*self).resize(w, h) }
	fn element_at_pos(&self, x: u32, y: u32) -> (&::Element,(u32,u32)) { (*self).element_at_pos(x,y) }
}
/// Unit type is a valid element. Just does nothing.
impl Element for ()
{
	fn focus_change(&self, _have: bool) { }
	fn handle_event(&self, _ev: ::InputEvent, _win: &mut ::window::WindowTrait) -> bool { false }
	fn render(&self, _surface: ::surface::SurfaceView, _force: bool) { }
	fn resize(&self, _w: u32, _h: u32) { }
	fn element_at_pos(&self, _x: u32, _y: u32) -> (&::Element,(u32,u32)) { (self,(0,0)) }
}

impl Element for Colour
{
	fn resize(&self, _w: u32, _u: u32) {}
	fn render(&self, surface: ::surface::SurfaceView, force: bool) {
		if force {
			surface.fill_rect(geom::Rect::new(0,0,!0,!0), *self);
		}
	}
	fn element_at_pos(&self, _x: u32, _y: u32) -> (&::Element,(u32,u32)) { (self,(0,0)) }
}

pub use window::{Window, WindowTrait};
pub use layout::{Frame,Box};
pub use input::text_box::TextInput;
pub use input::button::{Button, ButtonBcb};
pub use image::Image;

pub use static_layout::Box as StaticBox;
pub use static_layout::BoxEle;

pub use scrollbar::Widget as Scrollbar;
pub type ScrollbarV = scrollbar::Widget<scrollbar::Vertical>;
pub type ScrollbarH = scrollbar::Widget<scrollbar::Horizontal>;

pub use text::Label;

pub fn initialise()
{
	use syscalls::Object;
	use syscalls::threads::{S_THIS_PROCESS,ThisProcessWaits};
	::syscalls::threads::wait(&mut [S_THIS_PROCESS.get_wait(ThisProcessWaits::new().recv_obj())], !0);
	::syscalls::gui::set_group( S_THIS_PROCESS.receive_object::<::syscalls::gui::Group>(0).unwrap() );
}
