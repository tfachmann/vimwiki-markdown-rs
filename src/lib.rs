use chrono::Utc;
use convert_case::{Case, Casing};
use path_clean::PathClean;
use pathdiff::diff_paths;
use pulldown_cmark::{html, Options, Parser};
use regex::{Captures, Regex};
use std::fs;
use std::io::{Error, Write};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};


trait PathSpaces<T> {
    fn handle_spaces(&self) -> T;
}

/// PathSpaces implemented for PathBuf
impl PathSpaces<PathBuf> for PathBuf {
    fn handle_spaces(&self) -> PathBuf {
        PathBuf::from(handle_spaces(self.to_str().unwrap_or("")))
    }
}

/// PathSpaces implemented for String
impl PathSpaces<String> for String {
    fn handle_spaces(&self) -> String {
        handle_spaces(self)
    }
}

fn handle_spaces(path: &str) -> String {
    path.replace(' ', "%20")
}

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

fn fix_link(alt: &str, uri: &str, options: &VimWikiOptions) -> String {
    fn handle_fragment(uri: &str) -> (&str, Option<&str>) {
        let split:Vec<&str> = uri.split('#').collect();
        match split.len() {
            1 => (split[0], None),
            2 => (split[0], Some(split[1])),
            _ => (&uri[..], None),
        }

    }
    fn is_vimwiki_link(input_dir: &Path, uri: &str, ext: &str) -> bool {
        // handle fragment
        let (url_raw, _) = handle_fragment(&uri);
        input_dir.join(Path::new(url_raw)).with_extension(ext).is_file()
    }
    fn handle_title(uri: &str) -> (&str, Option<&str>) {
        // split uri in (url, title)
        let re_title = Regex::new(r#"\s+""#).unwrap();
        let split: Vec<&str> = re_title.split(&uri).collect();
        match split.len() {
            1 => (split[0], None),
            2 => (split[0], Some(split[1])),
            _ => (&uri[..], None),
        }
    }

    let uri: String = uri.to_owned();

    // necessary parameter
    let input_dir = Path::new(&options.input_file).parent().unwrap();
    let output_dir = Path::new(&options.output_dir);

    //let re = Regex::new(r"\[(?P<title>.*)\]\((?P<uri>(?:(?!#).)*)(?P<fragment>(?:#)?.*)\)").unwrap();
    let uri: String = if is_vimwiki_link(input_dir, &uri, &options.extension) {
        let (url_raw, fragment) = handle_fragment(&uri);
        // convert (wiki extension) to .html
        let tmp = Path::new(&url_raw);
        let url_raw = tmp
            .parent()
            .unwrap()
            .join(tmp.file_stem().unwrap())
            .to_str()
            .unwrap()
            .to_owned();
        match fragment {
            Some(fragment) => format!("{}.html#{}", url_raw, fragment.to_string().handle_spaces()),
            None => format!("{}.html", url_raw),
        }
    }
    else {
        // no vimwiki link
        // TODO: assure the file exists
        let (url_raw, title) = handle_title(&uri);
        let url_raw: String = {
            if url_raw.starts_with("file:") {
                // force absolute path
                let tmp: String = url_raw.replace("file:", "");
                let tmp = Path::new(&tmp);
                if tmp.is_absolute() {
                    tmp.to_path_buf()
                } else {
                    input_dir.join(tmp)
                }
            } else if url_raw.starts_with("local:") {
                // force relative path
                let tmp: String = url_raw.replace("local:", "");
                diff_paths(input_dir.join(tmp), output_dir)
                    .unwrap()
            } else {
                PathBuf::from(url_raw)
            }
        }
        .clean()
            .handle_spaces()
            .to_str()
            .unwrap_or(url_raw) // something went wrong, take url
            .to_owned();
        match title {
            Some(title) => format!("{} \"{}", url_raw, title),
            None => url_raw,
        }
    };
    format!("[{}]({})", alt, uri)
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

#[derive(Serialize, Deserialize)]
pub struct ProgramOptions {
    highlight_theme: String,
}

impl ProgramOptions {
    pub fn default() -> ProgramOptions {
        ProgramOptions {
            highlight_theme: "default".to_string(),
        }
    }

    pub fn new(highlight_theme: Option<&str>) -> ProgramOptions {
        ProgramOptions {
            highlight_theme: highlight_theme.unwrap_or("default").to_string(),
        }
    }

    pub fn load(path: &PathBuf) -> ProgramOptions {
        match fs::read_to_string(path) {
            Ok(data_str) => match toml::from_str(&data_str) {
                Ok(data) => data,
                Err(_) => ProgramOptions::default(),
            }
            Err(_) => ProgramOptions::default(),
        }
    }

    pub fn save(&self, path: &PathBuf) {
        let data_str = toml::to_string_pretty(self).unwrap();
        fs::write(path, data_str).expect("Couldn't write to File");
    }
}

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
                Err("The syntax has to be `markdown`".to_owned())
            }
        } else {
            Err("The amount of arguments from VimWiki do not match".to_owned())
        }
    }

    pub fn stem(&self) -> String {
        Path::new(&self.input_file)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned()
    }

    pub fn output_filepath(&self) -> String {
        format!("{}{}.html", self.output_dir, self.stem())
    }

    pub fn get_template_html(&self, highlightjs_theme: &str) -> Result<String, Error> {
        let mut text = fs::read_to_string(&self.template_file).unwrap_or_else(|_| default_template());
        let now = Utc::now();
        text = text
            .replace("%root_path%", &self.root_path)
            .replace("%title%", &self.stem().to_case(Case::Title))
            .replace("%pygments%", "")
            .replace("%code_theme%", highlightjs_theme)
            .replace("%date%", &now.format("%e. %b %Y").to_string());
        Ok(text)
    }

    pub fn get_body_html(&self) -> Result<String, Error> {
        let mut text = fs::read_to_string(&self.input_file)?;
        // TODO: handle Fragment
        let re = Regex::new(r"\[(?P<title>.*)\]\((?P<uri>(.)*)\)").unwrap();
        //let re = Regex::new(r"\[(?P<title>.*)\]\((?P<uri>(.)*)#(?P<fragment>.*)\)").unwrap();
        text = re
            .replace_all(&text, |caps: &Captures| {
                fix_link(&caps["title"], &caps["uri"], self)
            })
        .to_string();
        text = get_html(text);
        Ok(text)
    }
}

