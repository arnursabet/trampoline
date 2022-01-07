use crate::project::TrampolineProject;
use crate::{TrampolineResource, TrampolineResourceType};
use anyhow::Result;
use molecule_codegen::{Compiler, Language};

use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;

type SchemaResult<T> = std::result::Result<T, SchemaError>;

#[derive(Debug, Error)]
pub enum SchemaError {
    #[error("Error occurred: {0:?}")]
    Io(#[from] std::io::Error),
    #[error("Error compiling molecule schema file:\n {0}")]
    Molecule(String),
}

#[derive(Debug, Clone, Default)]
pub struct Schema {
    _name: String,
    _path: PathBuf,
}

impl From<Schema> for TrampolineResourceType {
    fn from(p: Schema) -> Self {
        Self::Schema(p)
    }
}

pub type SchemaInitArgs = (TrampolineProject, String, Option<String>);

impl TrampolineResource for Schema {
    type Error = SchemaError;
    type InitArgs = SchemaInitArgs;

    fn load(path: impl AsRef<Path>) -> Result<TrampolineResourceType, Self::Error> {
        let name = path
            .as_ref()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        Ok(Schema {
            _name: name,
            _path: path.as_ref().to_path_buf(),
        }
        .into())
    }

    fn init(args: Self::InitArgs) -> Result<TrampolineResourceType, Self::Error> {
        let (proj, name, content) = args;
        let mut schema_dir = proj.root_dir;
        schema_dir.push("schemas");

        let mut gen_bindings_flag = content.is_some();
        schema_dir.push("mol");
        schema_dir.push(&format!("{}.mol", name));
        // If the dir exists, then this indicates that init is being used to
        // generate rust bindings rather than generate a new schema.
        // If dir does not exist, then this indicates creation of a new schema definition
        if schema_dir.exists() {
            println!("Schema file: {:?} exists.\n", schema_dir);
            gen_bindings_flag = true;
        } else {
            println!("Creating {}", &schema_dir.as_path().to_str().unwrap());
            std::fs::write(&schema_dir, content.unwrap_or_else(|| "".to_string()))?;
        }

        let molecule_file_path = schema_dir.clone().canonicalize()?;
        //println!("MOL PATH: {:?}", molecule_file_path);
        schema_dir.pop();
        schema_dir.pop();
        schema_dir.push("src");

        let target_bindings_file_path = schema_dir.clone().canonicalize()?;

        // Go back to src dir
        //println!("TARGET path: {:?}", target_bindings_file_path);

        schema_dir.push("lib.rs");
        if gen_bindings_flag {
            println!(
                "Generating Rust bindings for schema at {}/{}.rs\n",
                &target_bindings_file_path.as_path().to_str().unwrap(),
                name
            );
            gen_bindings(&molecule_file_path, &target_bindings_file_path)?;
            println!("Adding module {} to lib.rs", name);
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .append(true)
                .open(&schema_dir)?;
            f.write_all(format!("mod {};\n", name).as_bytes())?;
        }

        Ok(Schema {
            _path: molecule_file_path,
            _name: name,
        }
        .into())
    }
}

// pub fn new(name: &str, project: &TrampolineProject) -> Result<Self> {
//
// }

pub fn gen_bindings(input: impl Into<PathBuf>, output: impl Into<PathBuf>) -> SchemaResult<()> {
    let mut compiler = Compiler::new();
    compiler.input_schema_file(input.into().as_path());
    compiler.output_dir(output.into().as_path());
    compiler.generate_code(Language::Rust);
    compiler.run().map_err(SchemaError::Molecule)
}
