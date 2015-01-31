// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Modules/video_vgs/mod.rs
// - VGA (and derivative) device driver
//
#![no_std]
#![feature(box_syntax)]
use kernel::_common::*;
#[macro_use] extern crate kernel;
#[macro_use] extern crate core;
mod std {
	pub use core::fmt;
}

use kernel::metadevs::video::{Framebuffer,Rect};
use kernel::device_manager;
use kernel::metadevs::video;

module_define!{VGA, [DeviceManager, Video], init}

mod crtc;

/// Driver object for the PCI
struct VgaPciDriver;

struct VgaDevice
{
	/// Handle to video metadev registration
	_video_handle: video::FramebufferRegistration,
}
/**
 * Real device instance (registered with the video manager)
 */
struct VgaFramebuffer
{
	io_base: u16,
	window: ::kernel::memory::virt::AllocHandle,
	crtc: crtc::CrtcRegs,
	w: u16,
	h: u16,
}
struct CrtcAttrs
{
	frequency: u16,
	h_front_porch: u16,
	h_active: u16,
	h_back_porch: u16,
	h_sync_len: u16,
	
	v_front_porch: u16,
	v_active: u16,
	v_back_porch: u16,
	v_sync_len: u16,
}

#[allow(non_upper_case_globals)]
static s_vga_pci_driver: VgaPciDriver = VgaPciDriver;
#[allow(non_upper_case_globals)]
static s_legacy_bound: ::core::atomic::AtomicBool = ::core::atomic::ATOMIC_BOOL_INIT;

fn init()
{
	// 1. Register Driver
	device_manager::register_driver(&s_vga_pci_driver);
}

impl device_manager::Driver for VgaPciDriver
{
	fn name(&self) -> &str {
		"vga"
	}
	fn bus_type(&self) -> &str {
		"pci"
	}
	fn handles(&self, bus_dev: &device_manager::BusDevice) -> u32
	{
		let classcode = bus_dev.get_attr("class");
		// [class] [subclass] [IF] [ver]
		if classcode & 0xFFFFFF00 == 0x03000000 {
			1	// Handle as weakly as possible (vendor-provided drivers bind higher)
		}
		else {
			0
		}
	}
	fn bind(&self, _bus_dev: &device_manager::BusDevice) -> Box<device_manager::DriverInstance+'static>
	{
		if s_legacy_bound.swap(true, ::core::atomic::Ordering::AcqRel)
		{
			panic!("Duplicate binding of legacy VGA");
		}
		box VgaDevice::new(0x3B0)
	}
	
}

impl VgaDevice
{
	fn new(iobase: u16) -> VgaDevice
	{
		VgaDevice {
			_video_handle: video::add_output(box VgaFramebuffer::new(iobase) ),
		}
	}
}

impl device_manager::DriverInstance for VgaDevice
{
}

impl VgaFramebuffer
{
	fn new(base: u16) -> VgaFramebuffer
	{
		log_debug!("Creating VGA driver at base {:#3x}", base);
		let mut rv = VgaFramebuffer {
			io_base: base,
			window: ::kernel::memory::virt::map_hw_rw(0xA0000, (0xC0-0xA0), module_path!()).unwrap(),
			crtc: crtc::CrtcRegs::load(base + 0x24),	// Colour CRTC regs
			w: 320,
			h: 240,
			};
		
		rv
	}
	
