pub mod rust;

pub trait Compiler {
    fn compile(source: &str);
}
