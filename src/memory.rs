#[derive(Clone, Copy, Debug)]
pub(crate) struct Memory {
	pub(crate) physical: crate::c_ulong,
	pub(crate) num_pages: crate::c_uint,
	pub(crate) used_pages: crate::c_uint,
}

impl Memory {
	pub(crate) fn update(&mut self, reader: &mut impl std::io::Read) -> Result<(), crate::Error> {
		let inactive_pages: crate::c_uint = crate::Parse::parse(reader)?;
		let cache_pages: crate::c_uint = crate::Parse::parse(reader)?;
		let free_pages: crate::c_uint = crate::Parse::parse(reader)?;
		self.used_pages = self.num_pages - inactive_pages - cache_pages - free_pages;
		Ok(())
	}
}

impl crate::Parse for Memory {
	fn parse<R>(reader: &mut R) -> std::io::Result<Self> where R: std::io::Read {
		let physical = crate::Parse::parse(reader)?;
		let num_pages = crate::Parse::parse(reader)?;
		Ok(crate::memory::Memory {
			physical,
			num_pages,
			used_pages: 0,
		})
	}
}
