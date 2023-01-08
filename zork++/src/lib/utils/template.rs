use crate::config_cli::{CliArgs, CppCompiler};
use crate::utils::constants::autogenerated_example;
use log::info;
use std::io::Write;
use std::process::Command;
use std::{
    fs::{DirBuilder, File},
    path::Path,
};

/// Generates a new C++ standarized empty base project
/// with a pre-designed structure to organize the
/// user code in a modern fashion way.
///
/// Base template for the project files and folders:
///    - ./ifc/<project_name>
///        - math.<extension>
///    - ./src/<project_name>
///       - math.<extension>
///       - math2.<extension>
///    - main.cpp
///    - test
///    - dependencies
pub fn create_templated_project(cli_args: &CliArgs) {
    let example_dir_name = Path::new(autogenerated_example::ROOT_PATH_NAME);
    info!("Creating the autogenerated template project");

    let compiler = cli_args.compiler.as_ref().unwrap_or(&CppCompiler::CLANG);
    let path_ifc = example_dir_name.join("ifc");
    let path_src = example_dir_name.join("src");
    let path_test = example_dir_name.join("test");
    let path_dependencies = example_dir_name.join("deps");

    create_directory(example_dir_name);
    create_directory(&path_ifc);
    create_directory(&path_src);
    create_directory(&path_test);
    create_directory(&path_dependencies);

    create_file(
        &path_ifc,
        &format!("{}.{}", "math", compiler.get_default_extesion()),
        autogenerated_example::IFC_MOD_FILE.as_bytes(),
    );
    create_file(
        &path_src,
        "main.cpp", // TODO from constants
        autogenerated_example::MAIN.as_bytes(),
    );
    create_file(
        &path_src,
        "math.cpp",
        autogenerated_example::SRC_MOD_FILE.as_bytes(),
    );
    create_file(
        &path_src,
        "math2.cpp",
        autogenerated_example::SRC_MOD_FILE_2.as_bytes(),
    );

    // TODO The replaces must dissapear in the next PR
    let mut zork_conf = autogenerated_example::CONFIG_FILE
        .replace("<project_name>", autogenerated_example::PROJECT_NAME)
        .replace("<autog_test>", autogenerated_example::PROJECT_NAME)
        .replace(
            "<autogenerated_executable>",
            autogenerated_example::PROJECT_NAME,
        );

    if cfg!(windows) {
        zork_conf = zork_conf.replace("libcpp", "stdlib")
    }
    create_file(
        Path::new(autogenerated_example::ROOT_PATH_NAME),
        autogenerated_example::CONFIG_FILE_NAME,
        zork_conf.as_bytes(),
    );

    if cli_args.git {
        Command::new("git")
            .current_dir(autogenerated_example::ROOT_PATH_NAME)
            .arg("init")
            .spawn()
            .expect("Error initializing a new GIT repository");
    }
}

fn create_file<'a>(path: &Path, filename: &'a str, buff_write: &'a [u8]) {
    let mut file = File::create(path.join(filename))
        .unwrap_or_else(|_| panic!("Error creating the example file: {filename}",));

    file.write_all(buff_write)
        .unwrap_or_else(|_| panic!("Error writting the example file: {filename}",));
}

fn create_directory(path_create: &Path) {
    DirBuilder::new()
        .recursive(true)
        .create(path_create)
        .unwrap_or_else(|_| panic!("Error creating directory: {:?}", path_create.as_os_str()))
}
