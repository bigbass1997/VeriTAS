use std::process::Output;
use crate::Error;

mod bizhawk;
pub use bizhawk::*;

mod fceux;
pub use fceux::*;

mod gens;
pub use gens::*;

#[derive(Debug, Clone)]
pub enum EmulatorContext {
    BizHawk(BizHawkContext),
    Fceux(FceuxContext),
    Gens(GensContext),
    
}
impl EmulatorContext {
    pub fn as_bizhawk(&self) -> Option<&BizHawkContext> {
        if let Self::BizHawk(ctx) = self { Some(ctx) } else { None }
    }
    
    pub fn as_fceux(&self) -> Option<&FceuxContext> {
        if let Self::Fceux(ctx) = self { Some(ctx) } else { None }
    }
    
    pub fn as_gens(&self) -> Option<&GensContext> {
        if let Self::Gens(ctx) = self { Some(ctx) } else { None }
    }
    
    pub fn run(&mut self) -> Result<Output, Error> {
        let mut cmd = match self {
            Self::BizHawk(ctx) => ctx.build_command()?,
            Self::Fceux(ctx) => ctx.build_command()?,
            Self::Gens(ctx) => ctx.build_command()?,
        };
        
        cmd.output().map_err(|e| e.into())
    }
}