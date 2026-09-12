# Integrantes Daniel Santos Baptista e Isaias Maia de Oliveira

# Banco de Dados em Memória — Rust + Lua

Banco chave-valor volátil, inspirado em Redis e Memcached, implementado em Rust e extensível em tempo de execução por scripts Lua. O programa aceita somente os comandos `ADD`, `GET` e `EXIT`; os dados existem apenas durante a execução.

O núcleo em Rust não conhece extensões específicas. Todos os arquivos `*.lua` encontrados em `extensions/` são carregados no início, registram seus próprios prefixos e podem validar ou transformar valores sem alterar nem recompilar o motor.

## Requisitos e compilação

- Rust 1.88 ou mais recente, com Cargo.
- Não é necessário instalar Lua: a dependência `mlua` compila Lua 5.4 junto com o projeto por meio da feature `vendored`.

Na raiz do repositório:

```bash
cargo build --release
```

O executável será criado em `target/release/banco-memoria` no Linux/macOS e em `target\release\banco-memoria.exe` no Windows.

## Execução

Modo de desenvolvimento:

```bash
cargo run
```

Executável compilado no Linux/macOS:

```bash
./target/release/banco-memoria
```

Executável compilado no Windows PowerShell:

```powershell
.\target\release\banco-memoria.exe
```

O programa deve ser iniciado na raiz do projeto, onde se encontra o diretório `extensions/`.

Exemplo interativo:

```text
> ADD nome_usuario Maria da Silva
OK
> GET nome_usuario
Maria da Silva
> EXIT
```

Também é possível fornecer comandos por *pipe*:

```bash
cargo build
cat roteiro.txt | ./target/debug/banco-memoria
```

No PowerShell:

```powershell
Get-Content roteiro.txt | .\target\debug\banco-memoria.exe
```

O fim da entrada encerra a execução da mesma forma que `EXIT`.

## Comandos

| Comando | Resultado |
|---|---|
| `ADD chave valor` | Valida e grava o valor; responde `OK`. O valor pode conter espaços. |
| `GET chave` | Busca, formata quando houver extensão e imprime o valor. |
| `EXIT` | Encerra o processo. |

Os comandos são sensíveis a maiúsculas e minúsculas. Entradas incompletas, comandos desconhecidos, chaves inexistentes e falhas de extensões produzem uma linha iniciada por `ERRO:` e não encerram o programa. Uma linha em branco apenas apresenta o prompt seguinte.

## Arquitetura e dependências

| Módulo | Responsabilidade | Depende de |
|---|---|---|
| `src/main.rs` | Localiza `extensions/`, monta o motor e inicia o terminal. | `engine`, `cli` |
| `src/cli.rs` | Imprime o prompt, lê linhas, escreve respostas e trata EOF. | `command`, `engine` |
| `src/command.rs` | Converte texto em `Command::Add`, `Command::Get` ou `Command::Exit`. | Nenhum outro módulo do projeto |
| `src/storage.rs` | Mantém o `HashMap<String, String>` compartilhado e oferece consultas genéricas. | Biblioteca padrão |
| `src/lua_bridge.rs` | Inicializa a VM, carrega scripts, registra extensões e converte retornos Lua. | `storage`, `mlua` |
| `src/engine.rs` | Coordena validação, leitura e gravação transacional. | `command`, `storage`, `lua_bridge` |

Somente `src/lua_bridge.rs` importa o crate `mlua`. O armazenamento, o terminal e o interpretador de comandos não sabem que Lua existe. O fluxo de uma escrita é:

```text
linha -> parser -> motor -> Lua valida/transforma -> Rust grava -> resposta
```

A gravação ocorre somente depois de a extensão retornar sucesso. Portanto, se uma tentativa de sobrescrita for inválida, o valor anterior continua intacto.

## Protocolo de extensões

Cada arquivo `.lua` executado no início chama a função global `register_extension` exatamente uma vez, passando uma tabela com:

| Campo | Tipo | Obrigatório | Significado |
|---|---|---:|---|
| `prefix` | `string` não vazia e sem espaços | Sim | Prefixo das chaves tratadas. |
| `on_add` | `function(key, value)` | Uma das funções | Valida ou transforma antes da gravação. |
| `on_get` | `function(key, value)` | Uma das funções | Formata ou transforma depois da leitura. |

As duas funções recebem a chave e o valor como strings. Elas devem devolver uma destas estruturas:

```lua
-- sucesso: o campo value também precisa ser string
return { ok = true, value = "valor que volta ao Rust" }

-- falha: error precisa ser uma string não vazia
return { ok = false, error = "motivo da rejeição" }
```

No Rust, a resposta é convertida para a enumeração específica:

```rust
pub enum ExtensionResult {
    Success(String),
    Failure(String),
}
```

`Success` no `ADD` contém o valor realmente armazenado; no `GET`, contém o texto exibido. `Failure` é convertido em `ERRO: <motivo>`. Erros de execução do script, registros inválidos ou retornos fora do contrato também são capturados e reportados sem `panic`.

