use path_clean::PathClean;
use pathdiff::diff_paths;
use regex::Regex;
use std::path::{Path, PathBuf};

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

fn handle_fragment(uri: &str) -> (&str, Option<&str>) {
    let split: Vec<&str> = uri.split('#').collect();
    match split.len() {
        1 => (split[0], None),
        2 => (split[0], Some(split[1])),
        _ => (&uri[..], None),
    }
}

fn fix_link_vimwiki(uri: &str) -> String {
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

fn fix_link_rest(uri: &str, input_dir: &Path, output_dir: &Path) -> String {
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
            diff_paths(input_dir.join(tmp), output_dir).unwrap()
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
}

/// Handles an input link split in `alt` and `uri` and returns a correct markdown link.
///
/// This will handle relative and absolut paths to the new output_dir and corrects vimwiki
/// references to point to html files
pub fn fix_link(
    alt: &str,
    uri: &str,
    input_file: &str,
    output_dir: &str,
    extension: &str,
) -> String {
    fn is_vimwiki_link(input_dir: &Path, uri: &str, ext: &str) -> bool {
        // handle fragment
        let (url_raw, _) = handle_fragment(&uri);
        input_dir
            .join(Path::new(url_raw))
            .with_extension(ext)
            .is_file()
    }
    let uri: String = uri.to_owned();

    // necessary parameter
    let input_dir = Path::new(input_file).parent().unwrap();
    let output_dir = Path::new(output_dir);

    let uri: String = if is_vimwiki_link(input_dir, &uri, extension) {
        fix_link_vimwiki(&uri)
    } else {
        fix_link_rest(&uri, input_dir, output_dir)
    };
    format!("[{}]({})", alt, uri)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn to_fix_link(link: &str) -> String {
        let input_file = "/abs/path/to/vimwiki/bar/mdfile.wiki";
        let output_dir = "/abs/path/to/vimwiki/site_html/bar/";
        let extension = "wiki";
        let re = Regex::new(r"\[(?P<title>.*)\]\((?P<uri>(.)*)\)").unwrap();
        let mut caps_it = re.captures_iter(link);
        let capture = caps_it.next();
        let (alt, uri) = match capture {
            Some(c) => (c["title"].to_string(), c["uri"].to_string()),
            None => ("".to_string(), "".to_string()),
        };
        fix_link(&alt, &uri, input_file, output_dir, extension)
    }
    fn to_fix_link_vimwiki(link: &str) -> String {
        let re = Regex::new(r"\[(?P<title>.*)\]\((?P<uri>(.)*)\)").unwrap();
        let mut caps_it = re.captures_iter(link);
        let capture = caps_it.next();
        let (alt, uri) = match capture {
            Some(c) => (c["title"].to_string(), c["uri"].to_string()),
            None => ("".to_string(), "".to_string()),
        };
        let uri = fix_link_vimwiki(&uri);
        format!("[{}]({})", alt, uri)
    }

    #[test]
    fn fix_link_vimwiki_relative() {
        let link = "[Link Title](another_file)";
        assert_eq!("[Link Title](another_file.html)", to_fix_link_vimwiki(link));
    }

    #[test]
    fn fix_link_vimwiki_relative_up() {
        let link = "[Link Title](../another_file)";
        assert_eq!(
            "[Link Title](../another_file.html)",
            to_fix_link_vimwiki(link)
        );
    }

    #[test]
    fn fix_link_vimwiki_relative_with_ext() {
        let link = "[Link Title](another_file.wiki)";
        assert_eq!("[Link Title](another_file.html)", to_fix_link_vimwiki(link));
    }

    #[test]
    fn fix_link_vimwiki_relative_fragment() {
        let link = "[Link Title](another_file#fragment)";
        assert_eq!(
            "[Link Title](another_file.html#fragment)",
            to_fix_link_vimwiki(link)
        );
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

    //#[test]
    //fn fix_link_force_symlink() {
    //unimplemented!();
    //}

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
