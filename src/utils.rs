use std::{env, io, path::Path, sync::OnceLock};

use anyhow::Result;
use flate2::read::GzDecoder;
use fs_err as fs;
use ureq::{Agent, AgentBuilder, Proxy};

pub fn global_agent() -> &'static Agent {
	static AGENT: OnceLock<Agent> = OnceLock::new();

	AGENT.get_or_init(|| {
		let proxy = env::var("https_proxy").or_else(|_| env::var("http_proxy"));
		let agent_builder = AgentBuilder::new();

		if let Ok(env_proxy) = proxy {
			let proxy = Proxy::new(env_proxy);
			if let Ok(proxy) = proxy {
				return agent_builder.proxy(proxy).build();
			}
		}

		agent_builder.build()
	})
}

#[cfg(feature = "self_update")]
pub fn get_self_target() -> &'static str {
	env!("TARGET")
}

pub fn get_self_version() -> &'static str {
	env!("CARGO_PKG_VERSION")
}

pub fn download(url: &str, file_path: &Path) -> Result<()> {
	let response = global_agent().get(url).call()?;
	let mut file = fs::File::create(file_path)?;

	io::copy(&mut response.into_reader(), &mut file)?;

	Ok(())
}

pub fn tar_unpack(file_path: &Path, dst: &Path) -> Result<()> {
	let file = fs::File::open(file_path)?;
	let dec = GzDecoder::new(file);
	let mut tar = tar::Archive::new(dec);

	tar.unpack(dst)?;

	Ok(())
}

#[cfg(all(windows, feature = "self_update"))]
pub fn zip_unpack(file_path: &Path, dst: &Path) -> Result<()> {
	let file = fs::File::open(file_path)?;
	let mut archive = zip::ZipArchive::new(file)?;
	archive.extract(dst)?;

	Ok(())
}
