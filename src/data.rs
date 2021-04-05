//! Funcionality for manipulating data.

use std;
use std::cmp::Ordering;
use std::hash::Hasher;

use rand::distributions::{Distribution, Uniform};
use rand::Rng;

use serde::{Deserialize, Serialize};
use siphasher::sip::SipHasher;

use super::{ItemId, Timestamp, UserId};

/// Basic interaction type.
#[derive(Clone, Serialize, Deserialize, Debug, Eq, Hash, PartialEq)]
pub struct Interaction {
    user_id: UserId,
    item_id: ItemId,
    timestamp: Timestamp,
}

impl Interaction {
    /// Create a new interaction.
    pub fn new(user_id: UserId, item_id: ItemId, timestamp: Timestamp) -> Self {
        Interaction {
            user_id,
            item_id,
            timestamp,
        }
    }
}

impl Interaction {
    /// Return the user id.
    pub fn user_id(&self) -> UserId {
        self.user_id
    }
    /// Return the item id.
    pub fn item_id(&self) -> ItemId {
        self.item_id
    }
    /// Return the interaction weight.
    pub fn weight(&self) -> f32 {
        1.0
    }
    /// Return the interaction timestamp.
    pub fn timestamp(&self) -> Timestamp {
        self.timestamp
    }
}

/// Randomly split interactions between test and traiing sets.
pub fn train_test_split<R: Rng>(
    interactions: &mut Interactions,
    rng: &mut R,
    test_fraction: f32,
) -> (Interactions, Interactions) {
    interactions.shuffle(rng);

    let (test, train) = interactions.split_at((test_fraction * interactions.len() as f32) as usize);

    (train, test)
}

/// Split interactions between training and test sets so that no user is in both sets.
/// Useful for testing generalization where we want to test the model's performance on
/// users who have not been seen during training.
pub fn user_based_split<R: Rng>(
    interactions: &Interactions,
    rng: &mut R,
    test_fraction: f32,
) -> (Interactions, Interactions) {
    let denominator = 100_000;
    let train_cutoff = (test_fraction * denominator as f32) as u64;

    let range = Uniform::new(0, std::u64::MAX);
    let (key_0, key_1) = (range.sample(rng), range.sample(rng));

    let is_train = |x: &Interaction| {
        let mut hasher = SipHasher::new_with_keys(key_0, key_1);
        let user_id = x.user_id();
        hasher.write_usize(user_id);
        hasher.finish() % denominator > train_cutoff
    };

    interactions.split_by(is_train)
}

/// A collection of individual interactions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Interactions {
    num_users: usize,
    num_items: usize,
    interactions: Vec<Interaction>,
}

impl Interactions {
    /// Crate a new interactions object.
    pub fn new(num_users: usize, num_items: usize) -> Self {
        Interactions {
            num_users,
            num_items,
            interactions: Vec::new(),
        }
    }
    /// Add a new interaction.
    pub fn push(&mut self, interaction: Interaction) {
        self.interactions.push(interaction);
    }

    /// Return the underlying data.
    pub fn data(&self) -> &[Interaction] {
        &self.interactions
    }

    /// Give the number of contained interactions.
    pub fn len(&self) -> usize {
        self.interactions.len()
    }

    /// Check if there are no interactions.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Shuffle the interactions in-place.
    pub fn shuffle<R: Rng>(&mut self, rng: &mut R) {
        rng.shuffle(&mut self.interactions);
    }

    /// Split interactions at `idx`.
    pub fn split_at(&self, idx: usize) -> (Self, Self) {
        let head = Interactions {
            num_users: self.num_users,
            num_items: self.num_items,
            interactions: self.interactions[..idx].to_owned(),
        };
        let tail = Interactions {
            num_users: self.num_users,
            num_items: self.num_items,
            interactions: self.interactions[idx..].to_owned(),
        };

        (head, tail)
    }

    /// Split interactions by predicate.
    pub fn split_by<F: Fn(&Interaction) -> bool>(&self, func: F) -> (Self, Self) {
        let head = Interactions {
            num_users: self.num_users,
            num_items: self.num_items,
            interactions: self
                .interactions
                .iter()
                .filter(|x| func(x))
                .cloned()
                .collect(),
        };
        let tail = Interactions {
            num_users: self.num_users,
            num_items: self.num_items,
            interactions: self
                .interactions
                .iter()
                .filter(|x| !func(x))
                .cloned()
                .collect(),
        };

        (head, tail)
    }

    /// Covert to triplet representation.
    pub fn to_triplet(&self) -> TripletInteractions {
        TripletInteractions::from(self)
    }

    /// Convert to compressed representation.
    pub fn to_compressed(&self) -> CompressedInteractions {
        CompressedInteractions::from(self)
    }

