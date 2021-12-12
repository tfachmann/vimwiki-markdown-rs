use std::env;
use std::io;

fn main() -> Result<(), io::Error> {
    // collect the command-line arguments
    let args: Vec<String> = env::args().collect();
    let wiki_options =
        vimwiki_markdown_rs::VimWikiOptions::new(&args).expect("Couldn't load VimWikiOptions");

    // get user specific configurations
    let program_options = vimwiki_markdown_rs::ProgramOptions::new();

    // run method, send Error back to user (vimwiki plugin)
    vimwiki_markdown_rs::to_html_and_save(&wiki_options, &program_options)?;

    Ok(())
}
