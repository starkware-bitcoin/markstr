use anyhow::Result;
use bitcoincore_rpc::{bitcoin::Network, Auth, Client, RpcApi};
use std::env;
use std::process::{Child, Command, Stdio};

#[cfg(test)]
mod test_core;

pub struct TestNode {
	pub rpc: Client,
	pub _proc: Option<DockerBitcoind>,
	pub wallet: Option<String>,
}

pub struct DockerBitcoind {
	pub child: Child,
	pub rpc_url: String,
	pub cookie_file: std::path::PathBuf,
	pub data_dir: tempfile::TempDir,
}

impl TestNode {
	pub fn start() -> Result<Self> {
		if env::var("BITCOIN_RPC_URL").is_ok() {
			Self::attach_from_env()
		} else {
			Self::start_with_funded_wallet()
		}
	}

	fn start_with_funded_wallet() -> Result<Self> {
		// Use Docker with bitcoin/bitcoin-csfs image
		let node = Self::start_docker_bitcoind()?;

		let base_url = &node.rpc_url;
		let auth = Auth::CookieFile(node.cookie_file.clone());
		let base_client = Client::new(base_url, auth.clone())?;

		let (wallet_client, wallet_name) = Self::ensure_wallet_and_fund(&base_client, base_url, &auth)?;
		Ok(Self { rpc: wallet_client, _proc: Some(node), wallet: Some(wallet_name) })
	}

	fn start_docker_bitcoind() -> Result<DockerBitcoind> {
		// Create temporary directory for bitcoin data
		let data_dir = tempfile::tempdir()?;
		let data_dir_path = data_dir.path();
		
		// Find an available port
		let rpc_port = Self::find_available_port()?;
		
		// Build docker arguments with basic regtest configuration
		let docker_args = vec![
			"run".to_string(),
			"--rm".to_string(),
			"-d".to_string(),
			"-p".to_string(), format!("{}:18443", rpc_port),
			"-v".to_string(), format!("{}:/home/bitcoin/.bitcoin", data_dir_path.display()),
			"bitcoin/bitcoin-csfs".to_string(),
			"bitcoind".to_string(),
			"-regtest".to_string(),
			"-fallbackfee=0.0001".to_string(),
			"-txindex=1".to_string(),
			"-printtoconsole=0".to_string(),
			"-server=1".to_string(),
			"-rpcbind=0.0.0.0".to_string(),
			"-rpcallowip=0.0.0.0/0".to_string(),
		];

		let child = Command::new("docker")
			.args(&docker_args)
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.spawn()
			.map_err(|e| anyhow::anyhow!("Failed to start docker container: {}", e))?;
		
		// Wait a bit for the container to start
		std::thread::sleep(std::time::Duration::from_secs(5));
		
		let rpc_url = format!("http://127.0.0.1:{}", rpc_port);
		let cookie_file = data_dir_path.join("regtest").join(".cookie");
		
		// Wait for the cookie file to be created
		let mut attempts = 0;
		while !cookie_file.exists() && attempts < 50 {
			std::thread::sleep(std::time::Duration::from_millis(200));
			attempts += 1;
		}
		
		if !cookie_file.exists() {
			return Err(anyhow::anyhow!("Cookie file not created after waiting"));
		}
		
		Ok(DockerBitcoind {
			child,
			rpc_url,
			cookie_file,
			data_dir,
		})
	}
	
	fn find_available_port() -> Result<u16> {
		use std::net::{TcpListener, SocketAddr};
		
		// Try to bind to an available port starting from 18443
		for port in 18443..18500 {
			let addr: SocketAddr = format!("127.0.0.1:{}", port).parse()?;
			if TcpListener::bind(addr).is_ok() {
				return Ok(port);
			}
		}
		
		Err(anyhow::anyhow!("No available ports found"))
	}

	fn attach_from_env() -> Result<Self> {
		let base_url = env::var("BITCOIN_RPC_URL")?;
		let (user, pass) = (env::var("BITCOIN_RPC_USER")?, env::var("BITCOIN_RPC_PASS")?);
		let auth = Auth::UserPass(user, pass);
		let base_client = Client::new(&base_url, auth.clone())?;

		let (wallet_client, wallet_name) = Self::ensure_wallet_and_fund(&base_client, &base_url, &auth)?;
		Ok(Self { rpc: wallet_client, _proc: None, wallet: Some(wallet_name) })
	}

	fn ensure_wallet_and_fund(base_client: &Client, base_url: &str, auth: &Auth) -> Result<(Client, String)> {
		let wallet_name = env::var("BITCOIN_WALLET").unwrap_or_else(|_| "testwallet".to_string());
		let _ = base_client.create_wallet(&wallet_name, None, None, None, None);
		let _ = base_client.load_wallet(&wallet_name);

		let wallet_url = format!("{}/wallet/{}", base_url, wallet_name);
		let wallet_client = Client::new(&wallet_url, auth.clone())?;

		let addr_unchecked = wallet_client.get_new_address(None, None)?;
		let addr = addr_unchecked.require_network(Network::Regtest)?;
		wallet_client.generate_to_address(101, &addr)?;

		Ok((wallet_client, wallet_name))
	}
}

impl Drop for TestNode {
	fn drop(&mut self) {
		if let Some(ref proc) = self._proc {
			// Try to stop bitcoind gracefully via RPC
			let auth = Auth::CookieFile(proc.cookie_file.clone());
			if let Ok(client) = Client::new(&proc.rpc_url, auth) {
				let _ = client.stop();
			}
			// Give it a moment to shut down gracefully
			std::thread::sleep(std::time::Duration::from_millis(500));
		}
	}
}

impl Drop for DockerBitcoind {
	fn drop(&mut self) {
		// Kill the docker container
		let _ = self.child.kill();
		let _ = self.child.wait();
	}
}
