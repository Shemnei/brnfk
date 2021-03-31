use ::std::fmt;
use std::{convert::TryFrom, io::Write, process::exit};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    IncPtr,
    DecPtr,
    Inc,
    Dec,
    Output,
    Input,
    JmpStart(usize),
    JmpEnd(usize),
}

impl TryFrom<&u8> for Command {
    type Error = Error;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        let cmd = match value {
            b'>' => Self::IncPtr,
            b'<' => Self::DecPtr,
            b'+' => Self::Inc,
            b'-' => Self::Dec,
            b'.' => Self::Output,
            b',' => Self::Input,
            b'[' => Self::JmpStart(0),
            b']' => Self::JmpEnd(0),
            a => {
                return Err(Error::InvalidCommand {
                    index: 0,
                    command: *a,
                })
            }
        };

        Ok(cmd)
    }
}

#[derive(Default, Debug, Clone)]
pub struct Tape {
    inner: Vec<u8>,
}

impl Tape {
    pub fn inc(&mut self, index: usize) {
        if index >= self.inner.len() {
            self.inner.resize(index + 1, 0);
        }
        self.inner[index] = self.inner[index].wrapping_add(1);
    }

    pub fn dec(&mut self, index: usize) {
        if index >= self.inner.len() {
            self.inner.resize(index + 1, 0);
        }
        self.inner[index] = self.inner[index].wrapping_sub(1);
    }

    pub fn set(&mut self, index: usize, value: u8) {
        if index >= self.inner.len() {
            self.inner.resize(index + 1, 0);
        }
        self.inner[index] = value;
    }

    pub fn get(&self, index: usize) -> u8 {
        *self.inner.get(index).unwrap_or(&0)
    }
}

#[derive(Debug, Clone)]
pub struct Program {
    commands: Vec<Command>,
}

#[derive(Debug, Clone)]
pub enum Error {
    InvalidCommand { index: usize, command: u8 },
    UnmatchedJump { index: usize },
}

impl Error {
    pub(crate) fn set_index(&mut self, idx: usize) {
        match self {
            Self::InvalidCommand { index, .. } => {
                *index = idx;
            }
            Self::UnmatchedJump { index, .. } => {
                *index = idx;
            }
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCommand { index, command } => write!(
                f,
                "Found invalid command `{}` (code: {}) at {}",
                *command as char, command, index
            ),
            Self::UnmatchedJump { index } => {
                write!(f, "No matching jump found for jump at {}", index)
            }
        }
    }
}

impl ::std::error::Error for Error {}

impl Program {
    pub fn load(data: impl AsRef<[u8]>) -> Result<Self, Error> {
        let data = data.as_ref();

        let mut jump_stack: Vec<usize> = Vec::new();
        let mut commands: Vec<Command> = Vec::with_capacity(data.len());

        for (b_loc, b) in data.iter().enumerate() {
            // make relaxed version
            if b.is_ascii_whitespace() {
                continue;
            }

            let mut command = Command::try_from(b).map_err(|mut err| {
                err.set_index(b_loc);
                err
            })?;

            match &mut command {
                Command::JmpStart(_) => {
                    jump_stack.push(commands.len());
                }
                Command::JmpEnd(index) => {
                    let idx = commands.len();

                    let matching = jump_stack
                        .pop()
                        .ok_or(Error::UnmatchedJump { index: idx })?;

                    *index = matching;

                    if let Some(Command::JmpStart(index)) = commands.get_mut(matching) {
                        *index = idx;
                    } else {
                        unreachable!("command vec is broken");
                    }
                }
                _ => {}
            }

            commands.push(command);
        }

        if !jump_stack.is_empty() {
            // Safety: Checked the len in if.
            let unmatched = jump_stack.pop().unwrap();

            Err(Error::UnmatchedJump { index: unmatched })
        } else {
            Ok(Self { commands })
        }
    }
}

pub trait Input: Iterator<Item = u8> {}
pub struct StdinInput;
impl Iterator for StdinInput {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        let mut line = String::new();
        ::std::io::stdin().read_line(&mut line).ok();
        Some(line.bytes().next()?)
    }
}
impl Input for StdinInput {}

pub trait Output {
    fn write(&mut self, value: u8);
}