pub fn run(wiki_options: VimWikiOptions, program_options: ProgramOptions) -> Result<(), Error> {
    // logging (DEBUG)
    let log_path = "/tmp/vimwiki_markdown_rs";
    let mut file = fs::File::create(log_path)?;

    write!(file, "{:#?}", wiki_options)?;

    // get template_html
    let template_html = wiki_options
        .get_template_html(&program_options.highlight_theme)
        .expect("Couldn't load Template");

    // getting the html body
    let body_html = wiki_options.get_body_html().expect("Couldn't load Body");
    let combined = template_html.replace("%content%", &body_html);

    // saving file
    let mut file = fs::File::create(wiki_options.output_filepath())?;
    write!(file, "{}", combined)?;

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
    fn to_fix_link(link: &str) -> String {
        let options = init_options();
        let re = Regex::new(r"\[(?P<title>.*)\]\((?P<uri>(.)*)\)").unwrap();
        let mut caps_it = re.captures_iter(link);
        let capture = caps_it.next();
        let (alt, uri) = match capture {
            Some(c) => {
                println!("{:?}", c);
                (c["title"].to_string(), c["uri"].to_string())
            },
            None => ("".to_string(), "".to_string()),
        };
        fix_link(&alt, &uri, &options)
    }

    #[test]
    fn fix_link_vimwiki_relative() {
        let link = "[Link Title](another_file)";
        assert_eq!("[Link Title](another_file.html)", to_fix_link(link));
    }

    #[test]
    fn fix_link_vimwiki_relative_up() {
        let link = "[Link Title](../another_file)";
        assert_eq!("[Link Title](../another_file.html)", to_fix_link(link));
    }

    #[test]
    fn fix_link_vimwiki_relative_with_ext() {
        let link = "[Link Title](another_file.wiki)";
        assert_eq!("[Link Title](another_file.html)", to_fix_link(link));
    }

    #[test]
    fn fix_link_vimwiki_relative_fragment() {
        let link = "[Link Title](another_file#fragment)";
        assert_eq!("[Link Title](another_file#fragment.html)", to_fix_link(link));
    }

    #[test]
    fn fix_link_relative() {
        // leave it unchanged as we force to use file: or local:
        let link = "[alt](../foo.png)";
        assert_eq!("[alt](../foo.png)", to_fix_link(link));
    }

    #[test]
    fn fix_link_absolute() {
        let link = "[alt](/abs/path/to/vimwiki/images/foo.png)";
        assert_eq!(
            "[alt](/abs/path/to/vimwiki/images/foo.png)",
            to_fix_link(link)
        );
    }

    #[test]
    fn fix_link_relative_local() {
        let link = "[alt](local:../foo.png)";
        assert_eq!("[alt](../../foo.png)", to_fix_link(link));
    }

    #[test]
    fn fix_link_relative_local_title() {
        let link = "[alt](local:../foo.png \"Title\")";
        assert_eq!("[alt](../../foo.png \"Title\")", to_fix_link(link));
    }

    #[test]
    fn fix_link_force_relative() {
        let link = "[alt](local:/abs/path/to/vimwiki/images/foo.png)";
        assert_eq!("[alt](../../images/foo.png)", to_fix_link(link));
    }

    #[test]
    fn fix_link_force_relative_title() {
        let link = "[alt](local:/abs/path/to/vimwiki/images/foo.png \"Title\")";
        assert_eq!("[alt](../../images/foo.png \"Title\")", to_fix_link(link));
    }

    #[test]
    fn fix_link_force_symlink() {
        unimplemented!();
    }

    #[test]
    fn fix_link_absolute_file() {
        let link = "[alt](file:/abs/path/to/vimwiki/images/foo.png)";
        assert_eq!(
            "[alt](/abs/path/to/vimwiki/images/foo.png)",
            to_fix_link(link)
        );
    }

    #[test]
    fn fix_link_force_absolute() {
        let link = "[alt](file:../images/foo.png)";
        assert_eq!(
            "[alt](/abs/path/to/vimwiki/images/foo.png)",
            to_fix_link(link)
        );
    }

    #[test]
    fn fix_link_spaces() {
        let link = "[alt](file:../images/foo with spaces.png)";
        assert_eq!(
            "[alt](/abs/path/to/vimwiki/images/foo%20with%20spaces.png)",
            to_fix_link(link)
        );
    }

    #[test]
    fn fix_link_spaces_title() {
        let link = "[alt](file:../images/foo with spaces.png \"Title\")";
        assert_eq!(
            "[alt](/abs/path/to/vimwiki/images/foo%20with%20spaces.png \"Title\")",
            to_fix_link(link)
        );
    }

    #[test]
    fn relative_paths() {
        let p1 = Path::new("/abs/path/to/Document/foo.xyz");
        let p2 = Path::new("/abs/path/to/whatever");
        assert_eq!(
            Path::new("../Document/foo.xyz"),
            pathdiff::diff_paths(&p1, &p2).unwrap()
        );
    }
}
