//! Ignoto Protocol Node
//!
//! A high-performance, minimal Substrate node designed to run the dev chain
//! with basic block authorship (Manual/Instant Seal) for development environments.

use std::sync::Arc;
use futures::FutureExt;
use polkadot_sdk::{
	sc_cli::{SubstrateCli, RunCmd, KeySubcommand},
	sc_service::{Configuration, TaskManager, PartialComponents, ChainType, Properties, error::Error as ServiceError},
	sc_telemetry::{Telemetry, TelemetryWorker},
	sc_transaction_pool_api::OffchainTransactionPoolFactory,
	sp_runtime::traits::Block as BlockT,
	*,
};

// --- Re-exports & Types ---
type Block = ignoto_runtime::interface::OpaqueBlock;
type RuntimeApi = ignoto_runtime::RuntimeApi;
type HostFunctions = sp_io::SubstrateHostFunctions;
type FullClient = sc_service::TFullClient<Block, RuntimeApi, sc_executor::WasmExecutor<HostFunctions>>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;

pub type Service = PartialComponents<
	FullClient,
	FullBackend,
	FullSelectChain,
	sc_consensus::DefaultImportQueue<Block>,
	sc_transaction_pool::TransactionPoolHandle<Block, FullClient>,
	Option<Telemetry>,
>;

// --- CLI module ---
pub mod cli {
	use super::*;

	#[derive(Debug, Clone)]
	pub enum Consensus {
		ManualSeal(u64),
		InstantSeal,
		None,
	}

	impl std::str::FromStr for Consensus {
		type Err = String;
		fn from_str(s: &str) -> Result<Self, Self::Err> {
			Ok(if s == "instant-seal" {
				Consensus::InstantSeal
			} else if let Some(block_time) = s.strip_prefix("manual-seal-") {
				Consensus::ManualSeal(block_time.parse().map_err(|_| "invalid block time")?)
			} else if s.to_lowercase() == "none" {
				Consensus::None
			} else {
				return Err("incorrect consensus identifier".into());
			})
		}
	}

	#[derive(Debug, clap::Parser)]
	pub struct Cli {
		#[command(subcommand)]
		pub subcommand: Option<Subcommand>,

		#[clap(long, default_value = "manual-seal-3000")]
		pub consensus: Consensus,

		#[clap(flatten)]
		pub run: sc_cli::RunCmd,
	}

	#[derive(Debug, clap::Subcommand)]
	pub enum Subcommand {
		/// Key management cli utilities
		#[command(subcommand)]
		Key(sc_cli::KeySubcommand),

		/// Export the chain specification.
		ExportChainSpec(sc_cli::ExportChainSpecCmd),

		/// Validate blocks.
		CheckBlock(sc_cli::CheckBlockCmd),

		/// Export blocks.
		ExportBlocks(sc_cli::ExportBlocksCmd),

		/// Export the state of a given block into a chain spec.
		ExportState(sc_cli::ExportStateCmd),

		/// Import blocks.
		ImportBlocks(sc_cli::ImportBlocksCmd),

		/// Remove the whole chain.
		PurgeChain(sc_cli::PurgeChainCmd),

		/// Revert the chain to a previous state.
		Revert(sc_cli::RevertCmd),

		/// Db meta columns information.
		ChainInfo(sc_cli::ChainInfoCmd),
	}
}

// --- ChainSpec module ---
pub mod chain_spec {
	use super::*;

	pub type ChainSpec = sc_service::GenericChainSpec;

	fn props() -> Properties {
		let mut properties = Properties::new();
		properties.insert("tokenDecimals".to_string(), 12.into());
		properties.insert("tokenSymbol".to_string(), "IGTO".into()); // Ignoto native token
		properties
	}

	pub fn development_chain_spec() -> Result<ChainSpec, String> {
		let wasm_binary = ignoto_runtime::WASM_BINARY
			.ok_or_else(|| "Development WASM binary is not available".to_string())?;
		
		Ok(ChainSpec::builder(wasm_binary, Default::default())
			.with_name("Ignoto Development Network")
			.with_id("ignoto_dev")
			.with_chain_type(ChainType::Development)
			.with_genesis_config_preset_name(sp_genesis_builder::DEV_RUNTIME_PRESET)
			.with_properties(props())
			.build())
	}
}

// --- RPC module ---
pub mod rpc {
	use super::*;
	use jsonrpsee::RpcModule;

	pub struct FullDeps<C, P> {
		pub client: Arc<C>,
		pub pool: Arc<P>,
	}

