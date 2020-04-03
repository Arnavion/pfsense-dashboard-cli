#[derive(serde_derive::Deserialize)]
pub(crate) struct Config {
	pub(crate) ssh: Ssh,
	#[serde(default)]
	pub(crate) services: Vec<Service>,
}

impl Config {
	pub(crate) fn load() -> Result<Self, crate::Error> {
		let mut path = dirs::config_dir().ok_or("config dir not defined")?;
		path.push("pfsense-dashboard");
		path.push("config.yaml");
		let f = std::fs::File::open(path)?;
		let result = serde_yaml::from_reader(f)?;
		Ok(result)
	}
}

#[derive(serde_derive::Deserialize)]
pub(crate) struct Ssh {
	pub(crate) hostname: String,
	pub(crate) username: String,
}

#[derive(serde_derive::Deserialize)]
pub(crate) struct Service {
	pub(crate) name: String,
	pub(crate) process: String,
	pub(crate) pidfile: Option<String>,
}
