use std::fs;

use anyhow::Result;
use csv::ReaderBuilder;
use fuzzy_finder::{item::Item, FuzzyFinder};
use log::LevelFilter;
use log4rs::{
    append::file::FileAppender,
    config::{Appender, Root},
    encode::pattern::PatternEncoder,
};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
struct LotrCharacter {
    name: String,
    bio: String,
}

fn main() -> Result<()> {
    let log_file_path = "./lotr_example.log".to_string();
    let log_file = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
        .build(log_file_path)?;
    let config = log4rs::config::Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(log_file)))
        .build(Root::builder().appender("logfile").build(LevelFilter::Info))?;
    log4rs::init_config(config)?;

    // Get some data to display
    let characters_csv = fs::read_to_string("examples/data/lotr_characters.csv")
        .expect("Something went wrong reading the file");
    let mut rdr = ReaderBuilder::new()
        .delimiter(b':')
        .from_reader(characters_csv.as_bytes());

    // Squish the data into an Item
    let mut characters: Vec<Item<LotrCharacter>> = Vec::new();
    for result in rdr.deserialize() {
        let record: LotrCharacter = result?;
        characters.push(Item::new(record.name.clone(), record));
    }

    // Do the find
    let result = FuzzyFinder::find(characters, 8)?;

    // Handle the result
    match result {
        Some(result) => println!(
            "Ah, a fascinating character is {}. Let me tell you about them: {}",
            result.name, result.bio
        ),

        None => println!("Whatever, philistine."),
    }
    Ok(())
}
