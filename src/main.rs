use clap::Parser;
use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
use url::Url;

#[derive(Parser, Debug)]
#[clap(
    author = "YuheiNakasaka",
    version = "0.0.1",
    about = "sb2md is a CLI for converting Scrapbox to Markdown",
    long_about = None
)]
struct Cli {
    path: String,
}

#[derive(Deserialize)]
struct ScrapboxPage {
    lines: Vec<ScrapboxLine>,
}

#[derive(Deserialize)]
struct ScrapboxLine {
    text: String,
}

struct SbRequest {
    url: String,
}

impl SbRequest {
    fn new(path: String) -> Self {
        let paths: Vec<String> = path.split("/").map(|p| p.to_string()).collect();
        let url = format!("https://scrapbox.io/api/pages/{}/{}", paths[0], paths[1]);
        let parsed_url = Url::parse(&url[..]).expect("url is invalid");
        Self {
            url: parsed_url.to_string(),
        }
    }

    fn fetch(&self) -> Result<ScrapboxPage, reqwest::Error> {
        let resp: ScrapboxPage = reqwest::blocking::get(&self.url)?.json()?;
        Result::Ok(resp)
    }
}

lazy_static! {
    static ref RGX_CODE_BLOCK: Regex = Regex::new(r"^code:.+").unwrap();
    static ref RGX_CODE_BLOCK_WITH_EXT: Regex = Regex::new(r"^code:[^.]*\.([^.]*)$").unwrap();
    static ref RGX_TABLE: Regex = Regex::new(r"^table:(.*)$").unwrap();
    static ref RGX_SPACED_LINE: Regex = Regex::new(r"^[\s|\t]+").unwrap();
    static ref RGX_HEADING: Regex = Regex::new(r"^\[(\*+)\s([^\]]+)\]$").unwrap();
}

enum TokenType {
    CodeBlock,
    Table,
    Other,
}

struct ToMd {
    page: ScrapboxPage,
    token_type: TokenType,
    output: String,
}

impl ToMd {
    fn new(page: ScrapboxPage) -> Self {
        Self {
            page,
            token_type: TokenType::Other,
            output: String::new(),
        }
    }

    fn convert(&mut self) -> String {
        for line in &self.page.lines {
            match self.token_type {
                TokenType::CodeBlock => {
                    if !RGX_SPACED_LINE.is_match(&line.text[..]) {
                        self.output.push_str("```\n\n");
                        self.token_type = TokenType::Other;
                    } else {
                        self.output.push_str(&format!("{}\n", line.text));
                    }
                }
                TokenType::Table => {
                    if !RGX_SPACED_LINE.is_match(&line.text[..]) {
                        self.token_type = TokenType::Other;
                        self.output.push_str("\n");
                    } else {
                        let texts = line.text.trim().split("\t").collect::<Vec<&str>>();
                        let texts = texts.join(" | ");
                        let texts = format!("{}{}{}\n", "| ", texts, " |");
                        self.output.push_str(&texts);
                    }
                }
                TokenType::Other => {
                    if RGX_CODE_BLOCK.is_match(&line.text[..]) {
                        let captures = RGX_CODE_BLOCK_WITH_EXT.captures(&line.text);
                        if captures.is_some() {
                            let ext = captures.unwrap().get(1).unwrap().as_str();
                            self.output.push_str(&format!("```{}\n", ext));
                        } else {
                            self.output.push_str("```\n");
                        }
                        self.token_type = TokenType::CodeBlock;
                    } else if RGX_TABLE.is_match(&line.text[..]) {
                        self.token_type = TokenType::Table;
                    } else if RGX_HEADING.is_match(&line.text[..]) {
                        let captures = RGX_HEADING.captures(&line.text).unwrap();
                        let heading_level = &captures[1];
                        let heading_level = if heading_level.len() >= 4 {
                            1
                        } else {
                            5 - heading_level.len()
                        };
                        let heading_level = "#".repeat(heading_level);
                        let heading_text = &captures[2];
                        self.output
                            .push_str(&format!("{} {}\n", heading_level, heading_text));
                    } else {
                        self.output.push_str(&format!("{}\n", line.text));
                    }
                }
            }
        }
        self.output.to_owned()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    let sbrequest = SbRequest::new(args.path);
    let resp = sbrequest.fetch().expect("failed to fetch");
    let md = ToMd::new(resp).convert();
    println!("{}", md);
    Ok(())
}
