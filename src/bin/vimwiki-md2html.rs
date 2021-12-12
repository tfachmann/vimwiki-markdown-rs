use anyhow::Result;
use env_logger::Env;
use log::info;
use std::path::PathBuf;
use structopt::StructOpt;

use vimwiki_markdown_rs::VimWikiOptions;

#[derive(StructOpt, Debug)]
#[structopt(name = "vimwiki-md2html")]
struct Opt {
    #[structopt(short, long, parse(from_occurrences))]
    verbose: u8,

    #[structopt(short = "e", long = "ext", default_value = "wiki")]
    extension: String,

    #[structopt(short = "t", long = "template", default_value = "default")]
    template_file: PathBuf,

    #[structopt(long = "root", default_value = "./")]
    root_path: PathBuf,

    #[structopt(short = "o", long = "output")]
    output_dir: PathBuf,

    #[structopt(name = "FILE")]
    input_file: PathBuf,
}

impl From<Opt> for VimWikiOptions {
    fn from(opt: Opt) -> Self {
        VimWikiOptions::new(
            &opt.extension,
            &opt.template_file,
            &opt.root_path,
            &opt.output_dir,
            &opt.input_file,
        )
    }
}

fn main() -> Result<()> {
    env_logger::from_env(Env::default().default_filter_or("INFO")).init();

    info!("Parsing commandline arguments");
    let opt = Opt::from_args();
    info!("{:#?}", opt);

    // get user specific configurations
    info!("Loading configuration file...");
    let program_options = vimwiki_markdown_rs::ProgramOptions::new();

    // run function
    info!("Generating html file...");
    vimwiki_markdown_rs::to_html_and_save(&opt.into(), &program_options)?;
    Ok(())
}
