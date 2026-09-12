use std::path::Path;

use banco_memoria::{cli, engine::Engine};

fn main() {
    let extensions_dir = Path::new("extensions");

    match Engine::new(extensions_dir) {
        Ok(engine) => {
            if let Err(error) = cli::run(engine) {
                eprintln!("ERRO: falha de entrada/saída: {error}");
            }
        }
        Err(error) => eprintln!("ERRO: não foi possível iniciar o banco: {error}"),
    }
}

