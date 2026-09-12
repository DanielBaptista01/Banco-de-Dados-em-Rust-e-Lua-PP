use std::{fmt, fs, path::Path};

use mlua::{Function, Lua, Table};

use crate::storage::Store;

const RUNTIME: &str = r#"
local registered_extensions = {}

function register_extension(extension)
    if type(extension) ~= "table" then
        error("o registro da extensão deve ser uma tabela")
    end

    local prefix = extension.prefix
    if type(prefix) ~= "string" or prefix == "" or prefix:find("%s") then
        error("o campo 'prefix' deve ser uma string não vazia e sem espaços")
    end

    if extension.on_add ~= nil and type(extension.on_add) ~= "function" then
        error("o campo 'on_add', quando presente, deve ser uma função")
    end

    if extension.on_get ~= nil and type(extension.on_get) ~= "function" then
        error("o campo 'on_get', quando presente, deve ser uma função")
    end

    if extension.on_add == nil and extension.on_get == nil then
        error("a extensão deve implementar 'on_add', 'on_get' ou ambos")
    end

    for _, current in ipairs(registered_extensions) do
        if current.prefix == prefix then
            error("prefixo já registrado: " .. prefix)
        end
    end

    table.insert(registered_extensions, extension)
    table.sort(registered_extensions, function(left, right)
        return #left.prefix > #right.prefix
    end)
end

local function validate_result(result)
    if type(result) ~= "table" or type(result.ok) ~= "boolean" then
        error("a extensão deve retornar uma tabela com o campo booleano 'ok'")
    end

    if result.ok then
        if type(result.value) ~= "string" then
            error("um resultado de sucesso deve conter o campo string 'value'")
        end
    elseif type(result.error) ~= "string" or result.error == "" then
        error("um resultado de falha deve conter o campo string não vazio 'error'")
    end

    return result
end

function dispatch_extension(operation, key, value)
    local selected = nil

    for _, extension in ipairs(registered_extensions) do
        if key:sub(1, #extension.prefix) == extension.prefix then
            selected = extension
            break
        end
    end

    if selected == nil then
        return { ok = true, value = value }
    end

    local handler = nil
    if operation == "ADD" then
        handler = selected.on_add
    elseif operation == "GET" then
        handler = selected.on_get
    else
        error("operação desconhecida enviada à VM")
    end

    if handler == nil then
        return { ok = true, value = value }
    end

    return validate_result(handler(key, value))
end
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExtensionResult {
    Success(String),
    Failure(String),
}

#[derive(Debug)]
pub(crate) struct BridgeError(String);

impl BridgeError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for BridgeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for BridgeError {}

pub(crate) struct LuaBridge {
    lua: Lua,
}

impl LuaBridge {
    pub(crate) fn load(directory: &Path, store: Store) -> Result<Self, BridgeError> {
        let lua = Lua::new();

        let get_store = store.clone();
        let db_get = lua
            .create_function(move |_, key: String| Ok(get_store.get(&key)))
            .map_err(|error| BridgeError::new(error.to_string()))?;
        lua.globals()
            .set("db_get", db_get)
            .map_err(|error| BridgeError::new(error.to_string()))?;

        let search_store = store;
        let db_find_key_by_value = lua
            .create_function(move |_, (value, except_key): (String, Option<String>)| {
                Ok(search_store.find_key_by_value(&value, except_key.as_deref()))
            })
            .map_err(|error| BridgeError::new(error.to_string()))?;
        lua.globals()
            .set("db_find_key_by_value", db_find_key_by_value)
            .map_err(|error| BridgeError::new(error.to_string()))?;

        lua.load(RUNTIME)
            .set_name("runtime de extensões")
            .exec()
            .map_err(|error| BridgeError::new(error.to_string()))?;

        let mut files = fs::read_dir(directory)
            .map_err(|error| {
                BridgeError::new(format!(
                    "não foi possível ler {}: {error}",
                    directory.display()
                ))
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| BridgeError::new(format!("falha ao listar extensões: {error}")))?;

        files.sort_by_key(|entry| entry.path());

        for entry in files {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("lua") {
                continue;
            }

            let source = fs::read_to_string(&path).map_err(|error| {
                BridgeError::new(format!("não foi possível ler {}: {error}", path.display()))
            })?;

            lua.load(&source)
                .set_name(path.to_string_lossy())
                .exec()
                .map_err(|error| {
                    BridgeError::new(format!("falha ao carregar {}: {error}", path.display()))
                })?;
        }

        Ok(Self { lua })
    }

    pub(crate) fn dispatch(
        &self,
        operation: &str,
        key: &str,
        value: &str,
    ) -> Result<ExtensionResult, BridgeError> {
        let dispatcher: Function = self
            .lua
            .globals()
            .get("dispatch_extension")
            .map_err(|error| BridgeError::new(error.to_string()))?;

        let response: Table = dispatcher
            .call((operation, key, value))
            .map_err(|error| BridgeError::new(error.to_string()))?;

        let succeeded: bool = response
            .get("ok")
            .map_err(|error| BridgeError::new(format!("retorno inválido da extensão: {error}")))?;

        if succeeded {
            let transformed: String = response.get("value").map_err(|error| {
                BridgeError::new(format!("retorno inválido da extensão: {error}"))
            })?;
            Ok(ExtensionResult::Success(transformed))
        } else {
            let reason: String = response.get("error").map_err(|error| {
                BridgeError::new(format!("retorno inválido da extensão: {error}"))
            })?;
            Ok(ExtensionResult::Failure(reason))
        }
    }
}