	pub fn create_full<C, P>(
		deps: FullDeps<C, P>,
	) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
	where
		C: Send
			+ Sync
			+ 'static
			+ sp_api::ProvideRuntimeApi<Block>
			+ sp_blockchain::HeaderBackend<Block>
			+ sp_blockchain::HeaderMetadata<Block, Error = sp_blockchain::Error>
			+ 'static,
		C::Api: sp_block_builder::BlockBuilder<Block>,
		C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, ignoto_runtime::interface::AccountId, ignoto_runtime::interface::Nonce>,
		P: sc_transaction_pool_api::TransactionPool + 'static,
	{
		use polkadot_sdk::substrate_frame_rpc_system::{System, SystemApiServer};
		let mut module = RpcModule::new(());
		let FullDeps { client, pool } = deps;

		module.merge(System::new(client, pool).into_rpc())?;
		Ok(module)
	}
}

// --- Service module ---
pub mod service {
	use super::*;

	pub fn new_partial(config: &Configuration) -> Result<Service, ServiceError> {
		let telemetry = config
			.telemetry_endpoints
			.clone()
			.filter(|x| !x.is_empty())
			.map(|endpoints| -> Result<_, sc_telemetry::Error> {
				let worker = TelemetryWorker::new(16)?;
				let telemetry = worker.handle().new_telemetry(endpoints);
				Ok((worker, telemetry))
			})
			.transpose()?;

		let executor = sc_service::new_wasm_executor(&config.executor);

		let (client, backend, keystore_container, task_manager) =
			sc_service::new_full_parts::<Block, RuntimeApi, _>(
				config,
				telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
				executor,
			)?;
		let client = Arc::new(client);

		let telemetry = telemetry.map(|(worker, telemetry)| {
			task_manager.spawn_handle().spawn("telemetry", None, worker.run());
			telemetry
		});

		let select_chain = sc_consensus::LongestChain::new(backend.clone());

		let transaction_pool = Arc::from(
			sc_transaction_pool::Builder::new(
				task_manager.spawn_essential_handle(),
				client.clone(),
				config.role.is_authority().into(),
			)
			.with_options(config.transaction_pool.clone())
			.with_prometheus(config.prometheus_registry())
			.build(),
		);

		let import_queue = sc_consensus_manual_seal::import_queue(
			Box::new(client.clone()),
			&task_manager.spawn_essential_handle(),
			config.prometheus_registry(),
		);

		Ok(PartialComponents {
			client,
			backend,
			task_manager,
			import_queue,
			keystore_container,
			select_chain,
			transaction_pool,
			other: telemetry,
		})
	}

	pub fn new_full<Network: sc_network::NetworkBackend<Block, <Block as BlockT>::Hash>>(
		config: Configuration,
		consensus: cli::Consensus,
	) -> Result<TaskManager, ServiceError> {
		let PartialComponents {
			client,
			backend,
			mut task_manager,
			import_queue,
			keystore_container,
			select_chain,
			transaction_pool,
			other: mut telemetry,
		} = new_partial(&config)?;

		let net_config = sc_network::config::FullNetworkConfiguration::<
			Block,
			<Block as BlockT>::Hash,
			Network,
		>::new(
			&config.network,
			config.prometheus_config.as_ref().map(|cfg| cfg.registry.clone()),
		);
		let metrics = Network::register_notification_metrics(
			config.prometheus_config.as_ref().map(|cfg| &cfg.registry),
		);

		let (network, system_rpc_tx, tx_handler_controller, sync_service) =
			sc_service::build_network(sc_service::BuildNetworkParams {
				config: &config,
				net_config,
				client: client.clone(),
				transaction_pool: transaction_pool.clone(),
				spawn_handle: task_manager.spawn_handle(),
				import_queue,
				block_announce_validator_builder: None,
				warp_sync_config: None,
				block_relay: None,
				metrics,
			})?;

		if config.offchain_worker.enabled {
			let offchain_workers =
				sc_offchain::OffchainWorkers::new(sc_offchain::OffchainWorkerOptions {
					runtime_api_provider: client.clone(),
					is_validator: config.role.is_authority(),
					keystore: Some(keystore_container.keystore()),
					offchain_db: backend.offchain_storage(),
					transaction_pool: Some(OffchainTransactionPoolFactory::new(
						transaction_pool.clone(),
					)),
					network_provider: Arc::new(network.clone()),
					enable_http_requests: true,
					custom_extensions: |_| vec![],
				})?;
			task_manager.spawn_handle().spawn(
				"offchain-workers-runner",
				"offchain-worker",
				offchain_workers.run(client.clone(), task_manager.spawn_handle()).boxed(),
			);
		}

		let rpc_extensions_builder = {
			let client = client.clone();
			let pool = transaction_pool.clone();

			Box::new(move |_| {
				let deps = rpc::FullDeps { client: client.clone(), pool: pool.clone() };
				rpc::create_full(deps).map_err(Into::into)
			})
		};

		let prometheus_registry = config.prometheus_registry().cloned();

		let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
			network,
			client: client.clone(),
			keystore: keystore_container.keystore(),
			task_manager: &mut task_manager,
			transaction_pool: transaction_pool.clone(),
			rpc_builder: rpc_extensions_builder,
			backend,
			system_rpc_tx,
			tx_handler_controller,
			sync_service,
			config,
			telemetry: telemetry.as_mut(),
			tracing_execute_block: None,
		})?;

		let proposer = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool.clone(),
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|x| x.handle()),
		);

		match consensus {
			cli::Consensus::InstantSeal => {
				let params = sc_consensus_manual_seal::InstantSealParams {
					block_import: client.clone(),
					env: proposer,
					client,
					pool: transaction_pool,
					select_chain,
					consensus_data_provider: None,
					create_inherent_data_providers: move |_, ()| async move {
						Ok(sp_timestamp::InherentDataProvider::from_system_time())
					},
				};

				let authorship_future = sc_consensus_manual_seal::run_instant_seal(params);

				task_manager.spawn_essential_handle().spawn_blocking(
					"instant-seal",
					None,
					authorship_future,
				);
			},
			cli::Consensus::ManualSeal(block_time) => {
				let (mut sink, commands_stream) = futures::channel::mpsc::channel(1024);
				task_manager.spawn_handle().spawn("block_authoring", None, async move {
					loop {
						futures_timer::Delay::new(std::time::Duration::from_millis(block_time)).await;
						if let Err(e) = sink.try_send(sc_consensus_manual_seal::EngineCommand::SealNewBlock {
							create_empty: true,
							finalize: true,
							parent_hash: None,
							sender: None,
						}) {
							log::error!("Failed to send seal command: {:?}", e);
						}
					}
				});

				let params = sc_consensus_manual_seal::ManualSealParams {
					block_import: client.clone(),
					env: proposer,
					client,
					pool: transaction_pool,
					select_chain,
					commands_stream: Box::pin(commands_stream),
					consensus_data_provider: None,
					create_inherent_data_providers: move |_, ()| async move {
						Ok(sp_timestamp::InherentDataProvider::from_system_time())
					},
				};
				let authorship_future = sc_consensus_manual_seal::run_manual_seal(params);

				task_manager.spawn_essential_handle().spawn_blocking(
					"manual-seal",
					None,
					authorship_future,
				);
			},
			_ => {},
		}

		Ok(task_manager)
	}
}

