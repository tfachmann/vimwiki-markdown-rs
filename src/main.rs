use anyhow::Result;
use env_logger::Env;
use log::info;
use std::env;

use vimwiki_markdown_rs::VimWikiOptions;

struct VimWikiCmdlineArgs {
    force: bool,
    syntax: String,
    extension: String,
    output_dir: String,
    input_file: String,
    css_file: String,
    template_file: String,
    root_path: String,
}

impl VimWikiCmdlineArgs {
    /// Creates a new `VimWikiCmdLineArgs` by parsing the `args` arguments vector.
    /// These are given by the convention of VimWiki.
    ///
    /// # Errors
    ///
    /// Will return `Err` if the length of `args` is wrong (not 12) or the syntax specified in
    /// `args[2]` is not `"markdown"`. The arguments are provided by VimWiki's plugin.
    ///
    /// # Usage
    ///
    ///
    ///```ignore
    ///let args = vec![
    ///    "vimwiki-markdown-rs",                   // program name
    ///    "1",                                     // force flag
    ///    "markdown",                              // syntax
    ///    "wiki",                                  // (wiki) extension
    ///    "/abs/path/to/vimwiki/site_html/bar/",   // directory of (html) output
    ///    "/abs/path/to/vimwiki/bar/mdfile.wiki",  // path of input / vimwiki file
    ///    "css-file.css",                          // path of css file
    ///    "/abs/path/to/vimwiki/templates/",       // directory of template
    ///    "template",                              // template filename
    ///    ".tpl",                                  // template extension
    ///    "../",                                   // relative path to root
    ///    "-",                                     // not clear / irrelevant
    ///];
    ///let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    ///
    ///VimWikiOptions::new(&args).unwrap();
    ///```
    fn new(args: &[String]) -> Result<VimWikiCmdlineArgs, String> {
        if args.len() == 12 {
            let template_file =
                [args[7].to_owned(), args[8].to_owned(), args[9].to_owned()].concat();
            if args[2] == "markdown" {
                let options = VimWikiCmdlineArgs {
                    force: args[1] == "1",
                    syntax: args[2].to_owned(),
                    extension: args[3].to_owned(),
                    output_dir: args[4].to_owned(),
                    input_file: args[5].to_owned(),
                    css_file: args[6].to_owned(),
                    template_file,
                    root_path: {
                        if args[10] == "-" && args[11] == "-" {
                            String::from("./")
                        } else {
                            args[10].to_owned()
                        }
                    },
                };
                Ok(options)
            } else {
                Err("The syntax has to be markdown".to_owned())
            }
        } else {
            Err(format!("The amount of arguments from VimWiki do not match. You provided {}, but {} are necessary", args.len(), 12))
        }
    }
}

impl From<VimWikiCmdlineArgs> for VimWikiOptions {
    fn from(cmdline_args: VimWikiCmdlineArgs) -> Self {
        VimWikiOptions::new(
            &cmdline_args.extension,
            &cmdline_args.output_dir,
            &cmdline_args.input_file,
            &cmdline_args.template_file,
            &cmdline_args.root_path,
        )
    }
}

fn main() -> Result<()> {
    env_logger::from_env(Env::default().default_filter_or("INFO")).init();

    // collect the command-line arguments
    info!("Parsing commandline arguments...");
    let args: Vec<String> = env::args().collect();
    let wiki_cmdline_args = VimWikiCmdlineArgs::new(&args).expect("Couldn't load VimWikiOptions");

    // get user specific configurations
    info!("Loading configuration file...");
    let program_options = vimwiki_markdown_rs::ProgramOptions::new();

    // run method, send Error back to user (vimwiki plugin)
    info!("Generating html file...");
    vimwiki_markdown_rs::to_html_and_save(&wiki_cmdline_args.into(), &program_options)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn init_options() -> VimWikiCmdlineArgs {
        let args = vec![
            "vimwiki-markdown-rs",
            "1",
            "markdown",
            "wiki",
            "/abs/path/to/vimwiki/site_html/bar/",
            "/abs/path/to/vimwiki/bar/mdfile.wiki",
            "css-file.css",
            "/abs/path/to/vimwiki/templates/",
            "template",
            ".tpl",
            "../",
            "-",
        ];
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        VimWikiCmdlineArgs::new(&args).unwrap()
    }

    #[test]
    fn options_correct() {
        init_options();
    }

    #[test]
    #[should_panic(expected = "arguments from VimWiki do not match")]
    fn options_wrong_length() {
        let args = vec![""; 11];
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();

        VimWikiCmdlineArgs::new(&args).unwrap();
    }

    #[test]
    #[should_panic(expected = "syntax has to be markdown")]
    fn options_not_markdown() {
        let args = vec![
            "vimwiki-markdown-rs",
            "1",
            "vimwiki", // has to be markdown
            "wiki",
            "/abs/path/to/vimwiki/site_html/bar/",
            "/abs/path/to/vimwiki/bar/mdfile.wiki",
            "css-file.css",
            "/abs/path/to/vimwiki/templates/",
            "template",
            ".tpl",
            "../",
            "-",
        ];
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();

        VimWikiCmdlineArgs::new(&args).unwrap();
    }
}