Se nenhuma extensão reconhecer a chave, a VM devolve o valor sem alteração. Se dois prefixos puderem reconhecer a mesma chave, vence o prefixo mais longo. Prefixos idênticos são rejeitados durante a inicialização. Os arquivos são carregados em ordem alfabética para tornar o boot determinístico.

### Consulta ao banco a partir de Lua

A ponte instala duas funções genéricas na VM:

```lua
local value_or_nil = db_get("uma_chave")
local key_or_nil = db_find_key_by_value("valor", "chave_a_ignorar")
```

- `db_get(key)` consulta o valor atual de uma chave.
- `db_find_key_by_value(value, except_key)` procura uma chave que contenha o valor. O segundo argumento pode ser `nil`; quando informado, essa chave é ignorada.

As funções capturam uma referência compartilhada ao armazenamento (`Rc<RefCell<HashMap<...>>>`), não uma cópia. Cada chamada Lua faz um empréstimo imutável curto e observa o estado atual. O motor não mantém um empréstimo mutável enquanto Lua executa: primeiro ocorre a validação e, depois que ela termina, o valor aprovado é gravado. Isso resolve a consulta durante `ADD` sem copiar toda a base, sem conflito de empréstimos e sem rollback.

## Como criar uma nova extensão

1. Crie um arquivo com extensão `.lua` dentro de `extensions/`.
2. Implemente `on_add`, `on_get` ou ambas, sempre usando o contrato de retorno acima.
3. No final do arquivo, chame `register_extension` com um prefixo ainda não utilizado.
4. Quando precisar do estado atual, use `db_get` ou `db_find_key_by_value`; essas consultas não devem modificar o banco.
5. Reinicie o executável. Não altere nem recompile o Rust.

Exemplo completo:

```lua
local function validate(_, value)
    if value == "" then
        return { ok = false, error = "o valor não pode ser vazio" }
    end
    return { ok = true, value = value }
end

local function format(_, value)
    return { ok = true, value = "[" .. value .. "]" }
end

register_extension({
    prefix = "exemplo_",
    on_add = validate,
    on_get = format,
})
```

Após salvar o script, `ADD exemplo_item texto` grava `texto` e `GET exemplo_item` exibe `[texto]`. O motor Rust permanece inalterado.

## Extensões entregues

### CPF (`cpf_`)

No `ADD`, aceita somente 11 algarismos, rejeita sequências repetidas, calcula manualmente os dois dígitos verificadores e usa `db_find_key_by_value` para impedir o mesmo CPF em chaves diferentes. A própria chave é ignorada na busca, permitindo regravar o mesmo CPF nela. No `GET`, transforma os algarismos em `000.000.000-00`.

### Data (`data_`)

No `ADD`, exige exatamente `aaaa-mm-dd`, valida mês, quantidade de dias e a regra gregoriana completa de ano bissexto: divisível por 400 ou divisível por 4 sem ser divisível por 100. No `GET`, transforma para `dd/mm/aaaa`.

### Temperatura (`temperatura_`)

Esta é a extensão proposta pelo projeto. Ela aceita temperaturas em Celsius com no máximo duas casas decimais, rejeita valores inferiores ao zero absoluto (`-273,15 °C`) e normaliza a gravação para duas casas. No `GET`, calcula e exibe a conversão para Fahrenheit.

Ela foi escolhida porque exercita recursos não presentes nas extensões obrigatórias: conversão numérica, limite físico, normalização decimal e transformação aritmética entre unidades, em vez de somente regras de documento ou calendário.

Exemplo:

```text
> ADD temperatura_sala 25.5
OK
> GET temperatura_sala
25.50 °C = 77.90 °F
```

Os testes próprios estão em `casos_teste_temperatura.txt`.

## Testes e verificações

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
python3 -m unittest discover -s tests -p "test_*.py"
```

- `casos_teste.txt` preserva o roteiro cumulativo fornecido no enunciado.
- Os testes unitários Rust cobrem parser, armazenamento, passagem transparente e chave inexistente.
- `tests/test_acceptance.py` executa o binário real por *pipe* e cobre CPF, data, chaves comuns, comandos inválidos, sobrescrita transacional e a extensão proposta.
- O workflow em `.github/workflows/ci.yml` repete formatação, Clippy e todos os testes a cada `push` e *pull request*.

## Decisões de projeto

- O despacho por prefixo acontece dentro da VM Lua; todas as operações válidas de `ADD` e todo `GET` de chave existente passam pelo despachante, mesmo quando não há extensão correspondente.
- As extensões transformam valores, mas não escrevem diretamente no banco. O Rust continua sendo o único responsável pelo *commit*.
- O armazenamento usa interior mutability para permitir callbacks de consulta vivos e de curta duração durante a execução Lua.
- A API oferecida a Lua é deliberadamente genérica e não contém conceitos de CPF, data ou temperatura.
- Dependências de validação e formatação não são usadas; as regras ficam implementadas manualmente nos scripts.

## Uso de ferramenta generativa

O projeto teve apoio do ChatGPT/Codex na discussão da arquitetura, elaboração inicial de código, documentação e testes. Todo o comportamento permanece explicitado neste README e deve ser compreendido, revisado e validado pelos integrantes antes da entrega, conforme a política da disciplina.

