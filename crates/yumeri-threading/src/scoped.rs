use crate::parallelism;

pub fn par_for_each<T: Sync>(items: &[T], min_batch: usize, f: impl Fn(&T) + Send + Sync) {
    if items.len() < min_batch {
        items.iter().for_each(&f);
        return;
    }

    let chunk_size = items.len().div_ceil(parallelism());

    std::thread::scope(|s| {
        for chunk in items.chunks(chunk_size) {
            s.spawn(|| {
                chunk.iter().for_each(&f);
            });
        }
    });
}

pub fn par_map<T: Sync, U: Send>(
    items: &[T],
    min_batch: usize,
    f: impl Fn(&T) -> U + Send + Sync,
) -> Vec<U> {
    if items.len() < min_batch {
        return items.iter().map(&f).collect();
    }

    let chunk_size = items.len().div_ceil(parallelism());

    std::thread::scope(|s| {
        let handles: Vec<_> = items
            .chunks(chunk_size)
            .map(|chunk| s.spawn(|| chunk.iter().map(&f).collect::<Vec<_>>()))
            .collect();

        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect()
    })
}