    /// Return number of users.
    pub fn num_users(&self) -> usize {
        self.num_users
    }

    /// Return number of items.
    pub fn num_items(&self) -> usize {
        self.num_items
    }

    /// Return (`num_users`, `num_items`).
    pub fn shape(&self) -> (usize, usize) {
        (self.num_users, self.num_items)
    }
}

impl From<Vec<Interaction>> for Interactions {
    fn from(interactions: Vec<Interaction>) -> Interactions {
        let num_users = interactions.iter().map(|x| x.user_id()).max().unwrap() + 1;
        let num_items = interactions.iter().map(|x| x.item_id()).max().unwrap() + 1;

        Interactions {
            num_users,
            num_items,
            interactions,
        }
    }
}

fn cmp_timestamp(x: &Interaction, y: &Interaction) -> Ordering {
    let uid_comparison = x.user_id().cmp(&y.user_id());

    if uid_comparison == Ordering::Equal {
        x.timestamp().cmp(&y.timestamp())
    } else {
        uid_comparison
    }
}

/// A compressed representation of interactions, where the
/// interactions themselves are arranged by user and by timestamp.
///
/// Normally created by [Interactions::to_compressed].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompressedInteractions {
    num_users: usize,
    num_items: usize,
    user_pointers: Vec<usize>,
    item_ids: Vec<ItemId>,
    timestamps: Vec<Timestamp>,
}

impl<'a> From<&'a Interactions> for CompressedInteractions {
    fn from(interactions: &Interactions) -> CompressedInteractions {
        let mut data = interactions.data().to_owned();

        data.sort_by(cmp_timestamp);

        let mut user_pointers = vec![0; interactions.num_users + 1];
        let mut item_ids = Vec::with_capacity(data.len());
        let mut timestamps = Vec::with_capacity(data.len());

        for datum in &data {
            item_ids.push(datum.item_id());
            timestamps.push(datum.timestamp());

            user_pointers[datum.user_id() + 1] += 1;
        }

        for idx in 1..user_pointers.len() {
            user_pointers[idx] += user_pointers[idx - 1];
        }

        CompressedInteractions {
            num_users: interactions.num_users,
            num_items: interactions.num_items,
            user_pointers,
            item_ids,
            timestamps,
        }
    }
}

impl CompressedInteractions {
    /// Iterate over users.
    pub fn iter_users(&self) -> CompressedInteractionsUserIterator {
        CompressedInteractionsUserIterator {
            interactions: self,
            idx: 0,
        }
    }

    /// Get a particular user's interactions.
    pub fn get_user(&self, user_id: UserId) -> Option<CompressedInteractionsUser> {
        if user_id >= self.num_users {
            return None;
        }

        let start = self.user_pointers[user_id];
        let stop = self.user_pointers[user_id + 1];

        Some(CompressedInteractionsUser {
            user_id,
            item_ids: &self.item_ids[start..stop],
            timestamps: &self.timestamps[start..stop],
        })
    }

    /// Return number of users.
    pub fn num_users(&self) -> usize {
        self.num_users
    }

    /// Return number of items.
    pub fn num_items(&self) -> usize {
        self.num_items
    }

    /// Return (`num_users`, `num_items`).
    pub fn shape(&self) -> (usize, usize) {
        (self.num_users, self.num_items)
    }

    /// Convert to `Interactions`.
    pub fn to_interactions(&self) -> Interactions {
        let mut interactions = Vec::new();

        for user in self.iter_users() {
            for (&item_id, &timestamp) in izip!(user.item_ids, user.timestamps) {
                interactions.push(Interaction {
                    user_id: user.user_id,
                    item_id,
                    timestamp,
                });
            }
        }

        interactions.shrink_to_fit();

        Interactions {
            num_users: self.num_users,
            num_items: self.num_items,
            interactions,
        }
    }
}

/// Iterator over compressed user data.
#[derive(Clone, Debug)]
pub struct CompressedInteractionsUserIterator<'a> {
    interactions: &'a CompressedInteractions,
    idx: usize,
}

/// A single user's data, arranged from earliest to latest.
#[derive(Debug, Clone)]
pub struct CompressedInteractionsUser<'a> {
    /// User id.
    pub user_id: UserId,
    /// The users's interactions.
    pub item_ids: &'a [ItemId],
    /// The timestamps of the user's interactions.
    pub timestamps: &'a [Timestamp],
}

impl<'a> CompressedInteractionsUser<'a> {
    /// Return length of interactions.
    pub fn len(&self) -> usize {
        self.item_ids.len()
    }

    /// Check if there are no interactions.
    pub fn is_empty(&self) -> bool {
        self.item_ids.is_empty()
    }

