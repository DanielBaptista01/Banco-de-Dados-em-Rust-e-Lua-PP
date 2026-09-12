use std::path::Path;

use crate::{
    command::Command,
    lua_bridge::{BridgeError, ExtensionResult, LuaBridge},
    storage::Store,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineResponse {
    Continue(String),
    Exit,
}

pub struct Engine {
    store: Store,
    extensions: LuaBridge,
}

impl Engine {
    pub fn new(extensions_directory: &Path) -> Result<Self, String> {
        let store = Store::new();
        let extensions = LuaBridge::load(extensions_directory, store.clone())
            .map_err(|error: BridgeError| error.to_string())?;
        Ok(Self { store, extensions })
    }

    pub fn execute(&self, command: Command) -> EngineResponse {
        match command {
            Command::Add { key, value } => self.add(key, value),
            Command::Get { key } => self.get(key),
            Command::Exit => EngineResponse::Exit,
        }
    }

    fn add(&self, key: String, value: String) -> EngineResponse {
        match self.extensions.dispatch("ADD", &key, &value) {
            Ok(ExtensionResult::Success(transformed)) => {
                self.store.insert(key, transformed);
                EngineResponse::Continue("OK".to_string())
            }
            Ok(ExtensionResult::Failure(reason)) => EngineResponse::Continue(error_line(&reason)),
            Err(error) => EngineResponse::Continue(error_line(&format!("falha na extensão: {error}"))),
        }
    }

    fn get(&self, key: String) -> EngineResponse {
        let Some(value) = self.store.get(&key) else {
            return EngineResponse::Continue("ERRO: chave inexistente".to_string());
        };

        match self.extensions.dispatch("GET", &key, &value) {
            Ok(ExtensionResult::Success(transformed)) => EngineResponse::Continue(transformed),
            Ok(ExtensionResult::Failure(reason)) => EngineResponse::Continue(error_line(&reason)),
            Err(error) => EngineResponse::Continue(error_line(&format!("falha na extensão: {error}"))),
        }
    }
}

fn error_line(reason: &str) -> String {
    let one_line = reason.lines().collect::<Vec<_>>().join(" | ");
    format!("ERRO: {one_line}")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::command::Command;

    use super::{Engine, EngineResponse};

    fn engine() -> Engine {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("extensions");
        Engine::new(&directory).expect("as extensões do projeto devem carregar")
    }

    #[test]
    fn stores_unmatched_keys_without_transformation() {
        let engine = engine();
        assert_eq!(
            engine.execute(Command::Add {
                key: "plain_key".to_string(),
                value: "value with spaces".to_string(),
            }),
            EngineResponse::Continue("OK".to_string())
        );
        assert_eq!(
            engine.execute(Command::Get {
                key: "plain_key".to_string(),
            }),
            EngineResponse::Continue("value with spaces".to_string())
        );
    }

    #[test]
    fn reports_missing_key_without_stopping() {
        let engine = engine();
        assert_eq!(
            engine.execute(Command::Get {
                key: "missing_key".to_string(),
            }),
            EngineResponse::Continue("ERRO: chave inexistente".to_string())
        );
    }
}
