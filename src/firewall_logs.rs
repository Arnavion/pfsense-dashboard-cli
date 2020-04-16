#[derive(Debug)]
pub(crate) struct Logs {
	inner: [Option<Log>; 10],

	// Index of the newest log. Moves backwards as new logs are pushed.
	head: usize,
}

impl Logs {
	pub(crate) fn new(interfaces: impl IntoIterator<Item = String>, ssh: &crate::config::Ssh) -> Result<std::sync::Arc<std::sync::Mutex<Self>>, crate::Error> {
		let result = std::sync::Arc::new(std::sync::Mutex::new(Logs {
			inner: Default::default(),
			head: 0,
		}));

		// Can't multiplex on the same session because ssh2 has internal mutexes to only let one command run at a time.
		// So create a new connection and session.
		let session = crate::connect(&ssh.hostname, &ssh.username, None)?;

		let logs = result.clone();

		let interfaces = interfaces.into_iter().collect();

		let _ = std::thread::spawn(move || if let Err(err) = log_reader_thread(&logs, &session, &interfaces) {
			eprintln!("{:?}", err);
			std::process::exit(1);
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

#[derive(Debug)]
pub(crate) struct Log {
	pub(crate) timestamp: String,
	pub(crate) interface: String,
	pub(crate) action: Action,
	pub(crate) protocol: Protocol,
}

impl Log {
	fn from_str(s: &str, interfaces: &std::collections::BTreeSet<String>) -> Result<Self, ()> {
		// Ref: https://docs.netgate.com/pfsense/en/latest/monitoring/filter-log-format-for-pfsense-2-2.html

		let timestamp = s.get(..("MMM dd HH:mm:ss".len())).ok_or(())?;

		let mut line_parts = s.split(',');

		let interface = line_parts.nth(4).ok_or(())?;
		if !interfaces.contains(interface) {
			return Err(());
		}

		let reason = line_parts.next().ok_or(())?;
		if reason != "match" {
			return Err(());
		}

		let action = line_parts.next().ok_or(())?;
		let action = action.parse()?;

		let direction = line_parts.next().ok_or(())?;
		if direction != "in" {
			return Err(());
		}

		let ipv4_or_v6 = line_parts.next().ok_or(())?;

		let (protocol_id_offset, source_ip_offset) = match ipv4_or_v6 {
			"4" => (6, 2),
			"6" => (4, 1),
			_ => return Err(()),
		};

		let protocol_id = line_parts.nth(protocol_id_offset).ok_or(())?;

		let source_ip = line_parts.nth(source_ip_offset).ok_or(())?;
		let destination_ip = line_parts.next().ok_or(())?;

		let protocol = match protocol_id {
			"1" | "58" => {
				let source = ip_addr_from_parts(ipv4_or_v6, source_ip).ok_or(())?;

				let destination = ip_addr_from_parts(ipv4_or_v6, destination_ip).ok_or(())?;

				Protocol::Icmp { source, destination }
			},

			"6" => {
				let source_port = line_parts.next().ok_or(())?;
				let source = socket_addr_from_parts(ipv4_or_v6, source_ip, source_port).ok_or(())?;

				let destination_port = line_parts.next().ok_or(())?;
				let destination = socket_addr_from_parts(ipv4_or_v6, destination_ip, destination_port).ok_or(())?;

				Protocol::Tcp { source, destination }
			},

			"17" => {
				let source_port = line_parts.next().ok_or(())?;
				let source = socket_addr_from_parts(ipv4_or_v6, source_ip, source_port).ok_or(())?;

				let destination_port = line_parts.next().ok_or(())?;
				let destination = socket_addr_from_parts(ipv4_or_v6, destination_ip, destination_port).ok_or(())?;

				Protocol::Udp { source, destination }
			},

			_ => return Err(()),
		};

		Ok(Log {
			timestamp: timestamp.to_owned(),
			interface: interface.to_owned(),
			action,
			protocol,
		})
	}
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Action {
	Block,
	Pass,
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

#[derive(Clone, Copy, Debug)]
pub(crate) enum Protocol {
	Icmp { source: std::net::IpAddr, destination: std::net::IpAddr },
	Tcp { source: std::net::SocketAddr, destination: std::net::SocketAddr },
	Udp { source: std::net::SocketAddr, destination: std::net::SocketAddr },
}

fn log_reader_thread(logs: &std::sync::Mutex<Logs>, session: &ssh2::Session, interfaces: &std::collections::BTreeSet<String>) -> Result<(), crate::Error> {
	loop {
		let lines = crate::ssh_exec::clog_filter_log::run(session);

		for line in lines {
			let line = line?;

			let log = match Log::from_str(&line, interfaces) {
				Ok(log) => log,
				Err(()) => continue,
			};

			let mut logs = logs.lock().expect("could not lock firewall logs queue");
			logs.push(log);
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
