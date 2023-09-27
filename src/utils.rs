use std::env;

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
