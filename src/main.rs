// wengwengweng

use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;

use dirty::*;
use dirty::math::*;
use dirty::app::*;
use input::Key;

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
	shader: Option<gfx::Shader2D<GeneralUniform>>,
	log: Vec<Msg>,
	show_log: bool,
}

#[derive(Clone)]
struct GeneralUniform {
	resolution: Vec2,
	mouse: Vec2,
	time: f32,
}

impl gfx::Uniform for GeneralUniform {
	fn values(&self) -> gfx::UniformValues {
		return hmap![
			"resolution" => &self.resolution,
			"mouse" => &self.mouse,
			"time" => &self.time,
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

		self.log.push(msg);
		self.show_log = true;

	}

	fn refresh(&mut self, ctx: &Ctx) {

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

		match gfx::Shader2D::from_frag(ctx, &content) {

			Ok(shader) => {

				let fname = path
					.file_name()
					.unwrap_or(std::ffi::OsStr::new("unknown file"));

				self.shader = Some(shader);
				self.log(Msg::success(&format!("{:?} loaded", fname)));

			},

			Err(msg) => self.log(Msg::failure(&msg)),

		}

	}

}

impl app::State for Viewer {

	fn init(ctx: &mut app::Ctx) -> Result<Self> {
		return Ok(Self {
			file: None,
			shader: None,
			log: vec![],
			show_log: false,
		});
	}

	fn event(&mut self, ctx: &mut app::Ctx, e: &input::Event) -> Result<()> {

		use input::Event::*;

		match e {
			FileDrop(path) => {
				self.file = Some(File::new(path));
				self.refresh(ctx);
			}
			KeyPress(k) => {
				let mods = ctx.key_mods();
				match *k {
					Key::Esc => ctx.quit(),
					Key::R => self.refresh(ctx),
					Key::L => self.show_log = !self.show_log,
					Key::Q if mods.meta => ctx.quit(),
					Key::F if mods.meta => ctx.toggle_fullscreen(),
					_ => {},
				}
			},
			_ => {},
		}

		return Ok(());

	}

	fn update(&mut self, ctx: &mut app::Ctx) -> Result<()> {

		if let Some(file) = &mut self.file {
			if file.check_modified() {
				self.refresh(ctx);
			}
		}

		return Ok(());

	}

	fn draw(&mut self, ctx: &mut app::Ctx) -> Result<()> {

		use gfx::Origin;

		if let Some(shader) = &self.shader {
			ctx.draw_2d_with(&shader, &GeneralUniform {
				resolution: vec2!(ctx.width(), ctx.height()),
				time: ctx.time().into(),
				mouse: ctx.mouse_pos().normalize(),
			}, |ctx| {
				ctx.draw(
					&shapes::rect(
						ctx.coord(Origin::TopLeft),
						ctx.coord(Origin::BottomRight),
					)
				)?;
				return Ok(());
			})?;
		} else {
			ctx.draw_t(
				mat4!()
					.t2(ctx.coord(Origin::TopLeft) + vec2!(24, -24)),
				&shapes::text("drop .frag files into this window")
					.align(gfx::Origin::TopLeft)
			)?;
		}

		if self.show_log {

			let mut y = 0.0;

			for msg in self.log
				.iter()
				.rev() {

				let color = match msg.ty {
					MsgType::Success => rgba!(0, 1, 0, 1),
					MsgType::Failure => rgba!(1, 0, 0, 1),
				};

				let padding = 8.0;
				let width = ctx.width() as f32;

				let text = shapes::text(&msg.content)
					.align(Origin::BottomLeft)
					.color(color)
					.wrap(width - padding * 2.0, false)
					.render(ctx);

				let th = text.height();
				let pos = ctx.coord(Origin::BottomLeft) + vec2!(0, y);

				ctx.draw(
					&shapes::rect(pos, pos + vec2!(width, th))
						.fill(rgba!(0, 0, 0, 0.6))
				)?;

				ctx.draw_t(
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

	if let Err(err) = app::launcher()
		.resizable(true)
		.run::<Viewer>() {
		println!("{}", err);
	}

}

