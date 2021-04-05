#![deny(missing_docs, missing_debug_implementations)]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::type_complexity))]
//! # sbr-rs
//!
//! `sbr` implements efficient recommender algorithms which operate on
//! sequences of items: given previous items a user has interacted with,
//! the model will recommend the items the user is likely to interact with
//! in the future.
//!
//! Implemented models:
//! - LSTM: a model that uses an LSTM network over the sequence of a user's interaction
//!         to predict their next action;
//! - EWMA: a model that uses a simpler exponentially-weighted average of past actions
//!         to predict future interactions.
//!
//! Which model performs the best will depend on your dataset. The EWMA model is much
//! quicker to fit, and will probably be a good starting point.
//!
//! ## Example
//! You can fit a model on the Movielens 100K dataset in about 10 seconds:
//!
//! ```rust
//! # extern crate sbr;
//! # extern crate rand;
//! # use std::time::Instant;
//! # use rand::SeedableRng;
//! let mut data = sbr::datasets::download_movielens_100k().unwrap();
//!
//! let mut rng = rand::XorShiftRng::from_seed([42; 16]);
//!
//! let (train, test) = sbr::data::user_based_split(&mut data, &mut rng, 0.2);
//! let train_mat = train.to_compressed();
//! let test_mat = test.to_compressed();
//!
//! println!("Train: {}, test: {}", train.len(), test.len());
//!
//! let mut model = sbr::models::lstm::Hyperparameters::new(data.num_items(), 32)
//!     .embedding_dim(32)
//!     .learning_rate(0.16)
//!     .l2_penalty(0.0004)
//!     .lstm_variant(sbr::models::lstm::LSTMVariant::Normal)
//!     .loss(sbr::models::Loss::WARP)
//!     .optimizer(sbr::models::Optimizer::Adagrad)
//!     .num_epochs(10)
//!     .rng(rng)
//!     .build();
//!
//! let start = Instant::now();
//! let loss = model.fit(&train_mat).unwrap();
//! let elapsed = start.elapsed();
//! let train_mrr = sbr::evaluation::mrr_score(&model, &train_mat).unwrap();
//! let test_mrr = sbr::evaluation::mrr_score(&model, &test_mat).unwrap();
//!
//! println!(
//!     "Train MRR {} at loss {} and test MRR {} (in {:?})",
//!     train_mrr, loss, test_mrr, elapsed
//! );
//! ```
#[macro_use]
extern crate itertools;
extern crate csv;
#[macro_use]
extern crate failure;
pub mod data;
pub mod datasets;
pub mod evaluation;
pub mod models;

/// Alias for user indices.
pub type UserId = usize;
/// Alias for item indices.
pub type ItemId = usize;
/// Alias for timestamps.
pub type Timestamp = usize;

/// Prediction error types.
#[derive(Debug, Fail)]
pub enum PredictionError {
    /// Failed prediction due to numerical issues.
    #[fail(display = "Invalid prediction value: non-finite or not a number.")]
    InvalidPredictionValue,
}

/// Fitting error types.
#[derive(Debug, Fail)]
pub enum FittingError {
    /// No interactions were given.
    #[fail(display = "No interactions were supplied.")]
    NoInteractions,
}

/// Trait describing models that can compute predictions given
/// a user's sequences of past interactions.
pub trait OnlineRankingModel {
    /// The representation the model computes from past interactions.
    type UserRepresentation: std::fmt::Debug;
    /// Compute a user representation from past interactions.
    fn user_representation(
        &self,
        item_ids: &[ItemId],
    ) -> Result<Self::UserRepresentation, PredictionError>;
    /// Given a user representation, rank `item_ids` according
    /// to how likely the user is to interact with them in the future.
    fn predict(
        &self,
        user: &Self::UserRepresentation,
        item_ids: &[ItemId],
    ) -> Result<Vec<f32>, PredictionError>;
}