pub struct StdoutOutput;
impl Output for StdoutOutput {
    fn write(&mut self, value: u8) {
        let stdout = ::std::io::stdout();
        let mut stdout = stdout.lock();
        // Write and flush on stdout should never fail.
        let _ = stdout.write(&[value]).expect("stdout write failed");
        let _ = stdout.flush().expect("stdout flush failed");
    }
}

pub struct Brainfuck<I, O> {
    input: I,
    output: O,
}

impl Default for Brainfuck<StdinInput, StdoutOutput> {
    fn default() -> Self {
        Self {
            input: StdinInput,
            output: StdoutOutput,
        }
    }
}

impl<I, O> Brainfuck<I, O>
where
    I: Input,
    O: Output,
{
    pub fn run(&mut self, program: &Program) {
        let commands = &program.commands;
        let mut tape = Tape::default();
        let mut d_ptr: usize = 0;
        let mut i_ptr: usize = 0;

        while i_ptr < commands.len() {
            let command = commands[i_ptr];

            match command {
                Command::IncPtr => {
                    d_ptr += 1;
                }
                Command::DecPtr => {
                    d_ptr -= 1;
                }
                Command::Inc => {
                    tape.inc(d_ptr);
                }
                Command::Dec => {
                    tape.dec(d_ptr);
                }
                Command::Output => {
                    self.output.write(tape.get(d_ptr));
                }
                Command::Input => {
                    tape.set(d_ptr, self.input.next().expect("failed to get input"));
                }
                Command::JmpStart(matching) => {
                    if tape.get(d_ptr) == 0 {
                        i_ptr = matching;
                        continue;
                    }
                }
                Command::JmpEnd(matching) => {
                    if tape.get(d_ptr) != 0 {
                        i_ptr = matching;
                        continue;
                    }
                }
            }

            i_ptr += 1;
        }
    }

    pub fn output(&self) -> &O {
        &self.output
    }

    pub fn output_mut(&mut self) -> &mut O {
        &mut self.output
    }

    pub fn input(&self) -> &I {
        &self.input
    }

    pub fn input_mut(&mut self) -> &mut I {
        &mut self.input
    }

    pub fn into_inner(self) -> (I, O) {
        (self.input, self.output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default, Debug)]
    struct StringOutput {
        inner: String,
    }

    impl Output for StringOutput {
        fn write(&mut self, value: u8) {
            self.inner.push(value as char);
        }
    }

    type TestBrainfuck = Brainfuck<StdinInput, StringOutput>;

    impl Default for TestBrainfuck {
        fn default() -> Self {
            Self {
                input: StdinInput,
                output: StringOutput::default(),
            }
        }
    }

    #[cfg(target_arch = "x86_64")]
    fn rdtsc() -> u64 {
        unsafe { core::arch::x86_64::_rdtsc() }
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn hello_world_speed() {
        let program = b"++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.";
        let program = Program::load(program).unwrap();

        let mut brnfk: Brainfuck<StdinInput, StringOutput> = Brainfuck::default();

        let start = rdtsc();
        brnfk.run(&program);
        let end = rdtsc();

        let cycles = end - start;
        let cycles_per_command = cycles / program.commands.len() as u64;
        println!("{}", cycles_per_command);
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn hello_world() {
        let program = b"++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.";
        let program = Program::load(program).unwrap();

        let mut brnfk: Brainfuck<StdinInput, StringOutput> = Brainfuck::default();

        brnfk.run(&program);

        let (_, out) = brnfk.into_inner();

        assert_eq!(out.inner, "Hello World!\n");
    }
}

/// Help text for cli usage.
const HELP_TEXT: &'static str = "brnfk - A brainfuck interpreter written in rust.
USAGE: brnfk [INPUT_FILE]";

fn main() {
    let args = ::std::env::args_os();

    let input_file = args.skip(1).next().unwrap_or_else(|| {
        eprintln!("{}", HELP_TEXT);
        exit(1);
    });

    let data = match ::std::fs::read(&input_file) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Failed to read input_file at {:?}: {}", input_file, err);
            exit(1);
        }
    };

    let program = match Program::load(data) {
        Ok(program) => program,
        Err(err) => {
            eprintln!("Failed load program: {:?}", err);
            exit(1);
        }
    };

    let mut brnfk = Brainfuck::<StdinInput, StdoutOutput>::default();
    brnfk.run(&program);
}
