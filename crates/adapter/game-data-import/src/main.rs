use std::{env, process::ExitCode};

use game_data_import::{generate_to_path, parse_cli_args};

const USAGE: &str = "usage: game-data-import --source DIR --output FILE --version-group IDENTIFIER [--locale zh-Hans] [--source-commit SHA]";

fn main() -> ExitCode {
    let options = match parse_cli_args(env::args().skip(1)) {
        Ok(Some(options)) => options,
        Ok(None) => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            eprintln!("{error}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    match generate_to_path(&options.source, &options.output, &options.import) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
