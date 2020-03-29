#[derive(Clone, Copy, Debug)]
pub(crate) struct BootTime(pub(crate) std::time::SystemTime);

impl crate::Parse for BootTime {
	fn parse<R>(reader: &mut R) -> std::io::Result<Self> where R: std::io::Read {
		let seconds: crate::time_t = crate::Parse::parse(reader)?;

		let microseconds: crate::time_t = crate::Parse::parse(reader)?;

		#[allow(clippy::identity_conversion)] // time_t -> u64 is not necessarily identity conversion
		let boot_time = std::time::Duration::from_micros(u64::from(seconds) * 1_000_000 + u64::from(microseconds));
		let boot_time = std::time::UNIX_EPOCH + boot_time;
		Ok(BootTime(boot_time))
	}
}
