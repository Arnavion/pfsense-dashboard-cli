#[derive(Debug)]
pub(crate) struct VersionInfo {
	pub(crate) version: String,
	pub(crate) version_patch: String,
	pub(crate) arch: String,
	pub(crate) os_release_date: String,
	pub(crate) os_base_version: String,
}

impl VersionInfo {
	pub(crate) fn get(session: &ssh2::Session) -> Result<Self, crate::Error> {
		let version = crate::ssh_exec::read_text_file(session, std::path::Path::new("/etc/version"))?;
		let version = version.split('\n').next().expect("split() returns at least one element").to_owned();

		let version_patch = crate::ssh_exec::read_text_file(session, std::path::Path::new("/etc/version.patch"))?;
		let version_patch = version_patch.split('\n').next().expect("split() returns at least one element").to_owned();

		let arch = crate::ssh_exec::uname_m::run(session)?;

		let os_release_date = crate::ssh_exec::read_text_file(session, std::path::Path::new("/etc/version.buildtime"))?;
		let os_release_date = os_release_date.split('\n').next().expect("split() returns at least one element").to_owned();

		let os_base_version = crate::ssh_exec::uname_sr::run(session)?;

		Ok(VersionInfo {
			version,
			version_patch,
			arch,
			os_release_date,
			os_base_version,
		})
	}
}
