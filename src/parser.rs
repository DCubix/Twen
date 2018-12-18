use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use std::env;
use std::fs;

use crate::node::{ NodeGraph, Input };

struct Reader {
	data: Vec<char>,
	pos: usize
}

impl Reader {
	pub fn new(input: Vec<char>) -> Reader {
		Reader {
			data: input,
			pos: 0
		}
	}

	pub fn has_next(&self) -> bool {
		self.pos < self.data.len()
	}

	pub fn prev(&self) -> Option<char> {
		if self.pos == 0 {
			None
		} else {
			Some(self.data[self.pos - 1])
		}
	}

	pub fn next(&mut self) -> Option<char> {
		if self.pos >= self.data.len() {
			return None;
		}
		self.pos += 1;
		Some(self.data[self.pos - 1])
	}

	pub fn peek(&self) -> Option<char> {
		if self.pos >= self.data.len() {
			return None;
		}
		Some(self.data[self.pos + 1])
	}

	pub fn step_back(&mut self) {
		self.pos -= 1;
	}

	pub fn current(&self) -> char {
		if self.pos >= self.data.len() {
			return '\0';
		}
		self.data[self.pos]
	}
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TokenType {
	Unknown,
	Identifier,
	Number,
	LParen,
	RParen,
	Equals,
	Comma,
	EOF
}

#[derive(Debug, Clone)]
pub struct Token {
	token_type: TokenType,
	lexeme: String,
	value: f32
}

impl Token {
	pub fn new(token_type: TokenType, lexeme: &str, value: f32) -> Token {
		Token {
			token_type,
			lexeme: lexeme.to_owned(),
			value
		}
	}
}

pub fn lex(input: &str) -> Vec<Token> {
	let mut sr = Reader::new(input.chars().collect());
	let mut tokens = Vec::new();

	while sr.has_next() {
		match sr.current() {
			'a'...'z' | 'A'...'Z' | '_' => { // Identifier
				let mut identifier = String::new();
				while sr.current().is_ascii_alphanumeric() && sr.has_next() {
					identifier.push(sr.current());
					sr.next();
				}
				tokens.push(Token::new(TokenType::Identifier, identifier.as_str(), 0.0))
			},
			'-' | '0'...'9' | '.' => { // Number
				let mut number = String::new();
				while (sr.current().is_digit(10) || sr.current() == '.' || sr.current() == '-') && sr.has_next() {
					number.push(sr.current());
					sr.next();
				}
				let num = match number.parse::<f32>() {
					Ok(n) => n,
					Err(e) => panic!("Invalid number. {}", e)
				};
				tokens.push(Token::new(TokenType::Number, number.as_str(), num));
			},
			'(' => {
				tokens.push(Token::new(TokenType::LParen, "", 0.0));
				sr.next();
			},
			')' => {
				tokens.push(Token::new(TokenType::RParen, "", 0.0));
				sr.next();
			},
			'=' => {
				tokens.push(Token::new(TokenType::Equals, "", 0.0));
				sr.next();
			},
			',' => {
				tokens.push(Token::new(TokenType::Comma, "", 0.0));
				sr.next();
			},
			' ' | '\n' | '\t' | '\r' => { sr.next(); },
			'#' => {
				while sr.current() != '\n' && sr.current() != '\r' && sr.has_next() {
					sr.next();
				}
			},
			_ => {
				tokens.push(Token::new(TokenType::Unknown, "", 0.0));
				sr.next();
			}
		}
	}
	tokens.push(Token::new(TokenType::EOF, "", 0.0));

	// println!("{:#?}", tokens);

	tokens
}

#[derive(Debug, Clone)]
pub enum Expr {
	Literal(f32),
	Identifier(String),
	Assign(Box<Expr>, Box<Expr>),
	Call(String, Vec<Expr>),
	Program(Vec<Expr>)
}

pub struct Parser {
	tokens: Vec<Token>,
	unknown_token: Token,
	pos: usize
}

impl Parser {
	pub fn new(input: &str) -> Parser {
		Parser {
			tokens: lex(input),
			unknown_token: Token::new(TokenType::Unknown, "", 0.0),
			pos: 0
		}
	}

