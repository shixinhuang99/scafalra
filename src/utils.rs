use std::{env, io, path::Path};

use anyhow::Result;
use flate2::read::GzDecoder;
use fs_err as fs;
use ureq::{Agent, AgentBuilder, Proxy};

pub fn build_proxy_agent() -> Agent {
	let env_proxy = env::var("https_proxy").or_else(|_| env::var("http_proxy"));
	let agent = AgentBuilder::new();

	if let Ok(env_proxy) = env_proxy {
		let proxy = Proxy::new(env_proxy);
		if let Ok(proxy) = proxy {
			return agent.proxy(proxy).build();
		}
	}

	agent.build()
}

#[cfg(feature = "self_update")]
pub fn get_self_target() -> &'static str {
	env!("TARGET")
}

pub fn get_self_version() -> &'static str {
	env!("CARGO_PKG_VERSION")
}

pub fn download(url: &str, file_path: &Path) -> Result<()> {
	let agent = build_proxy_agent();
	let response = agent.get(url).call()?;
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
