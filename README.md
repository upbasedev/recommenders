## Recommenders

[![Crates.io badge](https://img.shields.io/crates/v/recommenders.svg)](https://crates.io/crates/recommenders)
[![Docs.rs badge](https://docs.rs/recommenders/badge.svg)](https://docs.rs/recommenders/)
[![Build Status](https://travis-ci.org/recommenders/recommenders.svg?branch=master)](https://travis-ci.org/apibillme/recommenders)

`recommenders` implements efficient recommender algorithms which operate on
sequences of items: given previous items a user has interacted with,
the model will recommend the items the user is likely to interact with
in the future.

Implemented models:
- LSTM: a model that uses an LSTM network over the sequence of a user's interaction
        to predict their next action;
- EWMA: a model that uses a simpler exponentially-weighted average of past actions
        to predict future interactions.

Which model performs the best will depend on your dataset. The EWMA model is much
quicker to fit, and will probably be a good starting point.

### Example
You can fit a model on the Movielens 100K dataset in about 10 seconds:

```rust
let mut data = recommenders::datasets::download_movielens_100k().unwrap();

let mut rng = rand::XorShiftRng::from_seed([42; 16]);

let (train, test) = recommenders::data::user_based_split(&mut data, &mut rng, 0.2);
let train_mat = train.to_compressed();
let test_mat = test.to_compressed();

println!("Train: {}, test: {}", train.len(), test.len());

let mut model = recommenders::models::lstm::Hyperparameters::new(data.num_items(), 32)
    .embedding_dim(32)
    .learning_rate(0.16)
    .l2_penalty(0.0004)
    .lstm_variant(recommenders::models::lstm::LSTMVariant::Normal)
    .loss(recommenders::models::Loss::WARP)
    .optimizer(recommenders::models::Optimizer::Adagrad)
    .num_epochs(10)
    .rng(rng)
    .build();

let start = Instant::now();
let loss = model.fit(&train_mat).unwrap();
let elapsed = start.elapsed();
let train_mrr = recommenders::evaluation::mrr_score(&model, &train_mat).unwrap();
let test_mrr = recommenders::evaluation::mrr_score(&model, &test_mat).unwrap();

println!(
    "Train MRR {} at loss {} and test MRR {} (in {:?})",
    train_mrr, loss, test_mrr, elapsed
);
```

License: MIT
