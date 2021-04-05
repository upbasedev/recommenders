#[macro_use]
extern crate criterion;

extern crate csv;
extern crate rand;
extern crate recommenders;
extern crate serde;
extern crate serde_json;
extern crate wyrm;

use criterion::Criterion;

use recommenders::data::{Interaction, Interactions};
use recommenders::models::{ewma, lstm};
use recommenders::models::{Loss, Optimizer};

fn load_movielens(path: &str, sample_size: usize) -> Interactions {
    let mut reader = csv::Reader::from_path(path).unwrap();
    let interactions: Vec<Interaction> = reader.deserialize().map(|x| x.unwrap()).collect();

    let interactions = rand::seq::sample_slice(&mut rand::thread_rng(), &interactions, sample_size);

    Interactions::from(interactions)
}

fn bench_lstm(c: &mut Criterion) {
    c.bench_function("lstm", |b| {
        let data = load_movielens("data.csv", 10000).to_compressed();

        let mut model = lstm::Hyperparameters::new(data.num_items(), 128)
            .embedding_dim(32)
            .learning_rate(0.16)
            .l2_penalty(0.0004)
            .loss(Loss::Hinge)
            .optimizer(Optimizer::Adagrad)
            .num_epochs(3)
            .num_threads(1)
            .build();

        b.iter(|| {
            model.fit(&data).unwrap();
        })
    });
}

fn bench_ewma(c: &mut Criterion) {
    c.bench_function("ewma", |b| {
        let data = load_movielens("data.csv", 10000).to_compressed();

        let mut model = ewma::Hyperparameters::new(data.num_items(), 128)
            .embedding_dim(32)
            .learning_rate(0.16)
            .l2_penalty(0.0004)
            .loss(Loss::Hinge)
            .optimizer(Optimizer::Adagrad)
            .num_epochs(3)
            .num_threads(1)
            .build();

        b.iter(|| {
            model.fit(&data).unwrap();
        })
    });
}

criterion_group!{
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_lstm, bench_ewma
}
criterion_main!(benches);