    /// Return a chunked iterator over interactions for this user.
    /// The chunks are such that the _first_ chunk is smallest,
    /// and the remaining chunks are all of `chunk_size`.
    pub fn chunks(&self, chunk_size: usize) -> CompressedInteractionsUserChunkIterator<'a> {
        CompressedInteractionsUserChunkIterator {
            idx: 0,
            chunk_size,
            item_ids: &self.item_ids[..],
            timestamps: &self.timestamps[..],
        }
    }
}

impl<'a> Iterator for CompressedInteractionsUserIterator<'a> {
    type Item = CompressedInteractionsUser<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let value = if self.idx >= self.interactions.num_users {
            None
        } else {
            let start = self.interactions.user_pointers[self.idx];
            let stop = self.interactions.user_pointers[self.idx + 1];

            Some(CompressedInteractionsUser {
                user_id: self.idx,
                item_ids: &self.interactions.item_ids[start..stop],
                timestamps: &self.interactions.timestamps[start..stop],
            })
        };

        self.idx += 1;

        value
    }
}

/// Chunked iterator over a user's interactions.
/// The chunks are such that the _first_ chunk is smallest,
/// and the remaining chunks are all of `chunk_size`.
#[derive(Debug, Clone)]
pub struct CompressedInteractionsUserChunkIterator<'a> {
    idx: usize,
    chunk_size: usize,
    item_ids: &'a [ItemId],
    timestamps: &'a [Timestamp],
}

impl<'a> Iterator for CompressedInteractionsUserChunkIterator<'a> {
    type Item = (&'a [ItemId], &'a [Timestamp]);
    fn next(&mut self) -> Option<Self::Item> {
        let user_len = self.item_ids.len();

        if self.idx >= user_len {
            None
        } else {
            let chunk_size_mod = (user_len - self.idx) % self.chunk_size;
            let chunk_size = if chunk_size_mod == 0 {
                self.chunk_size
            } else {
                chunk_size_mod
            };

            let start_idx = self.idx;
            let stop_idx = self.idx + chunk_size;

            self.idx += chunk_size;

            Some((
                &self.item_ids[start_idx..stop_idx],
                &self.timestamps[start_idx..stop_idx],
            ))
        }
    }
}

/// Interactions in COO form.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TripletInteractions {
    num_users: usize,
    num_items: usize,
    user_ids: Vec<UserId>,
    pub(crate) item_ids: Vec<ItemId>,
    timestamps: Vec<Timestamp>,
}

impl TripletInteractions {
    /// Return lenght.
    pub fn len(&self) -> usize {
        self.user_ids.len()
    }

    /// Check if there are no interactions.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Iterate over minibatches of size `minibatch_size`.
    pub fn iter_minibatch(&self, minibatch_size: usize) -> TripletMinibatchIterator {
        TripletMinibatchIterator {
            interactions: self,
            idx: 0,
            stop_idx: self.len(),
            minibatch_size,
        }
    }

    /// Return a collection of iterators over a partitions of the data.
    pub fn iter_minibatch_partitioned(
        &self,
        minibatch_size: usize,
        num_partitions: usize,
    ) -> Vec<TripletMinibatchIterator> {
        let iterator = self.iter_minibatch(minibatch_size);
        let chunk_size = self.len() / num_partitions;

        (0..num_partitions)
            .map(|x| iterator.slice(x * chunk_size, (x + 1) * chunk_size))
            .collect()
    }

    /// Return number of users in the dataset.
    pub fn num_users(&self) -> usize {
        self.num_users
    }

    /// Return number of users in the dataset.
    pub fn num_items(&self) -> usize {
        self.num_items
    }

    /// Return (num_users, num_items).
    pub fn shape(&self) -> (usize, usize) {
        (self.num_users, self.num_items)
    }
}

/// Iterator over minibatches of triplet interactions.
#[derive(Clone, Debug)]
pub struct TripletMinibatchIterator<'a> {
    interactions: &'a TripletInteractions,
    idx: usize,
    stop_idx: usize,
    minibatch_size: usize,
}

impl<'a> TripletMinibatchIterator<'a> {
    /// Slice the iterator, yielding an iterator over a subslice of the data.
    pub fn slice(&self, start: usize, stop: usize) -> TripletMinibatchIterator<'a> {
        TripletMinibatchIterator {
            interactions: self.interactions,
            idx: start,
            stop_idx: stop,
            minibatch_size: self.minibatch_size,
        }
    }
}

/// A minibatch of triplet interactions.
#[derive(Debug, Clone)]
pub struct TripletMinibatch<'a> {
    /// User ids in the batch.
    pub user_ids: &'a [UserId],
    /// Item ids in the batch.
    pub item_ids: &'a [ItemId],
    /// Timestamps in the batch.
    pub timestamps: &'a [Timestamp],
}

