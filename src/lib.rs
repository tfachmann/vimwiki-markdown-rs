//! `vimwiki-markdown-rs` is a library to parse vimwiki-markdown files to html.
//!
//! The binary that comes with this crate should be embedded with the VimWiki-Plugin for a seamless
//! integration.

use chrono::Utc;
use convert_case::{Case, Casing};
use pulldown_cmark::{html, Options, Parser};
use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Error, Write};
use std::path::{Path, PathBuf};

mod links;

fn get_html(markdown: String) -> String {
    let mut html_out = String::with_capacity(markdown.len());
    let parser = Parser::new_ext(
        &markdown,
        Options::ENABLE_FOOTNOTES
            | Options::ENABLE_TABLES
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_TASKLISTS,
    );
    html::push_html(&mut html_out, parser);
    html_out
}

fn default_template() -> String {
    "<html>
<head>
    <link rel=\"Stylesheet\" type=\"text/css\" href=\"%root_path%style.css\" />
    <title>%title%</title>
    <meta http-equiv=\"Content-Type\" content=\"text/html; charset=utf-8\" />

    %pygments%
</head>
<body>
    <a href=\"%root_path%index.html\">Index</a> |
    <hr>
    <div class=\"content\">
    %content%
    </div>
</body>
</html>"
        .to_owned()
}

/// All options related to the program such as the `highlighting_theme`.
///
/// It offers options to save and load a `toml` configuration file.
#[derive(Serialize, Deserialize)]
pub struct ProgramOptions {
    highlight_theme: String,
}

impl ProgramOptions {
    /// Creates a new `ProgramOptions` with default settings.
    pub fn default() -> ProgramOptions {
        ProgramOptions {
            highlight_theme: "default".to_string(),
        }
    }

    /// Creates a new `ProgramOptions` from the toml configuration file.
    ///
    /// If the configuration file given by `path` does not exist or is invalid,
    /// `ProgramOptions` with `default` Parameters will be returned.
    pub fn load(path: &PathBuf) -> ProgramOptions {
        match fs::read_to_string(path) {
            Ok(data_str) => match toml::from_str(&data_str) {
                Ok(data) => data,
                Err(_) => ProgramOptions::default(),
            },
            Err(_) => ProgramOptions::default(),
        }
    }

    /// Save the `ProgramOptions` to a toml configuration file given with `path`.
    pub fn save(&self, path: &PathBuf) {
        let data_str = toml::to_string_pretty(self).unwrap();
        fs::write(path, data_str).expect("Couldn't write to File");
    }
}

/// All options / arguments related to `VimWiki`.
///
/// Not all options are used yet. However, `VimWiki` provides them and they might be used in
/// upcoming versions.
#[derive(Debug)]
pub struct VimWikiOptions {
    force: bool,
    syntax: String,
    extension: String,
    output_dir: String,
    input_file: String,
    css_file: String,
    template_file: String,
    root_path: String,
}

impl VimWikiOptions {
    /// Creates a new `VimWikiOptions` by parsing the `args` arguments vector.
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
    pub fn new(args: &[String]) -> Result<VimWikiOptions, String> {
        if args.len() == 12 {
            let template_file =
                [args[7].to_owned(), args[8].to_owned(), args[9].to_owned()].concat();
            if args[2] == "markdown" {
                let options = VimWikiOptions {
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
            Err("The amount of arguments from VimWiki do not match".to_owned())
        }
    }

    fn stem(&self) -> String {
        Path::new(&self.input_file)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned()
    }

    /// Returns the path of the html output as `String`
    pub fn output_filepath(&self) -> String {
        format!("{}{}.html", self.output_dir, self.stem())
    }

    fn get_template_html(&self, highlightjs_theme: &str) -> String {
        let text = fs::read_to_string(&self.template_file).unwrap_or_else(|_| default_template());
        let now = Utc::now();
        text.replace("%root_path%", &self.root_path)
            .replace("%title%", &self.stem().to_case(Case::Title))
            .replace("%pygments%", "")
            .replace("%code_theme%", highlightjs_theme)
            .replace("%date%", &now.format("%e. %b %Y").to_string())
    }

    fn get_body_html(&self) -> Result<String, Error> {
        let text = fs::read_to_string(&self.input_file)?;
        let re = Regex::new(r"\[(?P<title>.*)\]\((?P<uri>(.)*)\)").unwrap();

        // fix each found link
        let text = re
            .replace_all(&text, |caps: &Captures| {
                links::fix_link(
                    &caps["title"],
                    &caps["uri"],
                    &self.input_file,
                    &self.output_dir,
                    &self.extension,
                )
            })
            .to_string();
        Ok(get_html(text))
    }
}

/// Uses `VimWikiOptions` and `ProgramOptions` to load the template and body html. Returns the html String.
pub fn to_html(
    wiki_options: &VimWikiOptions,
    program_options: &ProgramOptions,
) -> Result<String, Error> {
    // get template_html
    let template_html = wiki_options.get_template_html(&program_options.highlight_theme);

    // get the html body
    let body_html = wiki_options.get_body_html().expect("Couldn't load Body");
    let combined = template_html.replace("%content%", &body_html);

    // return combined html
    Ok(combined)
}

/// Uses `VimWikiOptions` and `ProgramOptions` to load the template and body html. Also saves the html
/// file according the `wiki_options.output_filepath()`
pub fn to_html_and_save(
    wiki_options: &VimWikiOptions,
    program_options: &ProgramOptions,
) -> Result<(), Error> {
    // get html
    let html = to_html(wiki_options, program_options)
        .expect("Couldn't create html. The passed options might be compromised");

    // save file
    let mut file = fs::File::create(wiki_options.output_filepath())?;
    write!(file, "{}", html)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_options() -> VimWikiOptions {
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
        VimWikiOptions::new(&args).unwrap()
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

        VimWikiOptions::new(&args).unwrap();
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

        VimWikiOptions::new(&args).unwrap();
    }
}
