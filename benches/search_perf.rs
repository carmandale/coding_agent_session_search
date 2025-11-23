use coding_agent_search::default_data_dir;
use coding_agent_search::search::query::{SearchClient, SearchFilters};
use coding_agent_search::search::tantivy::index_dir;
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_empty_search(c: &mut Criterion) {
    let data_dir = default_data_dir();
    let index_path = index_dir(&data_dir).unwrap();
    let client = SearchClient::open(&index_path, None).unwrap();
    if let Some(client) = client {
        c.bench_function("search_empty_query", |b| {
            b.iter(|| {
                client
                    .search("", SearchFilters::default(), 10, 0)
                    .unwrap_or_default()
            })
        });
    }
}

criterion_group!(benches, bench_empty_search);
criterion_main!(benches);
