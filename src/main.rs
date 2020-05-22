use std::io;
use std::env;
use directories::ProjectDirs;
use std::path::Path;
use std::fs;

fn main() -> Result<(), io::Error> {

    // collect the command-line arguments
    let args: Vec<String> = env::args().collect();
    let wiki_options = vimwiki_markdown_rs::VimWikiOptions::new(&args).expect("Couldn't load VimWikiOptions");

    // get user specific configurations
    let program_options = if let Some(proj_dirs) = ProjectDirs::from("com", "tfachmann", "vimwiki-markdown-rs") {
        let conf_path = Path::new(proj_dirs.config_dir());
        if !conf_path.is_dir() {
            fs::create_dir(conf_path).unwrap_or(());
        }
        let conf_file = conf_path.join("config.toml");
        if conf_file.is_file() {
            vimwiki_markdown_rs::ProgramOptions::load(&conf_file)
        }
        else {
            // the settings file doesn't exist, save defaults
            let po = vimwiki_markdown_rs::ProgramOptions::default();
            po.save(&conf_file);
            po
        }
    }
    else {
        vimwiki_markdown_rs::ProgramOptions::default()
    };

    // run method, send Error back to user (vimwiki plugin)
    vimwiki_markdown_rs::to_html_and_save(&wiki_options, &program_options)?;

    Ok(())
}
