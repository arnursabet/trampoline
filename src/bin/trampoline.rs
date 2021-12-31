use std::collections::HashMap;

use anyhow::anyhow;
use anyhow::Result;
use ckb_app_config::BlockAssemblerConfig;
use ckb_hash::blake2b_256;

use ckb_types::{h256, H256};

use structopt::StructOpt;

use trampoline::docker::*;
use trampoline::opts::{NetworkCommands, SchemaCommand, TrampolineCommand};
use trampoline::parse_hex;
use trampoline::project::*;
use trampoline::schema::{Schema, SchemaInitArgs};
use trampoline::TrampolineResource;
use trampoline::TrampolineResourceType;

const SECP_TYPE_HASH: H256 =
    h256!("0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8");
fn create_block_assembler_from_pkhash(hash: &[u8]) -> BlockAssemblerConfig {
    use ckb_jsonrpc_types::{JsonBytes, ScriptHashType};
    BlockAssemblerConfig {
        code_hash: SECP_TYPE_HASH,
        hash_type: ScriptHashType::Type,
        use_binary_version_as_message_prefix: false,
        args: JsonBytes::from_bytes(bytes::Bytes::copy_from_slice(hash)),
        message: JsonBytes::default(),
        binary_version: "".to_string(),
    }
}

fn main() -> Result<()> {
    let opts = TrampolineCommand::from_args();

    let project = TrampolineProject::load(std::env::current_dir()?);

    match opts {
        TrampolineCommand::NewProject { name } => match project {
            Ok(project) => {
                if let TrampolineResourceType::Project(project) = project {
                    return Err(TrampolineProjectError::ProjectAlreadyExists {
                        name: project.config.name.to_string(),
                        path: project.root_dir.as_path().to_str().unwrap().to_string(),
                    }
                    .into());
                }
            }
            Err(_e) => {
                let project = TrampolineProject::from(TrampolineProject::init(name)?);
                std::env::set_current_dir(&project.root_dir)?;
                Docker::default().build()?;
            }
        },
        TrampolineCommand::Schema { command } => {
            match command {
                SchemaCommand::New { name, def } => {
                    let mut schema_args: SchemaInitArgs =
                        (TrampolineProject::from(project?), name, None);
                    if def.is_some() {
                        let content = def.unwrap();
                        schema_args.2 = Some(content);
                    }

                    //println!("{:?}", schema_args);
                    let _result = Schema::init(schema_args)?;
                }
                SchemaCommand::Build { name } => {
                    let schema_args: SchemaInitArgs =
                        (TrampolineProject::from(project?), name, None);
                    let _result = Schema::init(schema_args)?;
                }
            }
        }
        TrampolineCommand::Network { command } => {
            let project = TrampolineProject::from(project?);
            match command {
                NetworkCommands::Launch {} => {
                    let image = DockerImage {
                        name: "iamm/trampoline-env".to_string(),
                        tag: Some("latest".to_string()),
                        file_path: Some(".".to_string()),
                        host_mappings: vec![],
                        build_args: HashMap::new(),
                    };

                    let cmd: DockerCommand<DockerImage> =
                        DockerCommand::default().build(&image, true).unwrap();
                    cmd.execute(None)?;

                    let container_port = project.config.env.as_ref().unwrap().chain.container_port;
                    let host_port = project.config.env.as_ref().unwrap().chain.host_port;

                    let host_volume = project
                        .config
                        .env
                        .as_ref()
                        .unwrap()
                        .chain
                        .local_binding
                        .as_path();

                    let container_volume = project
                        .config
                        .env
                        .as_ref()
                        .unwrap()
                        .chain
                        .container_mount
                        .as_str();

                    let docker_volume = Volume {
                        host: host_volume,
                        container: std::path::Path::new(container_volume),
                    };

                    let container = DockerContainer {
                        name: format!("{}-node", &project.config.name),
                        port_bindings: vec![DockerPort {
                            host: host_port,
                            container: container_port,
                        }],
                        volumes: vec![docker_volume],
                        env_vars: HashMap::default(),
                        image,
                    };
                    let run: DockerCommand<DockerContainer> = DockerCommand::default()
                        .run(&container, false, true)
                        .unwrap();

                    run.execute(Some(vec!["run".to_string()]))?;
                }
                NetworkCommands::SetMiner { pubkey, lock_arg } => {
                    let mut config = project.load_ckb_config()?;
                    if let Some(pubkey) = pubkey {
                        let pubkey_bytes = parse_hex(pubkey.as_str())?;
                        let pubkey_hash = blake2b_256(&pubkey_bytes);
                        let pkey_hash_slice = &pubkey_hash[0..20];
                        let block_assembler = create_block_assembler_from_pkhash(pkey_hash_slice);
                        config.block_assembler = Some(block_assembler);
                        project.save_ckb_config(config)?;
                    } else if let Some(lock_arg) = lock_arg {
                        let lock_arg_bytes = parse_hex(lock_arg.as_str())?;
                        let block_assembler = create_block_assembler_from_pkhash(&lock_arg_bytes);
                        config.block_assembler = Some(block_assembler);
                        project.save_ckb_config(config)?;
                    }
                    Docker::default()
                        .name(format!("{}-node", project.config.name.as_str()).as_str())
                        .restart()?;
                }
                NetworkCommands::Miner { one_block: _ } => {
                    let config = project.load_ckb_config()?;
                    let block_assembler_args = config.block_assembler.as_ref();
                    if block_assembler_args.is_none() {
                        return Err(anyhow!("No miner address set. Refer to `trampoline net set-miner --help` for more information."));
                    }
                    let container_name = project.config.name.as_str();
                    let miner_mount_path = &project.config.env.unwrap().miner.container_mount;
                    Docker::exec(
                        format!("{}-node", container_name).as_str(),
                        vec!["ckb", "miner"],
                        miner_mount_path,
                    )?;
                }
                NetworkCommands::Indexer {} => {}
                _ => {
                    println!("Command not yet implemented!");
                    std::process::exit(0);
                }
            }
        }
        _ => {
            println!("Unrecognized command. Use `trampoline --help` for usage information.")
        }
    }

    Ok(())
}