	fn prev(&self) -> &Token {
		let i = (self.pos as i32) - 1;
		if i < 0 {
			&self.unknown_token
		} else {
			&self.tokens[i as usize]
		}
	}

	fn peek(&self) -> &Token {
		&self.tokens[self.pos]
	}

	fn advance(&mut self) {
		if self.pos < self.tokens.len() {
			self.pos += 1;
		}
	}

	fn accept(&mut self, tt: TokenType) -> bool {
		if self.pos >= self.tokens.len() {
			false
		} else if self.peek().token_type == tt {
			self.advance();
			true
		} else {
			false
		}
	}

	fn expect(&mut self, tt: TokenType) -> bool {
		if self.accept(tt) {
			true
		} else {
			panic!("Expected \"{:?}\".", tt);
		}
	}

	fn call(&mut self) -> Box<Expr> {
		let func_name = self.prev().lexeme.clone();
		self.expect(TokenType::LParen);

		let mut args = Vec::new();
		if self.peek().token_type != TokenType::RParen {
			loop {
				args.push(*self.factor());
				if self.peek().token_type == TokenType::RParen {
					self.advance();
					break;
				}
				if !self.accept(TokenType::Comma) {
					break;
				}
			}
		} else {
			self.advance();
		}

		Box::new(Expr::Call(func_name, args))
	}

	fn factor(&mut self) -> Box<Expr> {
		if self.accept(TokenType::Number) {
			Box::new(Expr::Literal(self.prev().value))
		} else if self.accept(TokenType::Identifier) {
			if self.peek().token_type != TokenType::LParen {
				Box::new(Expr::Identifier(self.prev().lexeme.clone()))
			} else {
				self.call()
			}
		} else if self.accept(TokenType::EOF) {
			Box::new(Expr::Literal(0.0))
		} else {
			self.advance();
			panic!("Syntax error: \"{:?}\"", self.prev().token_type);
		}
	}

	fn stmt(&mut self) -> Box<Expr> {
		let var_name = self.factor();
		if self.accept(TokenType::Equals) {
			let val = self.factor();
			Box::new(Expr::Assign(var_name, val))
		} else {
			self.advance();
			var_name
		}
	}

	pub fn parse(&mut self) -> Box<Expr> {
		let mut prog = Vec::new();
		while self.prev().token_type != TokenType::EOF {
			prog.push(*self.stmt());
		}
		// println!("{:#?}", prog);
		Box::new(Expr::Program(prog))
	}
}

#[derive(Debug, Copy, Clone)]
pub enum Value {
	Number(f32),
	NodeID(usize),
	StoreID(usize),
	Nil
}

impl Into<Input> for Value {
	fn into(self) -> Input {
		match self {
			Value::Nil => Input::Value(0.0),
			Value::NodeID(i) => Input::Node(i),
			Value::StoreID(i) => Input::Store(i),
			Value::Number(v) => Input::Value(v)
		}
	}
}

impl Value {
	pub fn get_number(self) -> f32 {
		match self {
			Value::Number(v) => v,
			_ => 0.0
		}
	}
}

pub struct GraphLoader {
	variables: HashMap<String, Value>,
	parser: Parser
}

impl GraphLoader {
	pub fn new(file: &str) -> GraphLoader {
		let s = match fs::read_to_string(file) {
			Ok(s) => s,
			Err(e) => panic!("Error: {}", e)
		};
		GraphLoader {
			parser: Parser::new(s.as_str()),
			variables: HashMap::new()
		}
	}

