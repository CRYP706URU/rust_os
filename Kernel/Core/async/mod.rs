// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/async/mod.rs
///! Asynchronous IO and waiting support
use _common::*;
use core::atomic::{AtomicBool,ATOMIC_BOOL_INIT};

pub use self::mutex::Mutex;
pub use self::timer::Timer;

pub mod mutex;
pub mod timer;

pub mod events
{
	pub type EventMask = u32;
}

/// A general-purpose wait event (when flag is set, waiters will be informed)
pub struct EventSource
{
	flag: AtomicBool,
	waiter: ::sync::mutex::Mutex<Option<::threads::SleepObjectRef>>
}

pub enum Waiter<'a>
{
	None,
	Event(EventWait<'a>),
	Poll(Option< PollCb<'a> >),
}

type PollCb<'a> = Box<for<'r> FnMut(Option<&'r mut Waiter<'a>>) -> bool + Send + 'a>;

type EventCb<'a> = Box<for<'r> ::lib::thunk::Invoke<(&'r mut Waiter<'a>),()> + Send + 'a>;
struct EventWait<'a>
{
	source: Option<&'a EventSource>,
	callback: Option<EventCb<'a>>,
}


/// A handle returned by a read operation (re-borrows the target buffer)
pub struct ReadHandle<'buf,'src>
{
	buffer: &'buf [u8],
	waiter: EventWait<'src>,
}

/// A handle returned by a read operation (re-borrows the target buffer)
pub struct WriteHandle<'buf,'src>
{
	buffer: &'buf [u8],
	waiter: EventWait<'src>,
}

pub enum WaitError
{
	Timeout,
}

static s_event_none: EventSource = EventSource { flag: ATOMIC_BOOL_INIT, waiter: mutex_init!(None) };

impl EventSource
{
	pub fn new() -> EventSource
	{
		EventSource {
			flag: ATOMIC_BOOL_INIT,
			waiter: ::sync::mutex::Mutex::new(None),
		}
	}
	pub fn wait_on<'a, F: FnOnce(&mut EventWait) + Send + 'a>(&'a self, f: F) -> Waiter<'a>
	{
		Waiter::event(self, f)
	}
	pub fn trigger(&self)
	{
		self.flag.store(true, ::core::atomic::Ordering::Relaxed);
		self.waiter.lock().as_mut().map(|r| r.signal());
	}
}

impl<'a> Waiter<'a>
{
	pub fn none() -> Waiter<'a>
	{
		Waiter::None
	}
	pub fn event<'b, F: FnOnce(&mut EventWait) + Send + 'b>(src: &'b EventSource, f: F) -> Waiter<'b>
	{
		Waiter::Event( EventWait {
			source: Some(src),
			callback: Some(box f as EventCb),
			} )
	}
	pub fn poll<F: FnMut(Option<&mut Waiter<'a>>)->bool + Send + 'a>(f: F) -> Waiter<'a>
	{
		Waiter::Poll( Some(box f) )
	}

	pub fn is_valid(&self) -> bool
	{
		match *self
		{
		Waiter::None => true,
		Waiter::Event(ref i) => i.callback.is_some(),
		Waiter::Poll(ref c) => c.is_some(),
		}
	}
	pub fn is_ready(&self) -> bool
	{
		match *self
		{
		Waiter::None => true,
		Waiter::Event(ref i) => match i.source
			{
			Some(r) => r.flag.load(::core::atomic::Ordering::Relaxed),
			None => true
			},
		Waiter::Poll(ref c) => match *c
			{
			Some(ref cb) => cb(None),
			None => true,
			},
		}
	}
	
	/// Returns false if binding was impossible
	pub fn bind_signal(&mut self, sleeper: &mut ::threads::SleepObject) -> bool
	{
		match *self
		{
		Waiter::None => true,
		Waiter::Event(ref i) => {
			match i.source
			{
			Some(r) => { *r.waiter.lock() = Some(sleeper.get_ref()) },
			None => {},
			}
			true
			},
		Waiter::Poll(ref c) => false,
		}
	}
	
	pub fn run_completion(&mut self)
	{
		match *self
		{
		Waiter::None => {
			},
		Waiter::Event(ref i) => {
			i.callback.take().expect("EventWait::run_completion with callback None").invoke(self);
			},
		Waiter::Poll(ref callback) => {
			callback.take().expect("Wait::run_completion with Poll callback None")(Some(self));
			}
		}
	}
	
	//// Call the provided function after the original callback
	//pub fn chain<F: FnOnce(&mut EventWait) + Send + 'a>(mut self, f: F) -> EventWait<'a>
	//{
	//	let cb = self.callback.take().unwrap();
	//	let newcb = box move |e: &mut EventWait<'a>| { cb.invoke(e); f(e); };
	//	EventWait {
	//		callback: Some(newcb),
	//		source: self.source,
	//	}
	//}
}

impl<'o_b,'o_e> ReadHandle<'o_b, 'o_e>
{
	pub fn new<'b,'e>(dst: &'b [u8], w: EventWait<'e>) -> ReadHandle<'b,'e>
	{
		ReadHandle {
			buffer: dst,
			waiter: w,
		}
	}
}

impl<'o_b,'o_e> WriteHandle<'o_b, 'o_e>
{
	pub fn new<'b,'e>(dst: &'b [u8], w: EventWait<'e>) -> WriteHandle<'b,'e>
	{
		WriteHandle {
			buffer: dst,
			waiter: w,
		}
	}
}

// Note - List itself isn't modified, but needs to be &mut to get &mut to inners
pub fn wait_on_list(waiters: &mut [&mut Waiter])
{
	if waiters.len() == 0
	{
		panic!("wait_on_list - Nothing to wait on");
	}
	//else if waiters.len() == 1
	//{
	//	// Only one item to wait on, explicitly wait
	//	waiters[0].wait()
	//}
	else
	{
		// Multiple waiters
		// - Create an object for them to signal
		let mut obj = ::threads::SleepObject::new("wait_on_list");
		for ent in waiters.iter_mut()
		{
			ent.bind_signal( &mut obj );
		}
		
		// - Wait the current thread on that object
		obj.wait();
		
		// - When woken, run completion handlers on all completed waiters
		for ent in waiters.iter_mut()
		{
			if ent.is_ready()
			{
				ent.run_completion();
			}
		}
	}
}