// --- CLI runner implementation ---
impl SubstrateCli for cli::Cli {
	fn impl_name() -> String {
		"Ignoto Protocol Node".into()
	}

	fn impl_version() -> String {
		env!("CARGO_PKG_VERSION").into()
	}

	fn description() -> String {
		env!("CARGO_PKG_DESCRIPTION").into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/ignoto-protocol/core".into()
	}

	fn copyright_start_year() -> i32 {
		2026
	}

	fn load_spec(&self, id: &str) -> Result<Box<dyn sc_service::ChainSpec>, String> {
		Ok(match id {
			"dev" => Box::new(chain_spec::development_chain_spec()?),
			path => Box::new(chain_spec::ChainSpec::from_json_file(std::path::PathBuf::from(path))?),
		})
	}
}

fn main() -> sc_cli::Result<()> {
	let cli = cli::Cli::from_args();

	match &cli.subcommand {
		Some(cli::Subcommand::Key(cmd)) => cmd.run(&cli),
		Some(cli::Subcommand::ExportChainSpec(cmd)) => {
			let chain_spec = cli.load_spec(&cmd.chain)?;
			cmd.run(chain_spec)
		},
		Some(cli::Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, import_queue, .. } =
					service::new_partial(&config)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		},
		Some(cli::Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, .. } = service::new_partial(&config)?;
				Ok((cmd.run(client, config.database), task_manager))
			})
		},
		Some(cli::Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, .. } = service::new_partial(&config)?;
				Ok((cmd.run(client, config.chain_spec), task_manager))
			})
		},
		Some(cli::Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, import_queue, .. } =
					service::new_partial(&config)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		},
		Some(cli::Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.database))
		},
		Some(cli::Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, backend, .. } =
					service::new_partial(&config)?;
				Ok((cmd.run(client, backend, None), task_manager))
			})
		},
		Some(cli::Subcommand::ChainInfo(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| {
				cmd.run::<Block>(&config)
			})
		},
		None => {
			let runner = cli.create_runner(&cli.run)?;
			runner.run_node_until_exit(|config| async move {
				match config.network.network_backend {
					sc_network::config::NetworkBackendType::Libp2p =>
						service::new_full::<sc_network::NetworkWorker<_, _>>(config, cli.consensus)
							.map_err(sc_cli::Error::Service),
					sc_network::config::NetworkBackendType::Litep2p => service::new_full::<
						sc_network::Litep2pNetworkBackend,
					>(config, cli.consensus)
					.map_err(sc_cli::Error::Service),
				}
			})
		},
	}
}
