#[derive(Debug)]
pub(crate) struct Gateways {
	gateways: std::collections::BTreeMap<String, Option<Gateway>>,
}

impl Gateways {
	pub(crate) fn new(gateways: impl IntoIterator<Item = crate::pfconfig::Gateway>) -> Self {
		let gateways =
			gateways.into_iter()
			.map(|crate::pfconfig::Gateway { name }| (name, None))
			.collect();
		Gateways {
			gateways,
		}
	}

	pub(crate) fn iter(&self) -> impl Iterator<Item = (&str, Option<Gateway>)> {
		self.gateways.iter().map(|(name, gateway)| (&**name, *gateway))
	}

	pub(crate) fn update(&mut self, session: &ssh2::Session) -> Result<(), crate::Error> {
		for gateway in self.gateways.values_mut() {
			*gateway = None;
		}

		let gateway_pinger_statistics = crate::ssh_exec::dpinger::get_statistics(session);
		for gateway_pinger_statistics in gateway_pinger_statistics {
			let crate::ssh_exec::dpinger::Statistics { name, latency_average, latency_stddev, ping_packet_loss } = gateway_pinger_statistics?;
			if let Some(gateway) = self.gateways.get_mut(&name) {
				*gateway = Some(Gateway {
					latency_average,
					latency_stddev,
					ping_packet_loss,
				});
			}
		}

		Ok(())
	}
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Gateway {
	pub(crate) latency_average: std::time::Duration,
	pub(crate) latency_stddev: std::time::Duration,
	pub(crate) ping_packet_loss: crate::c_ulong,
}
