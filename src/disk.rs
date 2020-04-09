#[derive(Debug)]
pub(crate) struct Disk {
	pub(crate) name: String,
	pub(crate) serial_number: String,
	smart_status_exec: crate::ssh_exec::smartctl_a::Exec,
	pub(crate) smart_passed: bool,
	pub(crate) temperature: crate::c_uint,
}

impl Disk {
	pub(crate) fn get_all(session: &ssh2::Session) -> Result<Box<[Self]>, crate::Error> {
		let disk_names = crate::ssh_exec::sysctl_kern_disks::run(session)?;
		let result: Result<Box<[_]>, crate::Error> =
			disk_names.split(' ')
			.filter_map(|disk_name| {
				if disk_name.is_empty() {
					return None;
				}

				let name = disk_name.to_owned();
				Some(Disk::new(name, session))
			})
			.collect();
		let mut result = result?;
		result.sort_by(|disk1, disk2| disk1.name.cmp(&disk2.name));
		Ok(result)
	}

	fn new(name: String, session: &ssh2::Session) -> Result<Self, crate::Error> {
		let serial_number = crate::ssh_exec::smartctl_i::get_serial_number(&name, session)?;

		let smart_status_exec = crate::ssh_exec::smartctl_a::Exec::new(&name);

		Ok(Disk {
			name,
			serial_number,
			smart_status_exec,
			smart_passed: false,
			temperature: 0,
		})
	}
}

impl Disk {
	pub(crate) fn update(&mut self, session: &ssh2::Session) -> Result<(), crate::Error> {
		let (passed, current) = self.smart_status_exec.run(session)?;
		self.smart_passed = passed;
		self.temperature = current;
		Ok(())
	}
}

