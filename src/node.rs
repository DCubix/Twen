use std::f32::consts::PI;

#[derive(PartialEq, Debug)]
pub struct Phase {
	phase: f32,
	phase_step: f32,
	period: f32
}

impl Phase {
	pub fn new(period: f32, sample_rate: u32) -> Phase {
		Phase {
			period,
			phase: 0.0,
			phase_step: (PI * 2.0) / sample_rate as f32
		}
	}

	pub fn advance(&mut self, freq: f32) -> f32 {
		self.phase += self.phase_step * freq;
		self.phase %= self.period;
		self.phase
	}
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Input {
	Value(f32),
	Node(usize),
	Store(usize)
}

impl Input {
	pub fn sample(self, ctx: InputContext) -> f32 {
		match self {
			Input::Value(v) => v,
			Input::Node(id) => ctx.outputs[id],
			Input::Store(id) => ctx.store[id],
			_ => 0.0
		}
	}
}

#[derive(PartialEq, Debug)]
pub enum Node {
	Null,

	Saw(Phase, Input, Input),
	Sine(Phase, Input, Input),
	Square(Phase, Input, Input),
	Triangle(Phase, Input, Input),

	LFO(Phase, Input),
	Map(Input, f32, f32, f32, f32),

	Mix(Input, Input, f32),

	Add(Input, Input),
	Sub(Input, Input),
	Mul(Input, Input),

	Writer(usize, Input),

	Output(Input)
}

#[derive(Clone, Copy)]
struct InputContext<'outs, 'stor> {
	outputs: &'outs Vec<f32>,
	store: &'stor Vec<f32>
}

pub struct NodeGraph {
	nodes: Vec<Node>,
	dead: Vec<usize>,
	output_node: Option<usize>,

	sample_rate: u32,
	outputs: Vec<f32>,
	store: Vec<f32>
}

impl NodeGraph {
	pub fn new(sample_rate: u32) -> NodeGraph {
		NodeGraph {
			nodes: Vec::new(),
			dead: Vec::new(),
			outputs: Vec::new(),
			store: Vec::new(),
			output_node: None,
			sample_rate
		}
	}

	pub fn create_value_store(&mut self) -> usize {
		self.store.push(0.0);
		self.store.len() - 1
	}

	pub fn create_output(&mut self, from: Input) -> usize {
		let id = self.add_node(
			Node::Output(from)
		);
		self.output_node = Some(id);
		id
	}

	pub fn create_sine(&mut self, freq: Input, amp: Input) -> usize {
		self.add_node(
			Node::Sine(Phase::new(PI * 2.0, self.sample_rate), freq, amp)
		)
	}

	pub fn create_square(&mut self, freq: Input, amp: Input) -> usize {
		self.add_node(
			Node::Square(Phase::new(PI * 2.0, self.sample_rate), freq, amp)
		)
	}

	pub fn create_saw(&mut self, freq: Input, amp: Input) -> usize {
		self.add_node(
			Node::Saw(Phase::new(PI * 2.0, self.sample_rate), freq, amp)
		)
	}

	pub fn create_triangle(&mut self, freq: Input, amp: Input) -> usize {
		self.add_node(
			Node::Triangle(Phase::new(PI * 2.0, self.sample_rate), freq, amp)
		)
	}

	pub fn create_lfo(&mut self, freq: Input) -> usize {
		self.add_node(
			Node::LFO(Phase::new(PI * 2.0, self.sample_rate), freq)
		)
	}

	pub fn create_map(&mut self, sample: Input, from_min: f32, from_max: f32, to_min: f32, to_max: f32) -> usize {
		self.add_node(
			Node::Map(sample, from_min, from_max, to_min, to_max)
		)
	}

	pub fn create_add(&mut self, a: Input, b: Input) -> usize {
		self.add_node(
			Node::Add(a, b)
		)
	}

	pub fn create_sub(&mut self, a: Input, b: Input) -> usize {
		self.add_node(
			Node::Sub(a, b)
		)
	}

	pub fn create_mul(&mut self, a: Input, b: Input) -> usize {
		self.add_node(
			Node::Mul(a, b)
		)
	}

	pub fn create_writer(&mut self, id: usize, value: Input) -> usize {
		self.add_node(
			Node::Writer(id, value)
		)
	}

	pub fn create_mix(&mut self, a: Input, b: Input, factor: f32) -> usize {
		self.add_node(
			Node::Mix(a, b, factor)
		)
	}

	pub fn delete_node(&mut self, id: usize) -> Result<(), &str> {
		if self.dead.contains(&id) {
			return Err("Node doesn't exist");
		}
		self.nodes[id] = Node::Null;
		self.dead.push(id);
		Ok(())
	}

	pub fn sample(&mut self) -> f32 {
		for (id, n) in self.nodes.iter_mut().enumerate() {
			let outputs = &self.outputs;
			let store = &self.store;
			let ctx = InputContext {
				outputs, store
			};
			self.outputs[id] = match n {
				Node::Sine(p, freq, amp) => {
					p.advance(freq.sample(ctx)).sin() * amp.sample(ctx)
				},
				Node::Square(p, freq, amp) => {
					(if p.advance(freq.sample(ctx)) > 0.5 { 1.0 } else { -1.0 }) * amp.sample(ctx)
				},
				Node::Saw(p, freq, amp) => {
					(p.advance(freq.sample(ctx)) * 2.0 - 1.0) * amp.sample(ctx)
				},
				Node::Triangle(p, freq, amp) => {
					let a = amp.sample(ctx);
					let ph = p.advance(freq.sample(ctx));
					if ph < PI {
						(-1.0 + (2.0 / PI) * ph) * a
					} else {
						(3.0 - (2.0 / PI) * ph) * a
					}
				},
				Node::Output(input) => input.sample(ctx),
				Node::LFO(p, freq) => p.advance(freq.sample(ctx)).sin() * 0.5 + 0.5,
				Node::Map(sample, from_min, from_max, to_min, to_max) => {
					let s = sample.sample(ctx);
					let norm = (s - *from_min) / (*from_max - *from_min);
					norm * (*to_max - *to_min) + *to_min
				},
				Node::Add(a, b) => a.sample(ctx) + b.sample(ctx),
				Node::Sub(a, b) => a.sample(ctx) - b.sample(ctx),
				Node::Mul(a, b) => a.sample(ctx) * b.sample(ctx),
				Node::Writer(id, value) => {
					let s = value.sample(ctx);
					self.store[*id] = s;
					s
				},
				Node::Mix(a, b, f) => {
					let sa = a.sample(ctx);
					let sb = b.sample(ctx);
					(1.0 - *f) * sa + sb * *f
				},
				_ => 0.0
			};
		}
		if !self.nodes.is_empty() {
			let out_node = self.output_node.unwrap_or(self.nodes.len() - 1);
			self.outputs[out_node]
		} else {
			0.0
		}
	}

	fn add_node(&mut self, n: Node) -> usize {
		match self.dead.is_empty() {
			true => {
				self.nodes.push(n);
				self.outputs.push(0.0);
				self.nodes.len() - 1
			},
			false => {
				let id = self.dead.pop().unwrap();
				self.nodes[id] = n;
				id
			}
		}
	}
}