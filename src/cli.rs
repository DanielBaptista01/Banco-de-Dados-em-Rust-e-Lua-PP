use std::io::{self, BufRead, Write};

use crate::{
    command::{ParsedLine, parse},
    engine::{Engine, EngineResponse},
};

pub fn run(engine: Engine) -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_with_io(engine, stdin.lock(), stdout.lock())
}

pub fn run_with_io<R: BufRead, W: Write>(
    engine: Engine,
    mut input: R,
    mut output: W,
) -> io::Result<()> {
    let mut line = String::new();

    loop {
        write!(output, "> ")?;
        output.flush()?;

        line.clear();
        if input.read_line(&mut line)? == 0 {
            break;
        }

        match parse(&line) {
            Ok(ParsedLine::Empty) => continue,
            Ok(ParsedLine::Command(command)) => match engine.execute(command) {
                EngineResponse::Continue(response) => writeln!(output, "{response}")?,
                EngineResponse::Exit => break,
            },
            Err(reason) => writeln!(output, "ERRO: {reason}")?,
        }
    }

    Ok(())
}
