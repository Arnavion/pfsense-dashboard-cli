#[derive(Debug)]
pub(crate) struct TemperatureSysctl {
	pub(crate) name: String,
	pub(crate) value: crate::c_uint,
}

impl TemperatureSysctl {
	pub(crate) fn get_all(session: &ssh2::Session) -> Result<Box<[Self]>, crate::Error> {
		let result: Result<Vec<_>, crate::Error> =
			crate::ssh_exec::sysctl_aN::run(session)
			.filter_map(|sysctl_name| match sysctl_name {
				Ok(sysctl_name) =>
					if sysctl_name.contains("temperature") {
						Some(Ok(TemperatureSysctl {
							name: sysctl_name,
							value: 0,
						}))
					}
					else {
						None
					},

				Err(err) => Some(Err(err)),
			})
			.collect();
		let result = result?;

		let mut result = result.into_boxed_slice();
		result.sort_by(|temperature_sysctl1, temperature_sysctl2| temperature_sysctl1.name.cmp(&temperature_sysctl2.name));
		Ok(result)
	}

	pub(crate) fn update(&mut self, reader: &mut impl std::io::Read) -> Result<(), crate::Error> {
		self.value = crate::Parse::parse(reader)?;
		Ok(())
	}
}
