use criterion::{criterion_group, criterion_main, Criterion};
use snowpity_tg::util::media_conv;
use std::hint::black_box;


pub fn resize_bench_group(criterion: &mut Criterion) {

    let image = reqwest::blocking::get("https://derpicdn.net/img/view/2021/9/16/2701452.png")
        .unwrap()
        .bytes()
        .unwrap();

    criterion.bench_function("resize_image_to_bounding_box", |bencher| {
        bencher.iter(|| {
            let val =
                media_conv::resize_image_to_bounding_box_sync(black_box(image.clone()), black_box(2560))
                    .unwrap();
            dbg!(val.len());
            val
        })
    });
}

criterion_group!(benches, resize_bench_group);
criterion_main!(benches);
