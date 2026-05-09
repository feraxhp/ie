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
        .args([
            arg!( <file> "The name of the file to be opened" )
                .value_hint(ValueHint::FilePath)
            ,
            arg!( --full "When to open the editor as a full TUI" )
            ,
            arg!( -l --lines [number] "The the number of lines to show (inline mode only)" )
                .conflicts_with("full")
                .default_value("8")
                .value_parser(value_parser!(u16))
            ,
            arg!( -f --format [format] "Overrides the detected language for syntax parsing" )
                .value_parser([ 
                    "rust",
                    "javascript",
                    "typescript",
                    "python",
                    "go",
                    "java",
                    "c_sharp",
                    "c",
                    "cpp",
                    "html",
                    "css",
                    "yaml",
                    "json",
                    "toml",
                    "shell",
                    "markdown",
                    "markdown-inline",
                ])
            ,
        ]);

    let matches = command.get_matches();

    let is_full = matches.get_flag("full");
    let lines = matches.get_one::<u16>("lines").unwrap();
    let file = matches.get_one::<String>("file").unwrap();
    let format = matches.get_one::<String>("format").map(|f| f.clone());

    let mode = match is_full {
        true => TUImode::FULL,
        false => TUImode::INLINE(lines.clone()),
    };
    
    return Options { 
        mode,
        file: file.clone(),
        format,
    }
}