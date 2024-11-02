use criterion::{black_box, criterion_group, criterion_main, Criterion};
use quartz::{component::Component, World};

struct MyComponent {
    x: f32,
    y: f32,
    z: f32,
}
impl Component for MyComponent {}

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut world = World::new();

    c.bench_function("world spawn", |b| {
        b.iter(|| {
            let _entity = world.spawn(black_box(MyComponent {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            }));
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
