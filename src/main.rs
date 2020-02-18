use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use regex::Captures;

extern crate urlencoding;
use regex::Regex;

use urlencoding::decode;
#[macro_use]
extern crate lazy_static;
use uuid::Uuid;

use std::fs::File;
use std::io::prelude::*;

use tokio::process::Command;

lazy_static! {
    static ref LATEX_CODE: Regex = Regex::new(r"latex=(.+?)&scale=(\d+)").unwrap();
    static ref SCALE: Regex = Regex::new(r"width='(.+?)pt' height='(.+?)pt'").unwrap();
}

async fn greet(req: HttpRequest) -> HttpResponse {
    let name = req.query_string();
    let parms = decode(name).unwrap();
    let caps = LATEX_CODE.captures(parms.as_str()).unwrap();
    let uuid = Uuid::new_v4();

    let templates = format!(
        "\\documentclass{{standalone}}
\\usepackage{{ctex}}    
\\usepackage{{amsmath,amssymb,amstext,amsfonts,upgreek}}
\\begin{{document}}
${0}$
\\end{{document}}",
        &caps[1]
    );

    let scale: f32 = String::from(&caps[2]).parse().unwrap_or(1.0);
    let mut file = File::create(format!("{}.tex", uuid)).unwrap();
    file.write_all(templates.as_bytes()).unwrap();

    Command::new("xelatex")
        .arg(format!("{}.tex", uuid))
        .spawn()
        .expect("xelatex command failed to start")
        .await
        .expect("xelatex command failed to run");

    Command::new("dvisvgm")
        .args(&["-n", "-P", format!("{}.pdf", uuid).as_str()])
        .spawn()
        .expect("dvisvgm command failed to start")
        .await
        .expect("dvisvgm command failed to run");

    let mut svg = File::open(format!("{}.svg", uuid)).unwrap();
    let mut buf = String::new();
    svg.read_to_string(&mut buf).unwrap();

    let result = SCALE.replace(buf.as_str(), |caps: &Captures| {
        let w: f32 = String::from(&caps[1]).parse().unwrap();
        let h: f32 = String::from(&caps[2]).parse().unwrap();
        format!("width='{}pt' height='{}pt'", w * scale, h * scale)
    });

    HttpResponse::Ok()
        .content_type("application/xhtml+xml")
        .body(format!("{}", result))
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().route("/", web::get().to(greet)))
        .bind("127.0.0.1:8000")?
        .run()
        .await
}
