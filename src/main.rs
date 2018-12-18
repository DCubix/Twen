extern crate sdl2;
extern crate notify;

mod node;
mod parser;

use self::node::{ NodeGraph, Input };
use self::parser::{ Parser, GraphLoader };

use sdl2::pixels::Color;
use sdl2::rect::Point;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::audio::{ AudioCallback, AudioSpecDesired };

use notify::{ Watcher, RecursiveMode, DebouncedEvent };
use std::sync::mpsc;
use std::sync::mpsc::{ Receiver, Sender };
use std::time::Duration;
use std::fs::File;
use std::fs;
use std::path::Path;

struct AudioOutput {
	rx: Receiver<Vec<f32>>
}

impl AudioCallback for AudioOutput {
	type Channel = f32;

	fn callback(&mut self, out: &mut [f32]) {
		let data = self.rx.recv().unwrap();
		out.copy_from_slice(&data);
	}
}

fn main() {
	let sdl = sdl2::init().unwrap();
	let video = sdl.video().unwrap();
	let audio = sdl.audio().unwrap();

	let window = video
		.window("Twen", 640, 480)
		.position_centered()
		.build()
		.unwrap();
	let mut canvas = window.into_canvas().build().unwrap();

	let desired_spec = AudioSpecDesired {
		freq: Some(44100),
		channels: Some(1),
		samples: Some(1024)
	};

	let (mut audioSender, audioRecv) = mpsc::channel();
	let mut device = audio.open_playback(None, &desired_spec, |spec| {
		AudioOutput {
			rx: audioRecv
		}
	}).unwrap();
	device.resume();

	// Synth file
	let path = Path::new("synth.twg");
	if !path.exists() {
		File::create(path).expect("Failed to create file.");
		fs::write(path, "Output(0.0)").expect("Failed to write to file.");
	}

	// Node graph
	let mut loader = GraphLoader::new(path.to_str().unwrap());
	let mut graph = loader.load();

	// File changes listener
	let (tx, rx) = mpsc::channel();
	let mut watcher = notify::watcher(tx, Duration::from_secs(1)).expect("Failed to watch file.");
	watcher.watch(path, RecursiveMode::NonRecursive).unwrap();

	let mut event_pump = sdl.event_pump().unwrap();
	'running: loop {
		for event in rx.try_iter() {
			match event {
				DebouncedEvent::NoticeRemove(_) => {
					File::create(path).expect("Failed to create file.");
					fs::write(path, "Output(0.0)").expect("Failed to write to file.");
					let mut loader = GraphLoader::new(path.to_str().unwrap());
					graph = loader.load();
				},
				DebouncedEvent::NoticeWrite(_) => {
					let mut loader = GraphLoader::new(path.to_str().unwrap());
					graph = loader.load();
				},
				_ => {}
			}
		}

		for event in event_pump.poll_iter() {
			match event {
				Event::Quit {..} |
				Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
					break 'running
				},
				_ => {}
			}
		}

		let mut samples = Vec::new();
		for _ in 0..1024 {
			samples.push(graph.sample());
		}
		let samples_data = samples.clone();
		audioSender.send(samples).unwrap();

		canvas.set_draw_color(Color::RGB(0, 0, 0));
		canvas.clear();

		canvas.set_draw_color(Color::RGB(0, 200, 55));

		let mut px = 0;
		let mut py = 240;
		let step = 640.0 / 512.0;
		for i in (0..640).step_by(step as usize) {
			let s = (samples_data[640 - i] * 400.0) as i32;
			let y = 240 - s;

			canvas.draw_line(Point::new(px, py), Point::new(i as i32, y));

			py = y;
			px = i as i32;
		}

		canvas.present();
		::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
	}
}
