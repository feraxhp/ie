use clap::{Command, ValueHint, arg, value_parser};

pub enum TUImode {
    INLINE(u16),
    FULL,
}

pub struct Options {
    pub mode: TUImode,
    pub file: String,
}


pub fn get_options() -> Options {
    let command = Command::new("ie")
        .args([
            arg!( <file> "The name of the file to be opened" )
                .value_hint(ValueHint::FilePath)
            ,
            arg!( -l --lines [number] "The the number of lines to show (inline mode only)" )
                .conflicts_with("full")
                .default_value("8")
                .value_parser(value_parser!(u16))
            ,
            arg!( --full "When to open the editor as a full TUI" )
            ,
        ]);

    let matches = command.get_matches();

    let is_full = matches.get_flag("full");
    let lines = matches.get_one::<u16>("lines").unwrap();
    let file = matches.get_one::<String>("file").unwrap();

    let mode = match is_full {
        true => TUImode::FULL,
        false => TUImode::INLINE(lines.clone()),
    };
    
    return Options { 
        mode,
        file: file.clone(),
    }
}