impl<'a> TripletMinibatch<'a> {
    /// Return length of the minibatch.
    pub fn len(&self) -> usize {
        self.user_ids.len()
    }

    /// Check if there are no interactions.
    pub fn is_empty(&self) -> bool {
        self.item_ids.is_empty()
    }
}

impl<'a> Iterator for TripletMinibatchIterator<'a> {
    type Item = TripletMinibatch<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let value = if self.idx + self.minibatch_size > self.stop_idx {
            None
        } else {
            let start = self.idx;
            let stop = self.idx + self.minibatch_size;

            Some(TripletMinibatch {
                user_ids: &self.interactions.user_ids[start..stop],
                item_ids: &self.interactions.item_ids[start..stop],
                timestamps: &self.interactions.timestamps[start..stop],
            })
        };

        self.idx += self.minibatch_size;

        value
    }
}

impl<'a> From<&'a Interactions> for TripletInteractions {
    fn from(interactions: &'a Interactions) -> Self {
        let user_ids = interactions.data().iter().map(|x| x.user_id()).collect();
        let item_ids = interactions.data().iter().map(|x| x.item_id()).collect();
        let timestamps = interactions.data().iter().map(|x| x.timestamp()).collect();

        TripletInteractions {
            num_users: interactions.num_users,
            num_items: interactions.num_items,
            user_ids,
            item_ids,
            timestamps,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use rand;
    use rand::distributions::{Distribution, Uniform};
    use rand::SeedableRng;

    use super::*;

    #[test]
    fn to_compressed() {
        let num_users = 20;
        let num_items = 20;
        let num_interactions = 100;

        let user_range = Uniform::new(0, num_users);
        let item_range = Uniform::new(0, num_items);
        let timestamp_range = Uniform::new(0, 50);

        let mut rng = rand::XorShiftRng::from_seed([42; 16]);

        let interactions: Vec<_> = (0..num_interactions)
            .map(|_| Interaction {
                user_id: user_range.sample(&mut rng),
                item_id: item_range.sample(&mut rng),
                timestamp: timestamp_range.sample(&mut rng),
            })
            .collect();

        let mut interaction_set = HashSet::with_capacity(interactions.len());
        for interaction in &interactions {
            interaction_set.insert(interaction.clone());
        }

        let mut interactions = Interactions {
            num_users,
            num_items,
            interactions,
        };
        let (train, test) = user_based_split(&mut interactions, &mut rng, 0.5);

        let train = train.to_compressed().to_interactions();
        let test = test.to_compressed().to_interactions();

        assert_eq!(train.len() + test.len(), interaction_set.len());

        for interaction in train.data().iter().chain(test.data().iter()) {
            assert!(interaction_set.contains(interaction));
        }
    }

    #[test]
    fn test_chunk_iterator() {
        let num_users = 1;
        let num_items = 5;

        let mut interactions = Vec::new();

        for user in 0..num_users {
            for item in 0..num_items {
                interactions.push(Interaction::new(user, item, item));
            }
        }

        let interactions = Interactions::from(interactions).to_compressed();

        let chunks: Vec<_> = interactions
            .iter_users()
            .flat_map(|user| user.chunks(3))
            .collect();

        assert_eq!(chunks.len(), 2);

        let expected = [
            (vec![0, 1_usize], vec![0, 1_usize]),
            (vec![2_usize, 3, 4], vec![2_usize, 3, 4]),
        ];

        chunks.iter().zip(expected.iter()).for_each(|(x, y)| {
            assert_eq!(&x.0, &y.0.as_slice());
            assert_eq!(&x.0, &y.1.as_slice());
        });

        //assert!(chunks == []);
    }

    // #[test]
    // fn foo_bar() {
    //     let mut interactions = Vec::new();

    //     for user_id in 0..10 {
    //         for item_id in 0..10 {
    //             interactions.push(Interaction {
    //                 user_id: user_id,
    //                 item_id: item_id + 1000 * user_id,
    //                 timestamp: item_id,
    //             });
    //         }
    //     }

    //     let interactions = Interactions {
    //         num_users: 10,
    //         num_items: interactions.iter().map(|x| x.item_id).max().unwrap() + 1,
    //         interactions: interactions,
    //     };

    //     let mut rng = rand::thread_rng();
    //     let (train, test) = user_based_split(&interactions, &mut rng, 0.5);

    //     let train = train.to_compressed();
    //     let test = test.to_compressed();

    //     for user in train.iter_users() {
    //         println!("Train {:#?}", user);
    //     }
    //     for user in test.iter_users() {
    //         println!("Test {:#?}", user);
    //     }
    // }
}
