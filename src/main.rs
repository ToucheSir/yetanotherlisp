use std::io;
use std::io::prelude::*;

type CellIndex = usize;
type SymbolIndex = usize;

const NIL_INDEX: CellIndex = 0;

#[derive(Copy, Clone, Debug)]
enum CellType {
    Number(i32),
    Symbol(SymbolIndex),
    Cons(CellIndex),
    Free,
}

#[derive(Copy, Clone, Debug)]
struct Cell {
    val: CellType,
    tail: CellIndex,
}
impl Cell {
    fn empty() -> Cell {
        Cell {
            val: CellType::Free,
            tail: NIL_INDEX,
        }
    }
    fn new(val: CellType, tail: CellIndex) -> Self {
        Cell {
            val: val,
            tail: tail,
        }
    }
}

#[derive(Debug)]
struct CellStorage<'a> {
    free_index: CellIndex,
    cells: &'a mut [Cell],
}
impl<'a> CellStorage<'a> {
    fn new(buf: &mut [Cell]) -> CellStorage {
        CellStorage {
            cells: buf,
            free_index: NIL_INDEX + 1,
        }
    }
    fn alloc_cell(&mut self, val: CellType) -> CellIndex {
        if self.free_index == NIL_INDEX {
            panic!("Exhausted cell storage!");
        } else {
            let idx = self.free_index;
            self.free_index = self.cells[idx].tail;
            self.cells[idx].val = val;
            self.cells[idx].tail = NIL_INDEX;
            idx
        }
    }
    fn free_cell(&mut self, idx: CellIndex) {
        match self.cells[idx].val {
            CellType::Number(_) | CellType::Symbol(_) => {
                self.cells[idx] = Cell::new(CellType::Free, self.free_index);
                self.free_index = idx;
            }
            CellType::Cons(head) => {
                self.free_cell(head);
                let tail = self.cells[idx].tail;
                self.free_cell(tail);
                self.cells[idx] = Cell::new(CellType::Free, self.free_index);
                self.free_index = idx;
            }
            _ => {}
        }
    }

    fn get(&self, idx: CellIndex) -> Cell {
        self.cells[idx]
    }
    fn set_tail(&mut self, idx: CellIndex, tail: CellIndex) {
        self.cells[idx].tail = tail;
    }
    fn val_of(&self, idx: CellIndex) -> CellType {
        self.cells[idx].val
    }
    fn tail_of(&self, idx: CellIndex) -> CellIndex {
        self.cells[idx].tail
    }
}

macro_rules! car {
    ($exp: expr, $storage: expr) => {
        if let CellType::Cons(head) = $storage.get($exp).val {
            head
        } else {
            panic!("Not a cons cell")
        }
    }
}

macro_rules! cdr {
    ($exp: expr, $storage: expr) => {
        if let CellType::Cons(_) = $storage.val_of($exp) {
            $storage.tail_of($exp)
        } else {
            NIL_INDEX
        }
    }
}

fn is_cons(exp: CellIndex, cells: &CellStorage) -> bool {
    match cells.val_of(exp) {
        CellType::Cons(_) => true,
        _ => false,
    }
}

fn is_unary(exp: CellIndex, cells: &CellStorage) -> bool {
    is_cons(cdr!(exp, cells), cells) && cdr!(cdr!(exp, cells), cells) == NIL_INDEX
}

fn is_binary(exp: CellIndex, cells: &CellStorage) -> bool {
    is_cons(cdr!(exp, cells), cells) && is_cons(cdr!(cdr!(exp, cells), cells), cells) &&
    (cdr!(cdr!(cdr!(exp, cells), cells), cells) == NIL_INDEX)
}

fn print_exp(idx: CellIndex, storage: &CellStorage, env: &Env) {
    if idx == NIL_INDEX {
        print!("()");
    } else {
        match storage.get(idx).val {
            CellType::Symbol(sym) => {
                print!("{}", env.symbols[sym]);
            }
            CellType::Number(n) => {
                print!("{}", n);
            }
            CellType::Cons(_) => print_list(idx, storage, env),
            _ => {}
        }
    }
}

