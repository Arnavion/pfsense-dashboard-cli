#[derive(Debug)]
pub(crate) struct Interfaces {
	gateways: std::collections::BTreeMap<String, Interface>,
	bridges: std::collections::BTreeMap<String, Interface>,
	other: std::collections::BTreeMap<String, Interface>,
}

impl Interfaces {
	pub(crate) fn new(gateways: impl IntoIterator<Item = String>, bridges: impl IntoIterator<Item = String>, other: impl IntoIterator<Item = String>) -> Self {
		let make_pair = |name: String| { let interface = Interface::new(&name); (name, interface) };
		Interfaces {
			gateways: gateways.into_iter().map(make_pair).collect(),
			bridges: bridges.into_iter().map(make_pair).collect(),
			other: other.into_iter().map(make_pair).collect(),
		}
	}

	pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = (&'_ str, &'_ mut Interface, bool)> {
		self.gateways.iter_mut().map(|(name, interface)| (name.as_ref(), interface, false))
		.chain(self.bridges.iter_mut().map(|(name, interface)| (name.as_ref(), interface, true)))
		.chain(self.other.iter_mut().map(|(name, interface)| (name.as_ref(), interface, false)))
	}

	pub(crate) fn names(&self) -> impl Iterator<Item = &'_ str> {
		self.gateways.keys().map(AsRef::as_ref)
		.chain(self.bridges.keys().map(AsRef::as_ref))
		.chain(self.other.keys().map(AsRef::as_ref))
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
			#[allow(clippy::option_if_let_else)]
			let interface =
				if let Some(interface) = self.gateways.get_mut(&interface_name) {
					Some(interface)
				}
				else if let Some(interface) = self.bridges.get_mut(&interface_name) {
					Some(interface)
				}
				else if let Some(interface) = self.other.get_mut(&interface_name) {
					Some(interface)
				}
				else {
					None
				};

			if let Some(interface) = interface {
				if interface_statistics.network.starts_with("<Link#") {
					interface.received_bytes += interface_statistics.received_bytes;
					interface.sent_bytes += interface_statistics.sent_bytes;
				}
				else if !interface_statistics.address.starts_with("fe80:") {
					let address = interface_statistics.address.parse()?;
					let _ = interface.addresses.insert(InterfaceAddressOrdered(address));
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

	addresses: std::collections::BTreeSet<InterfaceAddressOrdered>,

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

			addresses: Default::default(),

			received_bytes_previous: 0,
			received_bytes: 0,

			sent_bytes_previous: 0,
			sent_bytes: 0,
		}
	}

	pub(crate) fn addresses(&self) -> impl Iterator<Item = std::net::IpAddr> + '_ {
		self.addresses.iter().map(|address| address.0)
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct InterfaceAddressOrdered(std::net::IpAddr);

impl std::cmp::PartialOrd for InterfaceAddressOrdered {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl std::cmp::Ord for InterfaceAddressOrdered {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		match (self.0, other.0) {
			(std::net::IpAddr::V6(_), std::net::IpAddr::V4(_)) => std::cmp::Ordering::Less,
			(std::net::IpAddr::V4(_), std::net::IpAddr::V6(_)) => std::cmp::Ordering::Greater,
			(std::net::IpAddr::V6(addr1), std::net::IpAddr::V6(addr2)) => addr1.cmp(&addr2),
			(std::net::IpAddr::V4(addr1), std::net::IpAddr::V4(addr2)) => addr1.cmp(&addr2),
		}
	}
}
