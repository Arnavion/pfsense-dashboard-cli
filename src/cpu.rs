#[derive(Clone, Copy, Debug)]
pub(crate) struct Cpu {
	total_previous: crate::c_ulong,
	total: crate::c_ulong,

	idle_previous: crate::c_ulong,
	idle: crate::c_ulong,
}

impl Cpu {
	pub(crate) fn new() -> Self {
		Cpu {
			total_previous: 0,
			total: 0,

			idle_previous: 0,
			idle: 0,
		}
	}

	pub(crate) fn update(&mut self, reader: &mut impl std::io::Read) -> Result<(), crate::Error> {
		self.total_previous = self.total;
		self.total = 0;
		self.idle_previous = self.idle;
		self.idle = 0;

		let mut part_num = 0_usize;
		loop {
			let part = match crate::Parse::parse(reader) {
				Ok(part) => part,
				Err(ref err) if err.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()),
				Err(err) => return Err(err.into()),
			};

			self.total += part;

			if part_num == 4 {
				self.idle = part;
			}

			part_num += 1;
		}
	}

	pub(crate) fn usage_percent(self) -> Option<f32> {
		if self.total_previous > 0 {
			let cpu_total_diff = self.total - self.total_previous;
			let cpu_idle_diff = self.idle - self.idle_previous;
			let cpu_usage_percent = (cpu_total_diff - cpu_idle_diff) as f32 * 100. / cpu_total_diff as f32;
			Some(cpu_usage_percent)
		}
		else {
			None
		}
	}
}
