#[derive(Debug)]
pub(crate) struct Service {
	pub(crate) name: String,
	is_running_exec: crate::ssh_exec::pgrep::Exec,
	pub(crate) is_running: bool,
}

impl Service {
	pub(crate) fn get_all<'a>(services: impl Iterator<Item = &'a crate::config::Service>) -> Box<[Self]> {
		let result: Vec<_> =
			services
			.map(|crate::config::Service { name, process, pidfile }| Service {
				name: name.to_owned(),
				is_running_exec: crate::ssh_exec::pgrep::Exec::new(&process, pidfile.as_ref().map(AsRef::as_ref)),
				is_running: false,
			})
			.collect();
		let result = result.into_boxed_slice();
		result
	}

	pub(crate) fn update(&mut self, session: &ssh2::Session) -> Result<(), crate::Error> {
		self.is_running = self.is_running_exec.run(session)?;
		Ok(())
	}
}
