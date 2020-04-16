#[derive(Debug)]
pub(crate) struct PfConfig {
	pub(crate) gateway_interfaces: Vec<String>,
	pub(crate) bridge_interfaces: Vec<String>,
	pub(crate) other_interfaces: Vec<String>,
	pub(crate) gateways: Vec<Gateway>,
	pub(crate) services: Vec<Service>,
}

#[derive(Debug)]
pub(crate) struct Gateway {
	pub(crate) name: String,
	pub(crate) interface: String,
}

#[derive(Debug)]
pub(crate) struct Service {
	pub(crate) name: String,
	pub(crate) executable: String,
}

impl PfConfig {
	pub(crate) fn load(session: &ssh2::Session) -> Result<Self, crate::Error> {
		let pfconfig = crate::ssh_exec::read_text_file(session, std::path::Path::new("/cf/conf/config.xml"))?;
		let pfconfig = roxmltree::Document::parse(&pfconfig)?;
		let mut pfconfig: PfSense<'_> = std::convert::TryInto::try_into(pfconfig.root_element())?;

		let mut gateway_interfaces = vec![];
		let mut gateways = vec![];

		for (gateway_name, gateway_interface) in pfconfig.gateways.0 {
			let r#if =
				pfconfig.interfaces.0
				.remove(gateway_interface)
				.ok_or_else(|| format!("gateway {} is defined on interface {} but this interface does not exist", gateway_name, gateway_interface))?;
			gateway_interfaces.push(r#if.to_owned());

			gateways.push(Gateway {
				name: gateway_name.to_owned(),
				interface: r#if.to_owned(),
			});
		}

		let mut interfaces: std::collections::BTreeSet<_> = pfconfig.interfaces.0.into_iter().map(|(_, interface_if)| interface_if).collect();
		let mut bridge_interfaces = vec![];

		if let Some(bridges) = pfconfig.bridges {
			for bridge_name in bridges.0 {
				bridge_interfaces.push(bridge_name.to_owned());

				if !interfaces.remove(bridge_name) {
					return Err(format!("bridge {} does not exist as an interface", bridge_name).into());
				}
			}
		}

		let other_interfaces = interfaces.into_iter().map(ToOwned::to_owned).collect();

		let services =
			pfconfig.installed_packages.0.into_iter()
			.map(|(name, executable)| Service {
				name: name.to_owned(),
				executable: executable.to_owned(),
			})
			.collect();

		let result = PfConfig {
			gateway_interfaces,
			bridge_interfaces,
			other_interfaces,
			gateways,
			services,
		};

		Ok(result)
	}
}

#[derive(Debug)]
struct PfSense<'input> {
	bridges: Option<Bridges<'input>>,
	interfaces: Interfaces<'input>,
	gateways: Gateways<'input>,
	installed_packages: InstalledPackages<'input>,
}

impl<'input> std::convert::TryFrom<roxmltree::Node<'input, 'input>> for PfSense<'input> {
	type Error = crate::Error;

	fn try_from(node: roxmltree::Node<'input, 'input>) -> Result<Self, Self::Error> {
		let bridges_tag_name: roxmltree::ExpandedName<'_> = "bridges".into();
		let interfaces_tag_name: roxmltree::ExpandedName<'_> = "interfaces".into();
		let gateways_tag_name: roxmltree::ExpandedName<'_> = "gateways".into();
		let installed_packages_tag_name: roxmltree::ExpandedName<'_> = "installedpackages".into();

		let mut bridges = None;
		let mut interfaces = None;
		let mut gateways = None;
		let mut installed_packages = None;

		for child in node.children() {
			let child_tag_name = child.tag_name();
			if child_tag_name == bridges_tag_name {
				bridges = Some(std::convert::TryInto::try_into(child)?);
			}
			else if child_tag_name == interfaces_tag_name {
				interfaces = Some(std::convert::TryInto::try_into(child)?);
			}
			else if child_tag_name == gateways_tag_name {
				gateways = Some(std::convert::TryInto::try_into(child)?);
			}
			else if child_tag_name == installed_packages_tag_name {
				installed_packages = Some(std::convert::TryInto::try_into(child)?);
			}
		}

		let interfaces = interfaces.ok_or("interfaces not found in config.xml")?;
		let gateways = gateways.ok_or("gateways not found in config.xml")?;
		let installed_packages = installed_packages.ok_or("installed packages not found in config.xml")?;

		Ok(PfSense {
			bridges,
			interfaces,
			gateways,
			installed_packages,
		})
	}
}

