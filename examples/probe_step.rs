use look::timing::Timings;
use look::{config::UpAxis, scene::compile_scene};

fn main() {
    let mut timings = Timings::default();
    let path = std::env::args().nth(1).expect("path");
    match compile_scene(std::path::Path::new(&path), UpAxis::Y, &mut timings) {
        Ok(scene) => {
            println!(
                "OK triangles={} vertices={}",
                scene.geometries[0].indices.len() / 3,
                scene.geometries[0].vertices.len()
            );
            println!("bounds: {:?}..{:?}", scene.bounds.min, scene.bounds.max);
        }
        Err(e) => println!("ERR: {e:#}"),
    }
}
