#[derive(Debug)]
pub(crate) struct Logs {
	inner: [Option<Log>; 10],

	// Index of the newest log. Moves backwards as new logs are pushed.
	head: usize,
}

#[derive(Debug)]
pub(crate) struct Log {
	pub(crate) timestamp: String,
	pub(crate) interface: String,
	pub(crate) action: Action,
	pub(crate) protocol: Protocol,
}

impl Logs {
	pub(crate) fn new(interfaces: impl IntoIterator<Item = String>, ssh: &crate::config::Ssh) -> Result<std::sync::Arc<std::sync::Mutex<Self>>, crate::Error> {
		let result = std::sync::Arc::new(std::sync::Mutex::new(Logs {
			inner: Default::default(),
			head: 0,
		}));

		// Can't multiplex on the same session because ssh2 has internal mutexes to only let one command run at a time.
		// So create a new connection and session.
		let session = crate::connect(&ssh.hostname, &ssh.username)?;

		let logs = result.clone();

		let interfaces = interfaces.into_iter().collect();

		let _ = std::thread::spawn(move || if let Err(err) = log_reader_thread(&logs, &session, &interfaces) {
			eprintln!("{:?}", err);
			std::process::abort();
		});

		Ok(result)
	}

	pub(crate) fn iter(&self) -> impl Iterator<Item = &'_ Log> {
		let (second, first) = self.inner.split_at(self.head);
		first.iter().chain(second).flat_map(Option::as_ref)
	}

	fn push(&mut self, log: Log) {
		self.head = (self.head + self.inner.len() - 1) % self.inner.len();
		self.inner[self.head] = Some(log);
	}
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Action {
	Block,
	Pass,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Protocol {
	Icmp { source: std::net::IpAddr, destination: std::net::IpAddr },
	Tcp { source: std::net::SocketAddr, destination: std::net::SocketAddr },
	Udp { source: std::net::SocketAddr, destination: std::net::SocketAddr },
}

impl std::fmt::Display for Action {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Action::Block => f.write_str("block"),
			Action::Pass => f.write_str("pass "),
		}
	}
}

impl std::str::FromStr for Action {
	type Err = ();

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"block" => Ok(Action::Block),
			"pass" => Ok(Action::Pass),
			_ => Err(()),
		}
	}
}

fn log_reader_thread(logs: &std::sync::Mutex<Logs>, session: &ssh2::Session, interfaces: &std::collections::BTreeSet<String>) -> Result<(), crate::Error> {
	loop {
		let lines = crate::ssh_exec::clog_filter_log::run(session);

		for line in lines {
			let line = line?;

			// Ref: https://docs.netgate.com/pfsense/en/latest/monitoring/filter-log-format-for-pfsense-2-2.html

			let timestamp = match line.get(..("MMM dd HH:mm:ss".len())) { Some(part) => part, None => continue };

			let mut line_parts = line.split(',');

			let interface = match line_parts.nth(4) { Some(part) => part, None => continue };
			if !interfaces.contains(interface) {
				continue;
			}

			let reason = match line_parts.next() { Some(part) => part, None => continue };
			if reason != "match" {
				continue;
			}

			let action = match line_parts.next() { Some(part) => part, None => continue };
			let action = match action.parse() {
				Ok(action) => action,
				Err(()) => continue,
			};

			let direction = match line_parts.next() { Some(part) => part, None => continue };
			if direction != "in" {
				continue;
			}

			let ipv4_or_v6 = match line_parts.next() { Some(part) => part, None => continue };

			let (protocol_id_offset, source_ip_offset) = match ipv4_or_v6 {
				"4" => (6, 2),
				"6" => (4, 1),
				_ => continue,
			};

			let protocol_id = match line_parts.nth(protocol_id_offset) { Some(part) => part, None => continue };

			let source_ip = match line_parts.nth(source_ip_offset) { Some(part) => part, None => continue };
			let destination_ip = match line_parts.next() { Some(part) => part, None => continue };

			let protocol = match protocol_id {
				"1" | "58" => {
					let source = match ip_addr_from_parts(ipv4_or_v6, source_ip) {
						Some(addr) => addr,
						None => continue,
					};

					let destination = match ip_addr_from_parts(ipv4_or_v6, destination_ip) {
						Some(addr) => addr,
						None => continue,
					};

					Protocol::Icmp { source, destination }
				},

				"6" => {
					let source_port = match line_parts.next() { Some(part) => part, None => continue };
					let destination_port = match line_parts.next() { Some(part) => part, None => continue };

					let source = match socket_addr_from_parts(ipv4_or_v6, source_ip, source_port) {
						Some(addr) => addr,
						None => continue,
					};

					let destination = match socket_addr_from_parts(ipv4_or_v6, destination_ip, destination_port) {
						Some(addr) => addr,
						None => continue,
					};

					Protocol::Tcp { source, destination }
				},

				"17" => {
					let source_port = match line_parts.next() { Some(part) => part, None => continue };
					let destination_port = match line_parts.next() { Some(part) => part, None => continue };

					let source = match socket_addr_from_parts(ipv4_or_v6, source_ip, source_port) {
						Some(addr) => addr,
						None => continue,
					};

					let destination = match socket_addr_from_parts(ipv4_or_v6, destination_ip, destination_port) {
						Some(addr) => addr,
						None => continue,
					};

					Protocol::Udp { source, destination }
				},

				_ => continue,
			};

			let mut logs = logs.lock().expect("could not lock firewall logs queue");
			logs.push(Log {
				timestamp: timestamp.to_owned(),
				interface: interface.to_owned(),
				action,
				protocol,
			});
		}

		// `clog -f` returned, for some reason. Restart it.
		std::thread::sleep(std::time::Duration::from_secs(1));
	}
}

fn ip_addr_from_parts(ipv4_or_v6: &str, ip: &str) -> Option<std::net::IpAddr> {
	match ipv4_or_v6 {
		"4" => Some(std::net::IpAddr::V4(ip.parse().ok()?)),
		"6" => Some(std::net::IpAddr::V6(ip.parse().ok()?)),
		_ => None,
	}
}

fn socket_addr_from_parts(ipv4_or_v6: &str, ip: &str, port: &str) -> Option<std::net::SocketAddr> {
	let ip_addr = ip_addr_from_parts(ipv4_or_v6, ip)?;
	let port = port.parse().ok()?;
	Some(std::net::SocketAddr::new(ip_addr, port))
}
