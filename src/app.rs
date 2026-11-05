use minifb::{Key, KeyRepeat, Window, WindowOptions};
use std::{
    error::Error,
    path::PathBuf,
    time::{Duration, Instant},
};

use crate::{HEIGHT, ROTATION_SPEED, WIDTH};
use crate::{
    framebuffer::Framebuffer,
    scene::{self, Body, Models},
};

fn screenshot(fb: &Framebuffer, body: Body) -> Result<(), Box<dyn Error>> {
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("screenshots");
    std::fs::create_dir_all(&folder)?;
    let path = folder.join(format!("{}.png", body.slug()));
    fb.save_png(&path)?;
    println!("Captura guardada: {}", path.display());
    Ok(())
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["--help"] {
        println!(
            "cargo run --release [-- --screenshots]\n1: estrella | 2: rocoso | 3: gaseoso\nFlechas: orbitar | Espacio: animar/pausar | E: luna/anillos | H: malla | R: reiniciar | S: captura | Esc: salir"
        );
        return Ok(());
    }
    if !args.is_empty() && args != ["--screenshots"] {
        return Err("Argumentos inválidos. Usa --help o --screenshots.".into());
    }
    let models = Models::load()?;
    let mut fb = Framebuffer::new(WIDTH, HEIGHT);
    fb.set_background(crate::cosmos::generate(WIDTH, HEIGHT));
    if args == ["--screenshots"] {
        for body in [Body::Star, Body::Rocky, Body::Gas] {
            scene::render(&mut fb, &models, body, &scene::camera(), 0.0, true, false);
            screenshot(&fb, body)?;
        }
        return Ok(());
    }

    let mut body = Body::Star;
    let mut camera = scene::camera();
    let mut hologram = false;
    let mut extras = true;
    let mut animate = false;
    let mut time = 0.0;
    let mut dirty = true;
    let mut last_frame = Instant::now();
    let mut window = Window::new("LAB5", WIDTH, HEIGHT, WindowOptions::default())?;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let now = Instant::now();
        let delta = (now - last_frame).as_secs_f32();
        last_frame = now;
        for (key, selected) in [
            (Key::Key1, Body::Star),
            (Key::Key2, Body::Rocky),
            (Key::Key3, Body::Gas),
        ] {
            if window.is_key_pressed(key, KeyRepeat::No) {
                body = selected;
                camera = scene::camera();
                time = 0.0;
                dirty = true;
            }
        }
        for (key, flag) in [
            (Key::H, &mut hologram),
            (Key::E, &mut extras),
            (Key::Space, &mut animate),
        ] {
            if window.is_key_pressed(key, KeyRepeat::No) {
                *flag = !*flag;
                dirty = true;
            }
        }
        if window.is_key_pressed(Key::R, KeyRepeat::No) {
            camera = scene::camera();
            time = 0.0;
            dirty = true;
        }
        let speed = ROTATION_SPEED * 60.0 * delta.min(0.1);
        for (key, yaw, pitch) in [
            (Key::Left, -speed, 0.0),
            (Key::Right, speed, 0.0),
            (Key::Up, 0.0, -speed),
            (Key::Down, 0.0, speed),
        ] {
            if window.is_key_down(key) {
                camera.orbit(yaw, pitch);
                dirty = true;
            }
        }
        if animate {
            time += delta;
            dirty = true;
        }
        if dirty {
            scene::render(&mut fb, &models, body, &camera, time, extras, hologram);
            let state = if animate { "Animando" } else { "Pausado" };
            window.set_title(&format!("{} | {} | 1/2/3: cuerpos | Flechas | Espacio | E: extras | H: malla | R: reset | S: PNG", body.name(), state));
            dirty = false;
        }
        if window.is_key_pressed(Key::S, KeyRepeat::No) {
            screenshot(&fb, body)?;
        }
        window.update_with_buffer(&fb.buffer, WIDTH, HEIGHT)?;
        std::thread::sleep(Duration::from_millis(8));
    }
    Ok(())
}
