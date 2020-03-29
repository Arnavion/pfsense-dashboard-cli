#[derive(Debug)]
pub(crate) struct Interfaces {
	wan: std::collections::BTreeMap<String, Interface>,
	lan_bridge: (String, Interface),
	lan: std::collections::BTreeMap<String, Interface>,
}

impl Interfaces {
	pub(crate) fn new(config: &crate::config::Config) -> Self {
		Interfaces {
			wan: config.interfaces.wan.iter().map(|name| (name.clone(), Interface::new(name))).collect(),
			lan_bridge: (config.interfaces.lan_bridge.clone(), Interface::new(&config.interfaces.lan_bridge)),
			lan: config.interfaces.lan.iter().map(|name| (name.clone(), Interface::new(name))).collect(),
		}
	}

	pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = (&'_ str, &'_ mut Interface, bool)> {
		self.wan.iter_mut().map(|(name, interface)| (name.as_ref(), interface, false))
		.chain(std::iter::once((self.lan_bridge.0.as_ref(), &mut self.lan_bridge.1, true)))
		.chain(self.lan.iter_mut().map(|(name, interface)| (name.as_ref(), interface, false)))
	}

	pub(crate) fn names(&self) -> impl Iterator<Item = &'_ str> {
		self.wan.keys().map(AsRef::as_ref)
		.chain(std::iter::once(self.lan_bridge.0.as_ref()))
		.chain(self.lan.keys().map(AsRef::as_ref))
	}

	pub(crate) fn wan_names(&self) -> impl Iterator<Item = &'_ str> {
		self.wan.keys().map(AsRef::as_ref)
	}

	pub(crate) fn update(&mut self, session: &ssh2::Session) -> Result<(), crate::Error> {
		for (_, interface, _) in self.iter_mut() {
			interface.addresses.clear();

			interface.received_bytes_previous = interface.received_bytes;
			interface.received_bytes = 0;

			interface.sent_bytes_previous = interface.sent_bytes;
			interface.sent_bytes = 0;

			interface.error = interface.ifconfig_exec.run(session)?;
		}

		let interface_statistics = crate::ssh_exec::netstat_bin::get_interfaces(session)?;

		for interface_statistics in interface_statistics {
			let interface_name = interface_statistics.name;
			let interface =
				if let Some(interface) = self.wan.get_mut(&interface_name) {
					Some(interface)
				}
				else if interface_name == self.lan_bridge.0 {
					Some(&mut self.lan_bridge.1)
				}
				else {
					self.lan.get_mut(&interface_name)
				};

			if let Some(interface) = interface {
				if !interface_statistics.network.starts_with("<Link#") && !interface_statistics.address.starts_with("fe80:") {
					interface.addresses.push(interface_statistics.address);
				}

				if interface_statistics.network.starts_with("<Link#") {
					interface.received_bytes += interface_statistics.received_bytes;
					interface.sent_bytes += interface_statistics.sent_bytes;
				}
			}
		}

		Ok(())
	}
}

#[derive(Debug)]
pub(crate) struct Interface {
	ifconfig_exec: crate::ssh_exec::ifconfig::Exec,

	pub(crate) error: Option<String>,

	pub(crate) addresses: Vec<String>,

	received_bytes_previous: u64,
	received_bytes: u64,

	sent_bytes_previous: u64,
	sent_bytes: u64,
}

impl Interface {
	fn new(name: &str) -> Self {
		let ifconfig_exec = crate::ssh_exec::ifconfig::Exec::new(name);

		Interface {
			ifconfig_exec,

			error: None,

			addresses: vec![],

			received_bytes_previous: 0,
			received_bytes: 0,

			sent_bytes_previous: 0,
			sent_bytes: 0,
		}
	}

	pub(crate) fn speed(&self, time_since_previous: std::time::Duration) -> Option<(f32, f32)> {
		if self.received_bytes_previous > 0 && self.sent_bytes_previous > 0 {
			let interface_received_speed = (self.received_bytes.saturating_sub(self.received_bytes_previous)) as f32 / time_since_previous.as_secs() as f32 * 8.;
			let interface_sent_speed = (self.sent_bytes.saturating_sub(self.sent_bytes_previous)) as f32 / time_since_previous.as_secs() as f32 * 8.;
			Some((interface_received_speed, interface_sent_speed))
		}
		else {
			None
		}
	}
}
