// wengwengweng

use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::time::SystemTime;
use std::collections::VecDeque;

use dirty::*;
use math::*;
use gfx::*;
use input::Key;

const LOG_SIZE: usize = 4;

#[derive(Clone, Copy, Debug)]
enum MsgType {
	Success,
	Failure,
}

#[derive(Clone, Debug)]
struct Msg {
	ty: MsgType,
	content: String,
}

impl Msg {
	fn new(ty: MsgType, content: &str) -> Self {
		return Self {
			ty: ty,
			content: String::from(content),
		};
	}
	fn success(content: &str) -> Self {
		return Self::new(MsgType::Success, content);
	}
	fn failure(content: &str) -> Self {
		return Self::new(MsgType::Failure, content);
	}
}

#[derive(Clone)]
struct File {
	path: PathBuf,
	last_modified: Option<SystemTime>,
}

impl File {

	fn new(path: impl AsRef<Path>) -> Self {

		let path = path.as_ref();

		let last_modified = std::fs::metadata(path)
			.ok()
			.map(|d| d.modified().ok())
			.flatten();

		return Self {
			last_modified: last_modified,
			path: path.to_path_buf(),
		};

	}

	fn check_modified(&mut self) -> bool {

		let last_modified = std::fs::metadata(&self.path)
			.ok()
			.map(|d| d.modified().ok())
			.flatten();

		let modified = self.last_modified != last_modified;

		self.last_modified = last_modified;

		return modified;

	}

}

struct Viewer {
	file: Option<File>,
	shader: Option<Shader<GeneralUniform>>,
	log: VecDeque<Msg>,
	show_log: bool,
}

#[derive(Clone)]
struct GeneralUniform {
	resolution: Vec2,
	time: Duration,
	mouse: Vec2,
}

impl CustomUniform for GeneralUniform {
	fn values(&self) -> UniformValues {
		return hmap![
			"u_resolution" => &self.resolution,
			"u_mouse" => &self.mouse,
			"u_time" => &self.time,
		];
	}
}

impl Viewer {

	fn log(&mut self, msg: Msg) {

		use term::style as s;

		match &msg.ty {
			MsgType::Success => println!("{}", s(&msg.content).green()),
			MsgType::Failure => eprintln!("{}", s(&msg.content).red()),
		}

		self.log.push_back(msg);
		self.show_log = true;

		if self.log.len() > LOG_SIZE {
			self.log.pop_front();
		}

	}

	fn refresh(&mut self, d: &mut Ctx) {

		let path = match &self.file {
			Some(file) => &file.path,
			None => return,
		};

		let content = match fs::read_str(path) {
			Ok(content) => content,
			Err(_) => {
				self.log(Msg::failure(&format!("failed to read {}", path.display())));
				return;
			},
		};

		match Shader::from_frag(d.gfx, &content) {

			Ok(shader) => {

				let fname = path
					.file_name()
					.unwrap_or(std::ffi::OsStr::new("unknown file"));

				d.window.set_title(&format!("{:?}", fname));
				self.shader = Some(shader);
				self.log(Msg::success(&format!("{:?} loaded", fname)));

			},

			Err(msg) => self.log(Msg::failure(&msg)),

		}

	}

	fn open(&mut self, d: &mut Ctx, path: impl AsRef<Path>) {

		self.file = Some(File::new(path));
		self.refresh(d);

	}

}

impl State for Viewer {

	fn init(d: &mut Ctx) -> Result<Self> {

		d.window.set_cursor(window::CursorIcon::Cross);

		let mut viewer = Self {
			file: None,
			shader: None,
			log: vecd![],
			show_log: false,
		};

		let args = std::env::args().collect::<Vec<String>>();

		if let Some(path) = args.get(1) {
			if fs::exists(path) {
				viewer.open(d, path);
			}
		}

		return Ok(viewer);

	}

	fn event(&mut self, d: &mut Ctx, e: &input::Event) -> Result<()> {

		use input::Event::*;

		match e {

			FileDrop(path) => self.open(d, path),

			KeyPress(k) => {

				let mods = d.window.key_mods();

				match *k {
					Key::Esc => d.window.quit(),
					Key::R => self.refresh(d),
					Key::L => self.show_log = !self.show_log,
					Key::C if self.show_log => self.log.clear(),
					Key::Q if mods.meta => d.window.quit(),
					Key::F if mods.meta => d.window.toggle_fullscreen(),
					_ => {},
				}

			},

			_ => {},

		}

		return Ok(());

	}

	fn update(&mut self, d: &mut Ctx) -> Result<()> {

		if let Some(file) = &mut self.file {
			if file.check_modified() {
				self.refresh(d);
			}
		}

		return Ok(());

	}

	fn draw(&mut self, d: &mut Ctx) -> Result<()> {

		if let Some(shader) = &self.shader {
			d.gfx.draw_with(&shader, &GeneralUniform {
				resolution: vec2!(d.gfx.width(), d.gfx.height()) * d.gfx.dpi(),
				mouse: d.window.mouse_pos() / vec2!(d.gfx.width(), d.gfx.height()),
				time: d.app.time(),
			}, |gfx| {
				gfx.draw(
					&shapes::uvrect(
						gfx.coord(Origin::TopLeft),
						gfx.coord(Origin::BottomRight),
					)
				)?;
				return Ok(());
			})?;
		} else {
			d.gfx.draw_t(
				mat4!()
					.t2(d.gfx.coord(Origin::TopLeft) + vec2!(24, -24))
					,
				&shapes::text("drop fragment shader files into this window")
					.size(12.0)
					.align(Origin::TopLeft)
			)?;
		}

		if self.show_log {

			let mut y = 0.0;

			for (i, msg) in self.log
				.iter()
				.rev()
				.enumerate() {

				let to = (i as f32).map(0.0, LOG_SIZE as f32, 1.0, 0.3);
				let bo = (i as f32).map(0.0, LOG_SIZE as f32, 0.7, 0.0);

				let color = match msg.ty {
					MsgType::Success => rgba!(0, 1, 0, to),
					MsgType::Failure => rgba!(1, 0, 0, bo),
				};

				let padding = 8.0;
				let width = d.gfx.width() as f32;

				let text = shapes::text(&msg.content)
					.align(Origin::BottomLeft)
					.color(color)
					.wrap(shapes::TextWrap {
						width: width - padding * 2.0,
						break_type: shapes::TextWrapBreak::Word,
					})
					.format(d.gfx);

				let th = text.height();
				let pos = d.gfx.coord(Origin::BottomLeft) + vec2!(0, y);

				d.gfx.draw(
					&shapes::rect(pos, pos + vec2!(width, th))
						.fill(rgba!(0, 0, 0, bo))
				)?;

				d.gfx.draw_t(
					mat4!()
						.t2(pos),
					&text
				)?;

				y += th;

			}

		}

		return Ok(());

	}

}

fn main() {
	if let Err(err) = launcher()
		.resizable(true)
		.run::<Viewer>() {
		elog!("{}", err);
	}
}

