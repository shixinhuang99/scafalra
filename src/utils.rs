use std::{
	env, io,
	path::{Path, PathBuf},
	sync::OnceLock,
};

use anyhow::Result;
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

pub const SELF_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(feature = "self_update")]
pub const SELF_TARGET: &str = env!("TARGET");

pub struct Downloader {
	url: String,
	file: PathBuf,
}

impl Downloader {
	pub fn new(url: &str, file: &Path, ext: &str) -> Self {
		Self {
			url: url.to_string(),
			file: file.with_extension(ext),
		}
	}

	pub fn download(&self) -> Result<&Self> {
		let response = global_agent().get(&self.url).call()?;
		let mut file = fs::File::create(&self.file)?;
		io::copy(&mut response.into_reader(), &mut file)?;

		Ok(self)
	}

	pub fn tar_unpack(&self, dst: &Path) -> Result<()> {
		let file = fs::File::open(&self.file)?;
		let dec = flate2::read::GzDecoder::new(file);
		let mut tar = tar::Archive::new(dec);
		tar.unpack(dst)?;
		fs::remove_file(&self.file)?;

		Ok(())
	}

	#[cfg(all(windows, feature = "self_update"))]
	pub fn zip_unpack(&self, dst: &Path) -> Result<()> {
		let file = fs::File::open(&self.file)?;
		let mut archive = zip::ZipArchive::new(file)?;
		archive.extract(dst)?;
		fs::remove_file(&self.file)?;

		Ok(())
	}
}
