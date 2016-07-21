extern crate clap;
extern crate env_logger;
#[macro_use] extern crate glium;
#[macro_use] extern crate log;

mod cfg;
mod data;
mod renderer;
mod res;

use clap::{Arg, App};
use renderer::run;

fn is_uint_and_geq_100(s: String) -> Result<(), String> {
    match s.parse::<u32>() {
        Ok(i) => {
            if i >= 100 {
                Ok(())
            } else {
                Err(String::from("Number has to be at least 100"))
            }
        },
        Err(_) => {
            Err(String::from("Not a positive number"))
        }
    }
}


fn main() {
    env_logger::init().unwrap();
    info!("that's fluxcore...booting up!");

    info!("parse command line args");
    let matches = App::new("fluxcore_ng")
        .version("???")
        .author("Marco Neumann")
        .about("fast data renderer")
        .arg(Arg::with_name("width")
             .short("w")
             .long("width")
             .default_value("800")
             .validator(is_uint_and_geq_100))
        .arg(Arg::with_name("height")
             .short("h")
             .long("height")
             .default_value("600")
             .validator(is_uint_and_geq_100))
        .arg(Arg::with_name("file")
             .required(true)
             .index(1)
             .value_name("FILE"))
        .get_matches();
    let width = matches.value_of("width").unwrap().parse::<u32>().unwrap();
    let height = matches.value_of("height").unwrap().parse::<u32>().unwrap();
    let file = String::from(matches.value_of("file").unwrap());

    info!("read data from file");
    let columns = match data::columns_from_file(&file) {
        Ok(c) => c,
        Err(s) => {
            error!("{}", s);
            return;
        }
    };

    run(width, height, columns, file);

    info!("shutting down");
}
