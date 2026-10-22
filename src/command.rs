use clap::{Command, ValueHint, arg, value_parser};

pub enum TUImode {
    INLINE(u16),
    FULL,
}

pub struct Options {
    pub mode: TUImode,
    pub file: String,
    pub format: Option<String>,
}

pub fn get_options() -> Options {
    let command = Command::new("ie")
        .about("A fast inline and fullscreen TUI editor")
        .args([
            arg!( <file> "The path of the file to be opened" )
                .value_hint(ValueHint::FilePath)
            ,
            arg!( --full "Open the editor in fullscreen TUI mode" )
            ,
            arg!( -l --lines [number] "The number of lines to display (inline mode only)" )
                .conflicts_with("full")
                .default_value("8")
                .value_parser(value_parser!(u16))
            ,
            arg!( -f --format [format] "Overrides the detected language for syntax highlighting" )
            ,
        ]);

    let matches = command.get_matches();

    let is_full = matches.get_flag("full");
    let lines = matches.get_one::<u16>("lines").unwrap();
    let file = matches.get_one::<String>("file").unwrap();
    let format = matches.get_one::<String>("format").cloned();

    let mode = match is_full {
        true => TUImode::FULL,
        false => TUImode::INLINE(*lines),
    };

    Options {
        mode,
        file: file.clone(),
        format,
    }
}
