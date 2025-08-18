
//TODO: Move this module into a stand-alone crate for TAS movie parsers

mod bk2;
pub use bk2::Bk2;

mod fm2;
pub use fm2::Fm2;

mod gmv;
pub use gmv::Gmv;

#[derive(Debug, Clone, PartialEq)]
pub enum MovieFormat {
    Bk2(Bk2),
    Fm2(Fm2),
    Gmv(Gmv),
}
impl MovieFormat {
    pub fn parse(data: &[u8], ext: &str) -> Option<Self> {
        match &data[0..16] {
            // BizHawk BK2 (ZIP)
            [b'P', b'K', 0x03, 0x04, ..] => {
                Some(Bk2::parse(data)?.into())
            },
            
            // Gens GMV (binary)
            [b'G', b'e', b'n', b's', b' ', b'M', b'o', b'v', b'i', b'e', ..] => {
                Some(Gmv::parse(data)?.into())
            },
            
            // FCEUX FM2 (plaintext)
            _ if ext == "fm2" => {
                Some(Fm2::parse(data)?.into())
            }
            
            _ => None
        }
    }
}