fn print_list(idx: CellIndex, storage: &CellStorage, env: &Env) {
    print!("(");
    let mut exp = idx;
    let mut cell = storage.get(exp);

    print_exp(car!(exp, storage), storage, env);
    exp = cdr!(exp, storage);
    cell = storage.get(exp);

    while let CellType::Cons(head) = cell.val {
        print!(" ");
        print_exp(head, storage, env);
        exp = cell.tail;
        cell = storage.get(exp);
    }

    if exp != NIL_INDEX {
        print!(" . ");
        print_exp(exp, storage, env);
    }
    print!(")");
}

struct Env {
    symbols: Vec<String>,
}
impl Env {
    fn new() -> Env {
        Env { symbols: Vec::new() }
    }

    fn add_sym(&mut self, name: String) -> SymbolIndex {
        match self.symbols.iter().position(|s| &name == s) {
            Some(idx) => idx,
            None => {
                self.symbols.push(name);
                self.symbols.len() - 1
            }
        }
    }
}

struct DefaultNS {
    add: SymbolIndex,
    sub: SymbolIndex,
    mul: SymbolIndex,
    div: SymbolIndex,
    modu: SymbolIndex,
    cons: SymbolIndex,
    hd: SymbolIndex,
    tl: SymbolIndex,
    quote: SymbolIndex,
}
impl DefaultNS {
    fn new(env: &mut Env) -> Self {
        DefaultNS {
            add: env.add_sym("add".to_string()),
            sub: env.add_sym("sub".to_string()),
            mul: env.add_sym("mul".to_string()),
            div: env.add_sym("div".to_string()),
            modu: env.add_sym("mod".to_string()),
            cons: env.add_sym("cons".to_string()),
            hd: env.add_sym("hd".to_string()),
            tl: env.add_sym("tl".to_string()),
            quote: env.add_sym("'".to_string()),
        }
    }
}

#[derive(Debug, PartialEq)]
enum Token {
    LeftParen,
    RightParen,
    Dot,
    Number(String),
    Symbol(String),
}
struct TokenStream<'a> {
    input: &'a Vec<u8>,
    pos: usize,
}
impl<'a> TokenStream<'a> {
    fn new(input: &'a Vec<u8>) -> Self {
        TokenStream {
            input: input,
            pos: 0,
        }
    }

