use std::{
    path::{Path, PathBuf},
    process::ExitStatus,
};
use std::collections::HashMap;

use crate::bounds::TranslationUnit;
///! Contains helpers and data structure to process in
/// a nice and neat way the commands generated to be executed
/// by Zork++
use crate::{
    cache::{self, ZorkCache},
    project_model::{compiler::CppCompiler, ZorkModel},
    utils::constants,
};
use color_eyre::{
    eyre::{eyre, Context},
    Report, Result,
};
use serde::{Deserialize, Serialize};

use super::arguments::Argument;

pub fn run_generated_commands(
    program_data: &ZorkModel<'_>,
    mut commands: Commands<'_>,
    cache: ZorkCache,
) -> Result<CommandExecutionResult> {
    log::info!("Proceeding to execute the generated commands...");
    let mut total_exec_commands = 0;
    let compiler = commands.compiler;

    for sys_module in &commands.system_modules {
        execute_command(compiler, sys_module.1, &cache)?;
    }

    for miu in commands.interfaces.iter_mut() {
        if !miu.processed {
            let r = execute_command(compiler, &miu.args, &cache);
            miu.execution_result = CommandExecutionResult::from(&r);
            total_exec_commands += 1;
            if let Err(e) = r {
                cache::save(program_data, cache, commands)?;
                return Err(e);
            } else if !r.as_ref().unwrap().success() {
                let c_miu = miu.clone();
                cache::save(program_data, cache, commands)?;
                return Err(eyre!(
                    "Ending the program, because the build of: {:?} wasn't ended successfully",
                    c_miu.file
                ));
            }
        }
    }

    for implm in &mut commands.implementations {
        if !implm.processed {
            let r = execute_command(compiler, &implm.args, &cache);
            implm.execution_result = CommandExecutionResult::from(&r);
            total_exec_commands += 1;
            if let Err(e) = r {
                return Err(e);
            } else if !r.as_ref().unwrap().success() {
                let c_miu = implm.clone();
                return Err(eyre!(
                    "Ending the program, because the build of: {:?} wasn't ended successfully",
                    c_miu.file
                ));
            }
        }
    }

    for source in &mut commands.sources {
        if !source.processed {
            let r = execute_command(compiler, &source.args, &cache);
            source.execution_result = CommandExecutionResult::from(&r);
            total_exec_commands += 1;
            if let Err(e) = r {
                return Err(e);
            } else if !r.as_ref().unwrap().success() {
                let c_miu = source.clone();
                return Err(eyre!(
                    "Ending the program, because the build of: {:?} wasn't ended successfully",
                    c_miu.file
                ));
            }
        }
    }

    if !commands.main.args.is_empty() {
        log::debug!("Executing the main command line...");

        let r = execute_command(compiler, &commands.main.args, &cache);
        commands.main.execution_result = CommandExecutionResult::from(&r);
        total_exec_commands += 1;

        if let Err(e) = r {
            cache::save(program_data, cache, commands)?;
            return Err(e);
        } else if !r.as_ref().unwrap().success() {
            cache::save(program_data, cache, commands)?;
            return Err(eyre!(
                "Ending the program, because the main command line execution wasn't ended successfully",
            ));
        }
    }

    log::info!("A total of: {total_exec_commands} has been successfully executed");
    cache::save(program_data, cache, commands)?;
    Ok(CommandExecutionResult::Success)
}

/// Executes a new [`std::process::Command`] to run the generated binary
/// after the build process in the specified shell
pub fn autorun_generated_binary(
    compiler: &CppCompiler,
    output_dir: &Path,
    executable_name: &str,
) -> Result<CommandExecutionResult> {
    let args = &[Argument::from(
        output_dir
            .join(compiler.as_ref())
            .join(executable_name)
            .with_extension(constants::BINARY_EXTENSION),
    )];

    log::info!(
        "[{compiler}] - Executing the generated binary => {:?}",
        args.join(" ")
    );

    Ok(CommandExecutionResult::from(
        std::process::Command::new(Argument::from(
            output_dir.join(compiler.as_ref()).join(executable_name),
        ))
        .spawn()?
        .wait()
        .with_context(|| format!("[{compiler}] - Command {:?} failed!", args.join(" "))),
    ))
}