#[derive(Debug)]
struct Bridges<'input>(Vec<&'input str>);

impl<'input> std::convert::TryFrom<roxmltree::Node<'input, 'input>> for Bridges<'input> {
	type Error = crate::Error;

	fn try_from(node: roxmltree::Node<'input, 'input>) -> Result<Self, Self::Error> {
		let bridge_tag_name: roxmltree::ExpandedName<'_> = "bridged".into();

		let inner: Result<_, crate::Error> =
			node.children()
			.filter_map(|child|
				if child.tag_name() == bridge_tag_name {
					let Bridge { bridgeif } = match std::convert::TryInto::try_into(child) {
						Ok(bridge) => bridge,
						Err(err) => return Some(Err(err)),
					};
					Some(Ok(bridgeif))
				}
				else {
					None
				})
			.collect();
		let inner = inner?;

		Ok(Bridges(inner))
	}
}

#[derive(Debug)]
struct Bridge<'input> {
	bridgeif: &'input str,
}

impl<'input> std::convert::TryFrom<roxmltree::Node<'input, 'input>> for Bridge<'input> {
	type Error = crate::Error;

	fn try_from(node: roxmltree::Node<'input, 'input>) -> Result<Self, Self::Error> {
		let bridgeif_tag_name: roxmltree::ExpandedName<'_> = "bridgeif".into();

		let bridgeif = node.children().find(|node| node.tag_name() == bridgeif_tag_name).ok_or("bridges.*.bridgeif not found in config.xml")?;
		let bridgeif = bridgeif.text().ok_or("bridges.*.bridgeif is not a text node")?;

		Ok(Bridge {
			bridgeif,
		})
	}
}

#[derive(Debug)]
struct Interfaces<'input>(std::collections::BTreeMap<&'input str, &'input str>);

impl<'input> std::convert::TryFrom<roxmltree::Node<'input, 'input>> for Interfaces<'input> {
	type Error = crate::Error;

	fn try_from(node: roxmltree::Node<'input, 'input>) -> Result<Self, Self::Error> {
		let inner: Result<_, crate::Error> =
			node.children()
			.filter_map(|child|
				if child.is_element() {
					let Interface { name, r#if } = match std::convert::TryInto::try_into(child) {
						Ok(interface) => interface,
						Err(err) => return Some(Err(err)),
					};
					Some(Ok((name, r#if)))
				}
				else {
					None
				})
			.collect();
		let inner = inner?;

		Ok(Interfaces(inner))
	}
}

#[derive(Debug)]
struct Interface<'input> {
	name: &'input str,
	r#if: &'input str,
}

impl<'input> std::convert::TryFrom<roxmltree::Node<'input, 'input>> for Interface<'input> {
	type Error = crate::Error;

	fn try_from(node: roxmltree::Node<'input, 'input>) -> Result<Self, Self::Error> {
		let if_tag_name: roxmltree::ExpandedName<'_> = "if".into();

		let name = node.tag_name().name();

		let r#if = node.children().find(|node| node.tag_name() == if_tag_name).ok_or("interfaces.*.if not found in config.xml")?;
		let r#if = r#if.text().ok_or("interfaces.*.if is not a text node")?;

		Ok(Interface {
			name,
			r#if,
		})
	}
}

#[derive(Debug)]
struct Gateways<'input>(std::collections::BTreeMap<&'input str, &'input str>);

impl<'input> std::convert::TryFrom<roxmltree::Node<'input, 'input>> for Gateways<'input> {
	type Error = crate::Error;

	fn try_from(node: roxmltree::Node<'input, 'input>) -> Result<Self, Self::Error> {
		let gateway_item_tag_name: roxmltree::ExpandedName<'_> = "gateway_item".into();

		let inner: Result<_, crate::Error> =
			node.children()
			.filter_map(|child|
				if child.tag_name() == gateway_item_tag_name {
					let GatewayItem { name, interface } = match std::convert::TryInto::try_into(child) {
						Ok(gateway) => gateway,
						Err(err) => return Some(Err(err)),
					};
					Some(Ok((name, interface)))
				}
				else {
					None
				})
			.collect();
		let inner = inner?;

		Ok(Gateways(inner))
	}
}

#[derive(Debug)]
struct GatewayItem<'input> {
	name: &'input str,
	interface: &'input str,
}

impl<'input> std::convert::TryFrom<roxmltree::Node<'input, 'input>> for GatewayItem<'input> {
	type Error = crate::Error;

