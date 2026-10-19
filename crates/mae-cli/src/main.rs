use std::env;
use std::process;

use mae_cli::execute_cli_args;

fn main() {
    let args: Vec<String> = env::args().collect();
    match execute_cli_args(&args) {
        Ok(output) => {
            println!("{output}");
        }
        Err(err) => {
            eprintln!("MAE Error: {err}");
            process::exit(1);
        }
    }
}
