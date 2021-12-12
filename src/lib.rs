//! `vimwiki-markdown-rs` is a library to parse vimwiki-markdown files to html.
//!
//! The binary that comes with this crate should be embedded with the VimWiki-Plugin for a seamless
//! integration.

use anyhow::Result;
use chrono::Utc;
use convert_case::{Case, Casing};
use directories::ProjectDirs;
use kuchiki::traits::*;
use log::warn;
use pulldown_cmark::{html, Options, Parser};
use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Error, ErrorKind, Write};
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
#[derive(Debug, Serialize, Deserialize)]
pub struct ProgramOptions {
    highlight_theme: String,
}

impl Default for ProgramOptions {
    /// Creates a new `ProgramOptions` with default settings.
    fn default() -> Self {
        Self {
            highlight_theme: "default".to_string(),
        }
    }
}

impl ProgramOptions {
    /// Creates a new `ProgramOptions` from the toml configuration file.
    ///
    /// If the configuration file given by `path` does not exist or is invalid,
    /// `ProgramOptions` with `default` Parameters will be returned.
    pub fn new() -> ProgramOptions {
        if let Some(proj_dirs) = ProjectDirs::from("com", "tfachmann", "vimwiki-markdown-rs") {
            let conf_path = Path::new(proj_dirs.config_dir());
            if !conf_path.is_dir() {
                fs::create_dir(conf_path).unwrap_or(());
            }
            let conf_file = conf_path.join("config.toml");
            match ProgramOptions::load(&conf_file) {
                Ok(po) => po,
                Err(err) => {
                    warn!(
                        "Could not load config in {}: {}\nUsing default.",
                        &conf_file.to_str().unwrap(),
                        &err
                    );
                    let po = ProgramOptions::default();
                    if let Err(err) = po.save(&conf_file) {
                        warn!(
                            "Could not save default config in {}: {}",
                            &conf_file.to_str().unwrap(),
                            &err
                        );
                    }
                    po
                }
            }
        } else {
            ProgramOptions::default()
        }
    }

    /// Creates a new `ProgramOptions` from the toml configuration file.
    ///
    /// If the configuration file given by `path` does not exist or is invalid,
    /// `ProgramOptions` with `default` Parameters will be returned.
    fn load(path: &PathBuf) -> Result<ProgramOptions> {
        let data_str = fs::read_to_string(path)?;
        let data: ProgramOptions = toml::from_str(&data_str)?;
        Ok(data)
    }

    /// Save the `ProgramOptions` to a toml configuration file given with `path`.
    fn save(&self, path: &PathBuf) -> Result<()> {
        let data_str = toml::to_string_pretty(self)?;
        fs::write(path, data_str)?;
        Ok(())
    }
}

/// All options / arguments related to `VimWiki`.
///
/// Not all options are used yet. However, `VimWiki` provides them and they might be used in
/// upcoming versions.
#[derive(Debug)]
pub struct VimWikiOptions {
    extension: String,
    output_dir: String,
    input_file: String,
    template_file: String,
    root_path: String,
}

impl VimWikiOptions {
    pub fn new(
        extension: &str,
        output_dir: &str,
        input_file: &str,
        template_file: &str,
        root_path: &str,
    ) -> Self {
        Self {
            extension: extension.to_string(),
            output_dir: output_dir.to_string(),
            input_file: input_file.to_string(),
            template_file: template_file.to_string(),
            root_path: root_path.to_string(),
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
        // read file to string
        let text = fs::read_to_string(&self.input_file)?;

        let mut var_store = HashMap::new();
        // parse defined commands
        let re_def = Regex::new(r"<'''(?P<data>(.|\n)*)'''>").unwrap();
        let mut caps_it = re_def.captures_iter(&text);
        let capture = caps_it.next();
        match capture {
            Some(c) => {
                let re_def_single = Regex::new(r"(?P<key>\S*?)\{(?P<value>[^}]*?)\}").unwrap();
                re_def_single
                    .captures_iter(&c["data"])
                    .into_iter()
                    .for_each(|e| {
                        var_store.insert(e["key"].to_owned(), e["value"].to_owned());
                    });
            }
            None => (),
        }
        // delete
        let text = re_def.replace_all(&text, "").to_string();

        // check whether variables were used and replace them
        // TODO: do this recursively, until all occurences are fixed
        let re_var =
            Regex::new(r"'\{(?P<before>.*?)\$(?P<var>\S+?)(?P<after>(\s.*?\}|\}))'").unwrap();
        let text = re_var
            .replace_all(&text, |caps: &Captures| {
                let val = match var_store.get(&caps["var"]) {
                    Some(value) => value,
                    None => panic!("Cannot find variable `{}`", &caps["var"]),
                };
                // due to the nature of the regex, the last } will always be included at the end
                let before = &caps["before"];
                let after = &caps["after"][0..&caps["after"].len() - 1];
                format!("'{{{}{}{}}}'", before, val, after)
            })
            .to_string();

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
        let html = get_html(text);
        let document = kuchiki::parse_html().one(html.clone());

        let re_cmd = Regex::new(r"'\{(?P<element>\S+)\s+(?P<type>\S+)\s+(?P<data>.*?)\}'").unwrap();

        let mut change_parents = vec![];
        document.descendants().for_each(|node| {
            if let Some(text) = node.as_text() {
                if let Some(capture) = re_cmd.captures_iter(&text.borrow()).next() {
                    let element_type = &capture["element"];
                    let html_attribute = match &capture["type"] {
                        "s" | "st" | "sty" | "styl" | "style" => "style",
                        _ => panic!("HTML attribute `{}` unknown", &capture["type"]),
                    };
                    let data = &capture["data"];
                    match element_type {
                        "p" | "pa" | "par" | "pare" | "paren" | "parent" => {
                            if let Some(parent) = node.parent() {
                                if let Some(element_data) = parent.as_element() {
                                    let mut att = element_data.attributes.borrow_mut();
                                    att.insert(html_attribute, data.to_string());
                                }
                                change_parents.push((parent, data.to_owned()));
                            }
                        }
                        _ => panic!("Element type `{}` unknown", element_type),
                    };
                }
            };
        });
        Ok(re_cmd.replace_all(&document.to_string(), "").to_string())
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
) -> Result<()> {
    // get html
    let html = to_html(wiki_options, program_options).map_err(|e| {
        Error::new(
            ErrorKind::InvalidInput,
            format!(
                "Could not create html. The passed options might be compromised: {}",
                e
            ),
        )
    })?;

    // save file
    let mut file = fs::File::create(wiki_options.output_filepath())?;
    write!(file, "{}", html)?;

    Ok(())
}

