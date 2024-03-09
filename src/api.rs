use std::{
	env, fs, io,
	path::{Path, PathBuf},
	sync::OnceLock,
};

use anyhow::Result;
use ureq::{Agent, AgentBuilder, Proxy};

use crate::{cli::AddArgs, debug, repository::Repository};

fn global_agent() -> &'static Agent {
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

pub struct GitHubApi {
	token: Option<String>,
	endpoint: String,
}

impl GitHubApi {
	pub fn new(endpoint: Option<&str>) -> Self {
		let endpoint = endpoint.unwrap_or("https://api.github.com").to_string();

		Self {
			token: None,
			endpoint,
		}
	}

	pub fn set_token(&mut self, token: &str) {
		self.token = Some(token.to_string());
	}

	pub fn download(
		&self,
		repo: &Repository,
		args: &AddArgs,
		dest_dir: &Path,
	) -> Result<PathBuf> {
		let mut url = format!(
			"{}/repos/{}/{}/zipball",
			&self.endpoint, &repo.owner, &repo.name
		);

		if let Some(repo_ref) = args
			.branch
			.as_ref()
			.or(args.tag.as_ref().or(args.commit.as_ref()))
		{
			url.push_str(&format!("/{}", repo_ref));
		}

		debug!("url: {}", &url);

		let mut req = global_agent().get(&url);

		req = req
			.set("Accept", "application/vnd.github+json")
			.set("User-Agent", "scafalra")
			.set("X-GitHub-Api-Version", "2022-11-28");

		if let Some(token) = &self.token {
			req = req.set("Authorization", &format!("Bearer {}", token));
		}

		let resp = req.call()?;
		let file_path = dest_dir.with_extension("zip");
		let mut file = fs::File::create(&file_path)?;

		io::copy(&mut resp.into_reader(), &mut file)?;

		Ok(file_path)
	}
}