	fn set_crtc(&mut self, attrs: CrtcAttrs)
	{
		use self::crtc::PIX_PER_CHAR;
		self.crtc.set_line_compare(0);	// Entire screen is Screen B
		self.crtc.set_screen_start(0);	// Screen A starts at offset 0 (making A and B functionally equal)
		self.crtc.set_byte_pan(0);	// Byte pan = 0, no horizontal offset
		
		self.crtc.set_offset(attrs.h_active * 1);	// Aka pitch (TODO: Need the BPP setting)
		
		// Horizontal
		let h_total = (attrs.h_front_porch + attrs.h_active + attrs.h_back_porch) / PIX_PER_CHAR as u16;
		self.crtc.set_h_total(h_total);
		let h_disp_end = attrs.h_active / PIX_PER_CHAR as u16;
		self.crtc.set_h_disp_end(h_disp_end);
		self.crtc.set_h_blank_start(h_disp_end+1);	// Must be larger than h_disp_end
		let h_blank_len = (attrs.h_front_porch + attrs.h_back_porch) / PIX_PER_CHAR as u16;
		self.crtc.set_h_blank_len( h_blank_len );
		let h_sync_start = (attrs.h_active + attrs.h_front_porch) / PIX_PER_CHAR as u16;
		self.crtc.set_h_sync_start(h_sync_start);
		let h_sync_end = ((h_sync_start + attrs.h_sync_len) / PIX_PER_CHAR as u16) & 31;
		self.crtc.set_h_sync_end(h_sync_end);
		
		// Vertical
		let v_total = attrs.v_front_porch + attrs.v_active + attrs.v_back_porch;
		self.crtc.set_v_total(v_total);
		let v_disp_end = attrs.v_active;
		self.crtc.set_v_disp_end(v_disp_end);
		self.crtc.set_v_blank_start(v_disp_end+1);
		let v_blank_end = attrs.v_front_porch + attrs.v_back_porch;
		self.crtc.set_v_blank_end(v_blank_end);
		let v_sync_start = attrs.v_active + attrs.v_back_porch;
		self.crtc.set_v_sync_start(v_sync_start);
		let v_sync_end = (v_sync_start + attrs.v_sync_len) & 31;
		self.crtc.set_v_sync_end(v_sync_end);
		
		// Frequency
		// - Just leave as 25MHz (Clock Select = 0)
		
		self.crtc.commit(self.io_base + 0x24);
		
		panic!("TODO: Set/check firequency {}Hz", attrs.frequency);
	}
	
	fn col32_to_u8(&self, colour: u32) -> u8
	{
		// 8:8:8 RGB -> 2:3:3 RGB
		let r8 = (colour >> 16) as u8;
		let g8 = (colour >>  8) as u8;
		let b8 = (colour >>  0) as u8;
		
		let r2 = (r8 + 0x3F) >> (8-2);
		let g3 = (g8 + 0x1F) >> (8-3);
		let b3 = (b8 + 0x1F) >> (8-3);
		return (r2 << 6) | (g3 << 3) | (b3 << 0);
	}
}

impl CrtcAttrs
{
	pub fn from_res(w: u16, h: u16, freq: u16) -> CrtcAttrs
	{
		match (w,h,freq)
		{
		(640,480,60) => CrtcAttrs {
			frequency: 60,
			h_active: 640,
			h_front_porch: 16+96,	// sync overlaps with front porch
			h_sync_len: 96,
			h_back_porch: 48,
			v_active: 480,
			v_front_porch: 10+2,
			v_sync_len: 2,
			v_back_porch: 33,
			},
		_ => {
			panic!("TODO: Obtain CRTC attributes from resolution {}x{} at {}Hz", w, h, freq);
			}
		}
	}
}

impl video::Framebuffer for VgaFramebuffer
{
	fn as_any(&self) -> &Any {
		self as &Any
	}
	fn activate(&mut self) {
		// Don't modeset yet, wait until video manager tells us to initialise
		// 320x240 @ 60Hz
		self.set_crtc(CrtcAttrs::from_res(320, 240, 60));
	}
	
	fn get_size(&self) -> Rect {
		// 320x200x 8bpp
		Rect::new( 0,0, self.w as u16, self.h as u16 )
	}
	fn set_size(&mut self, _newsize: Rect) -> bool {
		// Can't change
		false
	}
	
	fn blit_inner(&mut self, dst: Rect, src: Rect) {
		panic!("TODO: VGA blit_inner {} to {}", src, dst);
	}
	fn blit_ext(&mut self, dst: Rect, src: Rect, srf: &Framebuffer) -> bool {
		match srf.as_any().downcast_ref::<VgaFramebuffer>()
		{
		Some(_) => panic!("TODO: VGA blit_ext {} to  {}", src, dst),
		None => false,
		}
	}
	fn blit_buf(&mut self, dst: Rect, buf: &[u32]) {
		panic!("TODO: VGA blit_buf {} pixels to {}", buf.len(), dst);
	}
	fn fill(&mut self, dst: Rect, colour: u32) {
		assert!( dst.within(self.w as u16, self.h as u16) );
		let colour_val = self.col32_to_u8(colour);
		for row in dst.y .. dst.y + dst.h
		{
			let scanline = self.window.as_mut_slice::<u8>( (row * self.w) as usize, dst.w as usize);
			for col in dst.x .. dst.x + dst.w
			{
				scanline[col as usize] = colour_val;
			}
		}
	}
}

// vim: ft=rust