/// Executes a new [`std::process::Command`] configured according the choosen
/// compiler and the current operating system
fn execute_command(
    compiler: CppCompiler,
    arguments: &[Argument<'_>],
    cache: &ZorkCache,
) -> Result<ExitStatus, Report> {
    log::trace!(
        "[{compiler}] - Executing command => {:?}",
        format!("{} {}", compiler.get_driver(), arguments.join(" "))
    );

    if compiler.eq(&CppCompiler::MSVC) {
        std::process::Command::new(
            cache
                .compilers_metadata
                .msvc
                .dev_commands_prompt
                .as_ref()
                .expect("Zork++ wasn't able to found a correct installation of MSVC"),
        )
        .arg("&&")
        .arg(compiler.get_driver())
        .args(arguments)
        .spawn()?
        .wait()
        .with_context(|| format!("[{compiler}] - Command {:?} failed!", arguments.join(" ")))
    } else {
        std::process::Command::new(compiler.get_driver())
            .args(arguments)
            .spawn()?
            .wait()
            .with_context(|| format!("[{compiler}] - Command {:?} failed!", arguments.join(" ")))
    }
}

/// The pieces and details for the generated command line for
/// for some translation unit
#[derive(Debug, Clone)]
pub struct SourceCommandLine<'a> {
    pub directory: PathBuf,
    pub file: String,
    pub args: Vec<Argument<'a>>,
    pub processed: bool,
    pub execution_result: CommandExecutionResult,
}

impl<'a> SourceCommandLine<'a> {
    pub fn from_translation_unit(
        tu: impl TranslationUnit,
        args: Vec<Argument<'a>>,
        processed: bool,
        execution_result: CommandExecutionResult,
    ) -> Self {
        Self {
            directory: tu.path(),
            file: tu.file_with_extension(),
            args,
            processed,
            execution_result,
        }
    }

    pub fn path(&self) -> PathBuf {
        self.directory.join(Path::new(&self.file))
    }
}

impl<'a> IntoIterator for SourceCommandLine<'a> {
    type Item = Argument<'a>;
    type IntoIter = std::vec::IntoIter<Argument<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        self.args.into_iter()
    }
}

#[derive(Debug)]
pub struct ExecutableCommandLine<'a> {
    pub main: &'a Path,
    pub sources_paths: Vec<PathBuf>,
    pub args: Vec<Argument<'a>>,
    pub execution_result: CommandExecutionResult,
}

impl<'a> Default for ExecutableCommandLine<'a> {
    fn default() -> Self {
        Self {
            main: Path::new("."),
            sources_paths: Vec::with_capacity(0),
            args: Vec::with_capacity(0),
            execution_result: Default::default(),
        }
    }
}

/// Holds the generated command line arguments for a concrete compiler
#[derive(Debug)]
pub struct Commands<'a> {
    pub compiler: CppCompiler,
    pub system_modules: HashMap<String, Vec<Argument<'a>>>,
    pub interfaces: Vec<SourceCommandLine<'a>>,
    pub implementations: Vec<SourceCommandLine<'a>>,
    pub sources: Vec<SourceCommandLine<'a>>,
    pub main: ExecutableCommandLine<'a>,
    pub generated_files_paths: Vec<Argument<'a>>,
}

impl<'a> Commands<'a> {
    pub fn new(compiler: &'a CppCompiler) -> Self {
        Self {
            compiler: *compiler,
            system_modules: HashMap::with_capacity(0),
            interfaces: Vec::with_capacity(0),
            implementations: Vec::with_capacity(0),
            sources: Vec::with_capacity(0),
            main: ExecutableCommandLine::default(),
            generated_files_paths: Vec::with_capacity(0),
        }
    }
}

impl<'a> core::fmt::Display for Commands<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Commands for [{}]:\n- Interfaces: {:?},\n- Implementations: {:?},\n- Sources: {:?}",
            self.compiler,
            self.interfaces.iter().map(|vec| { vec.args.iter().map(|e| e.value).collect::<Vec<_>>().join(" "); }),
            self.implementations.iter().map(|vec| { vec.args.iter().map(|e| e.value).collect::<Vec<_>>().join(" "); }),
            self.sources.iter().map(|vec| { vec.args.iter().map(|e| e.value).collect::<Vec<_>>().join(" "); }),
        )
    }
}

/// Holds a custom representation of the execution of
/// a command line in a shell.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub enum CommandExecutionResult {
    /// A command that is executed correctly
    Success,
    /// A skipped command due to previous successful iterations
    Cached,
    /// A command which is return code indicates an unsuccessful execution
    Failed,
    /// The execution failed, returning a [`Result`] with the Err variant
    Error,
    /// A previous state before executing a command line
    #[default]
    Unreached,
}

impl From<Result<ExitStatus, Report>> for CommandExecutionResult {
    fn from(value: Result<ExitStatus, Report>) -> Self {
        match value {
            Ok(r) => {
                if r.success() {
                    CommandExecutionResult::Success
                } else {
                    CommandExecutionResult::Failed
                }
            }
            Err(_) => CommandExecutionResult::Error,
        }
    }
}

impl From<&Result<ExitStatus, Report>> for CommandExecutionResult {
    fn from(value: &Result<ExitStatus, Report>) -> Self {
        match value {
            Ok(r) => {
                if r.success() {
                    CommandExecutionResult::Success
                } else {
                    CommandExecutionResult::Failed
                }
            }
            Err(_) => CommandExecutionResult::Error,
        }
    }
}
