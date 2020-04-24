use pulldown_cmark::{Parser, Options, html};
use std::env;
use std::fs;
use std::path::Path;
use std::io::{Write, Error};
use convert_case::{Case, Casing};
use chrono::Utc;

fn get_html(markdown: String) -> String {
    let mut html_out = String::with_capacity(markdown.len());
    let parser = Parser::new_ext(
        &markdown,
        Options::ENABLE_FOOTNOTES |
        Options::ENABLE_TABLES |
        Options::ENABLE_STRIKETHROUGH |
        Options::ENABLE_TASKLISTS);
    html::push_html(&mut html_out, parser);
    html_out
}

#[derive(Debug)]
struct VimWikiOptions {
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
    pub fn new(args: &Vec<String>) -> Result<VimWikiOptions, String> {
        if args.len() != 12 {
            Err("The amount of arguments from VimWiki do not match".to_string())
        } else {
            let template_file = [args[7].to_owned(), args[8].to_owned(), args[9].to_owned()].concat();
            if args[2] != "markdown" {
                Err("The syntax has to be `markdown`".to_string())
            }
            else if !fs::metadata(&template_file).is_ok() {
                Err(format!("Template `{}` does not exist or can not be loaded", template_file))
            }
            else {
            let options = VimWikiOptions {
                force: {
                    if args[1] == "1" {
                        true
                    } else {
                        false
                    }
                },
                syntax: args[2].to_owned(),
                extension: args[3].to_owned(),
                output_dir: args[4].to_owned(),
                input_file: args[5].to_owned(),
                css_file: args[6].to_owned(),
                template_file,
                root_path: {
                    if args[10] == "-" && args[11] == "-" {
                        String::from("./")
                    }
                    else {
                        args[10].to_owned()
                    }
                },
            };
            Ok(options)
            }
        }
    }

    pub fn stem(&self) -> String {
        Path::new(&self.input_file).file_stem().unwrap().to_str().unwrap().to_owned()
    }

    pub fn output_filepath(&self) -> String {
        format!("{}{}.html", self.output_dir, self.stem())
    }

    pub fn get_template_html(&self) -> Result<String, Error> {
        let mut text = fs::read_to_string(&self.template_file)?;
        let now = Utc::now();
        text = text
            .replace("%root_path%", &self.root_path)
            .replace("%title%", &self.stem().to_case(Case::Title))
            .replace("%pygments%", "")
            .replace("%code_theme%", "dark")
            .replace("%date%", &now.format("%e. %b %Y").to_string());
        Ok(text)
    }

    pub fn get_body_html(&self) -> Result<String, Error> {
        let mut text = fs::read_to_string(&self.input_file)?;
        text = get_html(text);
        Ok(text)
    }
}

fn main() -> Result<(), Error> {
    // logging (DEBUG)
    let log_path = "/tmp/vimwiki_markdown_rs";
    let mut file = fs::File::create(log_path)?;

    // collect the command-line arguments
    let args: Vec<String> = env::args().collect();

    // parse arguments to get options struct
    let options = VimWikiOptions::new(&args).expect("Couldn't load VimWikiOptions");
    write!(file, "{:#?}", args.to_owned())?;
    write!(file, "{:#?}", options)?;

    // get template_html
    let template_html = options.get_template_html().expect("Couldn't load Template");
    let mut file2 = fs::File::create("/tmp/vimwiki_markdown_rs.html")?;
    write!(file2, "{}", template_html)?;

    // getting the html body
    let body_html = options.get_body_html().expect("Couldn't load Body");
    let combined = template_html.replace("%content%", &body_html);

    // saving file
    let mut file = fs::File::create(options.output_filepath())?;
    write!(file, "{}", combined)?;

    Ok(())
}
