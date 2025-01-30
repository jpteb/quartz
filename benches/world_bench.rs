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

    c.bench_function("world_spawn", |b| {
        b.iter(|| {
            let _entity = world.spawn(black_box(MyComponent {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            }));
        })
    });

    let mut world = World::new();
    const ENTITY_COUNT: u32 = 1000;
    // const ENTITY_COUNT: u32 = 1000000;
    for i in 0..ENTITY_COUNT {
        world.spawn(MyComponent {
            x: i as f32,
            y: (i + 1) as f32,
            z: (i + 2) as f32,
        });
    }

    c.bench_function("world_query", |b| {
        b.iter(|| {
            let query = world.query::<MyComponent>();

            for component in query {
                black_box(component);
            }
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