	fn visit(&mut self, expr: Expr, graph: &mut NodeGraph) -> Value {
		match expr {
			Expr::Literal(v) => Value::Number(v),
			Expr::Identifier(s) => {
				if !self.variables.contains_key(&s) {
					self.variables.insert(s, Value::Nil);
					Value::Nil
				} else {
					self.variables[&s]
				}
			},
			Expr::Assign(a, b) => {
				let _a = match *a {
					Expr::Identifier(nam) => nam.clone(),
					_ => panic!("Invalid variable.")
				};
				let _b = self.visit(*b, graph);
				let val = match self.variables.entry(_a) {
					Occupied(v) => v.into_mut(),
					Vacant(v) => v.insert(Value::Nil)
				};
				*val = _b;

				Value::Nil
			},
			Expr::Call(func, args) => {
				match func.as_str() {
					"CreateStore" => {
						Value::StoreID(graph.create_value_store())
					},
					"LFO" => {
						let freq = self.visit(args[0].clone(), graph).into();
						Value::NodeID(graph.create_lfo(freq))
					},
					"Output" => {
						let from = self.visit(args[0].clone(), graph).into();
						Value::NodeID(graph.create_output(from))
					},
					"Sine" => {
						let freq = self.visit(args[0].clone(), graph).into();
						let amp  = self.visit(args[1].clone(), graph).into();
						Value::NodeID(graph.create_sine(freq, amp))
					},
					"Square" => {
						let freq = self.visit(args[0].clone(), graph).into();
						let amp  = self.visit(args[1].clone(), graph).into();
						Value::NodeID(graph.create_square(freq, amp))
					},
					"Saw" => {
						let freq = self.visit(args[0].clone(), graph).into();
						let amp  = self.visit(args[1].clone(), graph).into();
						Value::NodeID(graph.create_saw(freq, amp))
					},
					"Triangle" => {
						let freq = self.visit(args[0].clone(), graph).into();
						let amp  = self.visit(args[1].clone(), graph).into();
						Value::NodeID(graph.create_triangle(freq, amp))
					},
					"Map" => {
						let input = self.visit(args[0].clone(), graph).into();
						let a  = self.visit(args[1].clone(), graph).get_number();
						let b  = self.visit(args[2].clone(), graph).get_number();
						let c  = self.visit(args[3].clone(), graph).get_number();
						let d  = self.visit(args[4].clone(), graph).get_number();
						Value::NodeID(graph.create_map(input, a, b, c, d))
					},
					"Add" => {
						let a = self.visit(args[0].clone(), graph).into();
						let b = self.visit(args[1].clone(), graph).into();
						Value::NodeID(graph.create_add(a, b))
					},
					"Sub" => {
						let a = self.visit(args[0].clone(), graph).into();
						let b = self.visit(args[1].clone(), graph).into();
						Value::NodeID(graph.create_sub(a, b))
					},
					"Mul" => {
						let a = self.visit(args[0].clone(), graph).into();
						let b = self.visit(args[1].clone(), graph).into();
						Value::NodeID(graph.create_mul(a, b))
					},
					"Writer" => {
						let a = match self.visit(args[0].clone(), graph).into() {
							Value::StoreID(id) => id,
							_ => panic!("Invalid Store ID.")
						};
						let input = self.visit(args[1].clone(), graph).into();
						Value::NodeID(graph.create_writer(a, input))
					},
					"Mix" => {
						let a = self.visit(args[0].clone(), graph).into();
						let b = self.visit(args[1].clone(), graph).into();
						let fac = self.visit(args[2].clone(), graph).get_number();
						Value::NodeID(graph.create_mix(a, b, fac))
					},
					_ => panic!("Invalid function: \"{}\"", func)
				}
			},
			Expr::Program(exprs) => {
				for expr in exprs.into_iter() {
					self.visit(expr, graph);
				}
				Value::Nil
			},
			_ => Value::Nil
		}
	}

	pub fn load(&mut self) -> NodeGraph {
		let prog = self.parser.parse();
		let mut graph = NodeGraph::new(44100);
		self.visit(*prog, &mut graph);
		graph
	}
}