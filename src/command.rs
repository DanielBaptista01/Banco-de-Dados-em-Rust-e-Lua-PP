#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Add { key: String, value: String },
    Get { key: String },
    Exit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedLine {
    Empty,
    Command(Command),
}

pub fn parse(line: &str) -> Result<ParsedLine, String> {
    let line = line.trim_end_matches(|character| character == '\r' || character == '\n');

    if line.trim().is_empty() {
        return Ok(ParsedLine::Empty);
    }

    let (verb, arguments) = split_first_token(line);

    match verb {
        "ADD" => parse_add(arguments).map(ParsedLine::Command),
        "GET" => parse_get(arguments).map(ParsedLine::Command),
        "EXIT" => {
            if arguments.is_some_and(|text| !text.trim().is_empty()) {
                Err("EXIT não recebe argumentos".to_string())
            } else {
                Ok(ParsedLine::Command(Command::Exit))
            }
        }
        _ => Err(format!("comando desconhecido: {verb}")),
    }
}

fn split_first_token(line: &str) -> (&str, Option<&str>) {
    match line.find(char::is_whitespace) {
        Some(index) => {
            let separator_length = line[index..]
                .chars()
                .next()
                .map(char::len_utf8)
                .unwrap_or(1);
            (&line[..index], Some(&line[index + separator_length..]))
        }
        None => (line, None),
    }
}

fn parse_add(arguments: Option<&str>) -> Result<Command, String> {
    let arguments = arguments
        .map(str::trim_start)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| "comando ADD incompleto: informe chave e valor".to_string())?;

    let separator_index = arguments
        .find(char::is_whitespace)
        .ok_or_else(|| "comando ADD incompleto: informe o valor".to_string())?;

    let key = &arguments[..separator_index];
    let separator_length = arguments[separator_index..]
        .chars()
        .next()
        .map(char::len_utf8)
        .unwrap_or(1);
    let value = &arguments[separator_index + separator_length..];

    if key.is_empty() {
        return Err("comando ADD incompleto: informe a chave".to_string());
    }

    if value.is_empty() {
        return Err("comando ADD incompleto: informe o valor".to_string());
    }

    Ok(Command::Add {
        key: key.to_string(),
        value: value.to_string(),
    })
}

fn parse_get(arguments: Option<&str>) -> Result<Command, String> {
    let key = arguments
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| "comando GET incompleto: informe a chave".to_string())?;

    if key.chars().any(char::is_whitespace) {
        return Err("GET recebe somente uma chave".to_string());
    }

    Ok(Command::Get {
        key: key.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::{Command, ParsedLine, parse};

    #[test]
    fn parses_add_and_preserves_spaces_inside_value() {
        assert_eq!(
            parse("ADD greeting hello world"),
            Ok(ParsedLine::Command(Command::Add {
                key: "greeting".to_string(),
                value: "hello world".to_string(),
            }))
        );
    }

    #[test]
    fn parses_get_and_exit() {
        assert_eq!(
            parse("GET greeting"),
            Ok(ParsedLine::Command(Command::Get {
                key: "greeting".to_string(),
            }))
        );
        assert_eq!(parse("EXIT"), Ok(ParsedLine::Command(Command::Exit)));
    }

    #[test]
    fn accepts_empty_line_without_command() {
        assert_eq!(parse("   \n"), Ok(ParsedLine::Empty));
    }

    #[test]
    fn rejects_unknown_lowercase_and_incomplete_commands() {
        assert!(parse("add key value").is_err());
        assert!(parse("ADD key").is_err());
        assert!(parse("GET").is_err());
        assert!(parse("EXIT now").is_err());
    }
}
