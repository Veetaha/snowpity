use criterion::{criterion_group, criterion_main, Criterion};
use snowpity_tg::util::media_conv;
use std::hint::black_box;

pub fn criterion_benchmark(criterion: &mut Criterion) {
    // https://derpibooru.org/images/2561187
    let image = reqwest::blocking::get("https://derpicdn.net/img/view/2021/9/16/2701452.png")
        .unwrap()
        .bytes()
        .unwrap();

    criterion.bench_function("resize_image_to_bounding_box", |bencher| {
        bencher.iter(|| {
            media_conv::resize_image_to_bounding_box_sync(black_box(&image), black_box(2560))
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
