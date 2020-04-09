#[derive(Debug)]
pub(crate) struct Service {
	pub(crate) name: String,
	is_running_exec: crate::ssh_exec::pgrep::Exec,
	pub(crate) is_running: bool,
}

impl Service {
	pub(crate) fn get_all(
		builtin_services: impl IntoIterator<Item = String>,
		installed_package_services: impl IntoIterator<Item = crate::pfconfig::Service>,
	) -> Result<Box<[Self]>, crate::Error> {
		let result: Result<Box<[_]>, crate::Error> =
			builtin_services.into_iter()
			.map(|name| -> Result<_, crate::Error> {
				let (executable, pidfile) = match &*name {
					"dhcpd" => ("dhcpd", None),
					"ntpd" => ("ntpd", Some("ntpd.pid")),
					"radvd" => ("radvd", Some("radvd.pid")),
					"sshd" => ("sshd", Some("sshd.pid")),
					"syslogd" => ("syslogd", Some("syslog.pid")),
					"unbound" => ("unbound", Some("unbound.pid")),
					name => return Err(format!("{:?} is not recognized as a built-in service", name).into()),
				};
				Ok((name, executable.to_owned(), pidfile.map(ToOwned::to_owned)))
			})
			.chain(
				installed_package_services.into_iter()
				.map(|crate::pfconfig::Service { name, executable }| Ok::<_, crate::Error>((name, executable, None)))
			)
			.map(|service| {
				let (name, executable, pidfile) = service?;
				Ok(Service {
					name,
					is_running_exec: crate::ssh_exec::pgrep::Exec::new(&executable, pidfile.as_ref().map(AsRef::as_ref)),
					is_running: false,
				})
			})
			.collect();
		let mut result = result?;
		result.sort_by(|service1, service2| service1.name.cmp(&service2.name));
		Ok(result)
	}

	pub(crate) fn update(&mut self, session: &ssh2::Session) -> Result<(), crate::Error> {
		self.is_running = self.is_running_exec.run(session)?;
		Ok(())
	}
}
