use crate::parser::MovieFormat;

#[derive(Debug, Clone, PartialEq)]
pub struct Gmv {
    
}
impl From<Gmv> for MovieFormat {
    fn from(value: Gmv) -> Self {
        Self::Gmv(value)
    }
}
impl Gmv {
    pub fn parse(_data: &[u8]) -> Option<Self> {
        todo!()
    }
}