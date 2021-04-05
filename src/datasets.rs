//! Built-in datasets for easy testing and experimentation.
use csv;
use failure;
use reqwest;

use crate::data::{Interaction, Interactions};

/// Dataset error types.
#[derive(Debug, Fail)]
pub enum DatasetError {
    /// Can't find the home directory.
    #[fail(display = "Cannot find home directory.")]
    NoHomeDir,
}

async fn download(url: &str) -> Result<Interactions, failure::Error> {
    let str = reqwest::get(url).await?.text().await?;

    let mut reader = csv::Reader::from_reader(str.as_bytes());
    let interactions: Vec<Interaction> = reader.deserialize().collect::<Result<Vec<_>, _>>()?;

    Ok(Interactions::from(interactions))
}

/// Download the Movielens 100K dataset and return it.
///
/// The data is stored in `~/.sbr-rs/`.
pub async fn download_movielens_100k() -> Result<Interactions, failure::Error> {
    Ok(download(
        "https://github.com/maciejkula/sbr-rs/raw/master/data.csv"
    ).await?)
}
