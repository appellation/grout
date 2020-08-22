use std::{mem::{MaybeUninit, replace}, ops::{Deref, DerefMut}, sync::Mutex};

#[derive(Debug)]
pub struct Pool<T> {
	constructor: fn() -> T,
	buffer: Mutex<Vec<T>>,
	size: usize,
}

impl<T> Pool<T> {
	pub fn new(constructor: fn() -> T) -> Self {
		Self {
			constructor,
			buffer: Default::default(),
			size: 10,
		}
	}

	pub fn take(&self) -> Recyclable<'_, T> {
		Recyclable {
			parent: self,
			data: MaybeUninit::new(self.buffer.lock().unwrap().pop().unwrap_or_else(|| (self.constructor)())),
		}
	}
}

impl<T> Default for Pool<T>
where T: Default
{
	fn default() -> Self {
		Self::new(T::default)
	}
}

pub struct Recyclable<'a, T> {
	parent: &'a Pool<T>,
	data: MaybeUninit<T>,
}

impl<'a, T> Deref for Recyclable<'a, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		unsafe { &*self.data.as_ptr() }
	}
}

impl<'a, T> DerefMut for Recyclable<'a, T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe { &mut *self.data.as_mut_ptr() }
	}
}

impl<'a, T> Drop for Recyclable<'a, T> {
	fn drop(&mut self) {
		let data = unsafe { replace(&mut self.data, MaybeUninit::uninit()).assume_init() };
		let mut buf = self.parent.buffer.lock().unwrap();
		if buf.len() < self.parent.size {
			buf.push(data);
		}
	}
}