	fn try_from(node: roxmltree::Node<'input, 'input>) -> Result<Self, Self::Error> {
		let interface_tag_name: roxmltree::ExpandedName<'_> = "interface".into();
		let name_tag_name: roxmltree::ExpandedName<'_> = "name".into();

		let interface = node.children().find(|node| node.tag_name() == interface_tag_name).ok_or("gateways.gateway_item.interface not found in config.xml")?;
		let interface = interface.text().ok_or("gateways.gateway_item.interface is not a text node")?;

		let name = node.children().find(|node| node.tag_name() == name_tag_name).ok_or("gateways.gateway_item.name not found in config.xml")?;
		let name = name.text().ok_or("gateways.gateway_item.name is not a text node")?;

		Ok(GatewayItem {
			interface,
			name,
		})
	}
}

#[derive(Debug)]
struct InstalledPackages<'input>(Vec<(&'input str, &'input str)>);

impl<'input> std::convert::TryFrom<roxmltree::Node<'input, 'input>> for InstalledPackages<'input> {
	type Error = crate::Error;

	fn try_from(node: roxmltree::Node<'input, 'input>) -> Result<Self, Self::Error> {
		let service_tag_name: roxmltree::ExpandedName<'_> = "service".into();

		let inner: Result<_, crate::Error> =
			node.children()
			.filter_map(|child|
				if child.tag_name() == service_tag_name {
					let InstalledPackageService { name, executable } = match std::convert::TryInto::try_into(child) {
						Ok(installed_package_service) => installed_package_service,
						Err(err) => return Some(Err(err)),
					};
					Some(Ok((name, executable)))
				}
				else {
					None
				})
			.collect();
		let inner = inner?;

		Ok(InstalledPackages(inner))
	}
}

#[derive(Debug)]
struct InstalledPackageService<'input> {
	name: &'input str,
	executable: &'input str,
}

impl<'input> std::convert::TryFrom<roxmltree::Node<'input, 'input>> for InstalledPackageService<'input> {
	type Error = crate::Error;

	fn try_from(node: roxmltree::Node<'input, 'input>) -> Result<Self, Self::Error> {
		let name_tag_name: roxmltree::ExpandedName<'_> = "name".into();
		let executable_tag_name: roxmltree::ExpandedName<'_> = "executable".into();

		let name = node.children().find(|node| node.tag_name() == name_tag_name).ok_or("installedpackages.service.name not found in config.xml")?;
		let name = name.text().ok_or("installedpackages.service.name is not a text node")?;

		let executable = node.children().find(|node| node.tag_name() == executable_tag_name).ok_or("installedpackages.service.executable not found in config.xml")?;
		let executable = executable.text().ok_or("installedpackages.service.executable is not a text node")?;

		Ok(InstalledPackageService {
			name,
			executable,
		})
	}
}