    fn eol(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn peek_ch(&self) -> char {
        self.input[self.pos] as char
    }

    fn next_ch(&mut self) -> char {
        let ch = self.input[self.pos];
        self.pos += 1;
        return ch as char;
    }

    fn peek_token(&mut self) -> Option<Token> {
        let old_pos = self.pos;
        let tok = self.next_token();
        self.pos = old_pos;
        tok
    }

    fn next_token(&mut self) -> Option<Token> {
        self.consume_whitespace();
        if self.eol() {
            None
        } else {
            let ch = self.peek_ch();
            Some(match ch {
                '(' => {
                    self.next_ch();
                    Token::LeftParen
                }
                ')' => {
                    self.next_ch();
                    Token::RightParen
                }
                '.' => {
                    self.next_ch();
                    Token::Dot
                }
                '\'' => {
                    self.next_ch();
                    // Token::Quote
                    Token::Symbol(String::from("'"))
                }
                _ => {
                    if ch.is_digit(10) {
                        Token::Number(self.consume_while(|c| c.is_digit(10)))
                    } else if ch.is_alphanumeric() {
                        Token::Symbol(self.consume_while(char::is_alphanumeric))
                    } else {
                        panic!("Syntax error: at '{}'!", ch);
                    }
                }
            })
        }
    }

    fn consume_while<F>(&mut self, test: F) -> String
        where F: Fn(char) -> bool
    {
        let mut res = String::new();

        while !self.eol() && test(self.peek_ch()) {
            res.push(self.next_ch());
        }

        return res;
    }

    fn consume_whitespace(&mut self) {
        self.consume_while(char::is_whitespace);
    }
}


struct Parser<'a> {
    env: &'a mut Env,
    nesting: usize,
    exp: CellIndex,
    exps: CellIndex,
    car: CellIndex,
    cdr: CellIndex,
}
impl<'a> Parser<'a> {
    fn new(env: &'a mut Env) -> Self {
        Parser {
            env: env,
            nesting: 0,
            exp: NIL_INDEX,
            exps: NIL_INDEX,
            car: NIL_INDEX,
            cdr: NIL_INDEX,
        }
    }
    fn parse(&mut self, input: &Vec<u8>, storage: &mut CellStorage) -> Option<CellIndex> {
        let mut tokens = TokenStream::new(input);
        self.parse_sexp(&mut tokens, storage)
    }
    fn parse_sexp(&mut self,
                  tokens: &mut TokenStream,
                  storage: &mut CellStorage)
                  -> Option<CellIndex> {
        match tokens.next_token() {
            Some(Token::Number(str_num)) => {
                let nval = str_num.parse::<i32>().unwrap();
                self.exp = storage.alloc_cell(CellType::Number(nval));
                Some(self.exp)
            }
            Some(Token::Symbol(name)) => {
                self.exp = storage.alloc_cell(CellType::Symbol(self.env.add_sym(name)));
                Some(self.exp)
            }
            Some(tok) => {
                assert_eq!(tok, Token::LeftParen);
                self.nesting += 1;
                match self.parse_sexps(tokens, storage) {
                    Some(exps) => {
                        self.nesting -= 1;
                        match tokens.next_token() {
                            Some(Token::RightParen) => {
                                self.exp = exps;
                                Some(self.exp)
                            }
                            _ => None,
                        }
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
    fn parse_sexps(&mut self,
                   tokens: &mut TokenStream,
                   storage: &mut CellStorage)
                   -> Option<CellIndex> {
        match tokens.peek_token() {
            Some(Token::RightParen) => Some(NIL_INDEX),
            _ => {
                match self.parse_sexp(tokens, storage) {
                    Some(car) => {
                        match (if let Some(Token::Dot) = tokens.peek_token() {
                            tokens.next_token();
                            self.parse_sexp(tokens, storage)
                        } else {
                            self.parse_sexps(tokens, storage)
                        }) {
                            Some(cdr) => {
                                let idx = storage.alloc_cell(CellType::Cons(car));
                                storage.set_tail(idx, cdr);
                                self.exps = idx;
                                Some(idx)
                            }
                            _ => None
                        }
                    },
                    _ => None
                }
            }
        }
    }
}

fn s_exp(input: &mut io::StdinLock,
         output: &mut io::Stdout,
         mut buf: &mut Vec<u8>,
         storage: &mut CellStorage,
         env: &mut Env)
         -> CellIndex {
    let mut parser = Parser::new(env);
    loop {
        print!("[{}] ", parser.nesting);
        output.flush().unwrap();
        if let Ok(n) = input.read_until(b'\n', &mut buf) {
            // Check for EOF
            if n == 0 {
                return 9000;
            }
        }
        match parser.parse(buf, storage) {
            Some(idx) => return idx,
            _ => println!("partial: {:?}", String::from_utf8_lossy(&*buf)),
        }
    }
}

#[derive(Debug)]
enum EvalError {
    IllegalOperator,
    NonUnary,
    NotCons(CellIndex),
    NonBinary,
    NonNumeric,
    UnknownOperator(SymbolIndex),
}

fn is_atom(exp: CellIndex, cells: &CellStorage) -> bool {
    match cells.val_of(exp) {
        CellType::Number(_) | CellType::Symbol(_) => true,
        _ => exp == NIL_INDEX,
    }
}

fn split_binary(exp: CellIndex, cells: &CellStorage) -> (CellIndex, CellIndex) {
    let head = car!(cdr!(exp, cells), cells);
    let tail = car!(cdr!(cdr!(exp, cells), cells), cells);
    (head, tail)
}

fn eval_cons(exp: CellIndex,
             cells: &mut CellStorage,
             env: &mut Env,
             ns: &DefaultNS)
             -> Result<CellIndex, EvalError> {
    if !is_binary(exp, cells) {
        Err(EvalError::NonBinary)
    } else {
        let (head, tail) = split_binary(exp, cells);
        let head = try!(eval(head, cells, env, ns));
        let tail = try!(eval(tail, cells, env, ns));
        let cons_cell = cells.alloc_cell(CellType::Cons(head));
        cells.set_tail(cons_cell, tail);
        Ok(cons_cell)
    }
}

fn eval_arithmetic(op: SymbolIndex,
                   exp: CellIndex,
                   cells: &mut CellStorage,
                   env: &mut Env,
                   ns: &DefaultNS)
                   -> Result<CellIndex, EvalError> {
    if !is_binary(exp, cells) {
        Err(EvalError::NonBinary)
    } else {
        let (head, tail) = split_binary(exp, cells);
        let lhs = try!(eval(head, cells, env, ns));
        let rhs = try!(eval(tail, cells, env, ns));
        match (cells.val_of(lhs), cells.val_of(rhs)) {
            (CellType::Number(a), CellType::Number(b)) => {
                Ok(cells.alloc_cell(CellType::Number(if op == ns.add {
                    a + b
                } else if op == ns.sub {
                    a - b
                } else if op == ns.mul {
                    a * b
                } else if op == ns.div {
                    a / b
                } else {
                    // if op == ns.modu
                    a % b
                })))
            }
            _ => Err(EvalError::NonNumeric),
        }
    }
}

fn eval(exp: CellIndex,
        cells: &mut CellStorage,
        env: &mut Env,
        ns: &DefaultNS)
        -> Result<CellIndex, EvalError> {
    let cell = cells.get(exp);
    if is_atom(exp, cells) {
        Ok(exp)
    } else if let CellType::Cons(head) = cell.val {
        if let CellType::Symbol(op) = cells.val_of(head) {
            if op == ns.quote {
                if !is_unary(exp, cells) {
                    Err(EvalError::NonUnary)
                } else {
                    Ok(car!(cdr!(exp, cells), cells))
                }
            } else if op == ns.hd || op == ns.tl {
                if !is_unary(exp, cells) {
                    Err(EvalError::NonUnary)
                } else {
                    let res = try!(eval(car!(cdr!(exp, cells), cells), cells, env, ns));
                    if !is_cons(res, cells) {
                        Err(EvalError::NotCons(exp))
                    } else if op == ns.hd {
                        Ok(car!(res, cells))
                    } else {
                        Ok(cdr!(res, cells))
                    }
                }
            } else if op == ns.cons {
                eval_cons(exp, cells, env, ns)
            } else if op == ns.add || op == ns.sub || op == ns.mul || op == ns.div || op == ns.modu {
                eval_arithmetic(op, exp, cells, env, ns)
            } else {
                Err(EvalError::UnknownOperator(op))
            }
        } else {
            Err(EvalError::IllegalOperator)
        }
    } else {
        panic!("Invalid expression")
    }
}

fn display_err(err_type: EvalError, cells: &CellStorage, env: &Env) {
    print!("\nError: ");
    match err_type {
        EvalError::IllegalOperator => println!("illegal operator!"),
        EvalError::NonUnary => println!("non unary expression!"),
        EvalError::NotCons(exp) => {
            print_exp(exp, cells, env);
            println!(" does not evaluate to a cons pair!");
        }
        EvalError::NonBinary => println!("non binary expression!"),
        EvalError::NonNumeric => println!("non unary expression!"),
        EvalError::UnknownOperator(op) => println!("unknown operator '{}'", env.symbols[op]),
    }
}

fn init_storage(buf: &mut [Cell]) -> CellStorage {
    for idx in 1..buf.len() - 1 {
        buf[idx].tail = idx + 1;
    }
    CellStorage::new(buf)
}

fn main() {
    let mut cells = [Cell::empty(); 64];
    let mut storage = init_storage(&mut cells);
    let mut env = Env::new();
    let ns = DefaultNS::new(&mut env);

    println!("An S-expression Evaluator.");
    let stdin = io::stdin();
    let mut buf = Vec::with_capacity(64);
    let mut input = stdin.lock();
    let mut output = io::stdout();

    loop {
        let idx = s_exp(&mut input, &mut output, &mut buf, &mut storage, &mut env);

        print_exp(idx, &mut storage, &env);
        match eval(idx, &mut storage, &mut env, &ns) {
            Ok(exp) => {
                print!(" ==> ");
                print_exp(exp, &storage, &env);
                storage.free_cell(exp);
                println!("");
            }
            Err(err_type) => {
                display_err(err_type, &storage, &env);
            }
        }

        storage.free_cell(idx);
        // println!("{:?}", &storage);
        buf.clear();
    }

    println!("\nEnd.");
}
