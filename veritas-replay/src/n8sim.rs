use std::marker::PhantomData;
use std::time::Duration;
use camino::Utf8Path;
use crate::n8sim::fs::{FsMapper, MAX_PER_PAGE};

pub mod fs;
pub mod sorted;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Input(pub u8);
impl From<u8> for Input {
    fn from(value: u8) -> Self {
        Self(value)
    }
}
impl From<Input> for u8 {
    fn from(value: Input) -> Self {
        value.0
    }
}
impl Input {
    const A: u8 = 0b10000000;
    const B: u8 = 0b01000000;
    const S: u8 = 0b00100000;
    const T: u8 = 0b00010000;
    const U: u8 = 0b00001000;
    const D: u8 = 0b00000100;
    const L: u8 = 0b00000010;
    const R: u8 = 0b00000001;
    
    pub fn new() -> Self {
        Self(0xFF)
    }
    
    pub fn and_a(self) -> Self {
        Self(self.0 & !Self::A)
    }
    pub fn and_b(self) -> Self {
        Self(self.0 & !Self::B)
    }
    pub fn and_select(self) -> Self {
        Self(self.0 & !Self::S)
    }
    pub fn and_start(self) -> Self {
        Self(self.0 & !Self::T)
    }
    pub fn and_up(self) -> Self {
        Self(self.0 & !Self::U)
    }
    pub fn and_down(self) -> Self {
        Self(self.0 & !Self::D)
    }
    pub fn and_left(self) -> Self {
        Self(self.0 & !Self::L)
    }
    pub fn and_right(self) -> Self {
        Self(self.0 & !Self::R)
    }
}



/// A simulation of the N8 flashcart's menu.
/// 
/// The simulator follows a builder-like pattern, taking a reference to a [`FsMapper`] and procedurally
/// building a sequence of inputs that navigate the flashcart's menu.
/// 
/// # Caution
/// The simulator always assumes the cursor starts at the top-most file entry in the root menu directory.
/// 
/// There is no way to acquire feedback from the menu to know where the cursor is at.
pub struct N8Simulator<'a> {
    fs_mapper: &'a FsMapper,
    func: Box<dyn FnMut(&mut dyn Iterator<Item = Input>) + 'a>,
    
    inputs: Vec<Input>,
}
impl<'a> N8Simulator<'a> {
    /// Creates a new simulator.
    pub fn new<F: FnMut(&mut dyn Iterator<Item = Input>) + 'a>(fs_mapper: &'a FsMapper, func: F) -> Self {
        Self {
            fs_mapper,
            func: Box::new(func),
            
            inputs: Vec::with_capacity(64),
        }
    }
    
    pub fn flush(mut self, ms: u64) -> Self {
        (self.func)(&mut self.inputs.into_iter());
        
        self.inputs = vec![];
        
        if ms > 0 {
            self = self.delay(ms);
        }
        
        self
    }
    
    pub fn delay(self, ms: u64) -> Self {
        std::thread::sleep(Duration::from_millis(ms));
        
        self
    }
    
    pub fn to_path(mut self, path: impl AsRef<Utf8Path>) -> Self {
        if let Some(locations) = self.fs_mapper.root().location(path) {
            for i in &locations[..(locations.len() - 1)] {
                self = self.to_i(*i).b(1).flush(1800);
            }
            
            if let Some(i) = locations.last() {
                self = self.to_i(*i).flush(0);
            }
        }
        
        self
    }
    
    pub fn to_i(mut self, i: usize) -> Self {
        let page = i / MAX_PER_PAGE;
        let i = i % MAX_PER_PAGE;
        
        for _ in 0..page {
            self = self.right(1).flush(500);
        }
        
        self.down(i)
    }
    
    pub fn a(mut self, n: usize) -> Self {
        for _ in 0..n {
            self.inputs.push(Input::new().and_a());
            self.inputs.push(Input::new());
        }
        
        self
    }
    pub fn b(mut self, n: usize) -> Self {
        for _ in 0..n {
            self.inputs.push(Input::new().and_b());
            self.inputs.push(Input::new());
        }
        
        self
    }
    pub fn select(mut self, n: usize) -> Self {
        for _ in 0..n {
            self.inputs.push(Input::new().and_select());
            self.inputs.push(Input::new());
        }
        
        self
    }
    pub fn start(mut self, n: usize) -> Self {
        for _ in 0..n {
            self.inputs.push(Input::new().and_start());
            self.inputs.push(Input::new());
        }
        
        self
    }
    pub fn up(mut self, n: usize) -> Self {
        for _ in 0..n {
            self.inputs.push(Input::new().and_up());
            self.inputs.push(Input::new());
        }
        
        self
    }
    pub fn down(mut self, n: usize) -> Self {
        for _ in 0..n {
            self.inputs.push(Input::new().and_down());
            self.inputs.push(Input::new());
        }
        
        self
    }
    pub fn left(mut self, n: usize) -> Self {
        for _ in 0..n {
            self.inputs.push(Input::new().and_left());
            self.inputs.push(Input::new());
        }
        
        self
    }
    pub fn right(mut self, n: usize) -> Self {
        for _ in 0..n {
            self.inputs.push(Input::new().and_right());
            self.inputs.push(Input::new());
        }
        
        self
    }
    
    pub fn output(self) -> impl Iterator<Item = Input> {
        self.inputs.into_iter()
    }
    
    pub fn output_raw(self) -> impl Iterator<Item = u8> {
        self.inputs.into_iter().map(|i| i.0)
    }
}