use iced_x86::*;
use std::collections::HashMap;

use crate::{Address, DebugError};

pub struct Disassembler {
    decoder_options: u32,
    bitness: u32,
    formatter: NasmFormatter,
}

impl Disassembler {
    pub fn new() -> Self {
        Self {
            decoder_options: DecoderOptions::NONE,
            bitness: 64,
            formatter: NasmFormatter::new(),
        }
    }
 
    
    pub fn decode_one(&self, bytes: &[u8], address: Address) -> Result<Instruction, DebugError> {
        let mut decoder = Decoder::with_ip(self.bitness, bytes, address.0 as u64, self.decoder_options);
        let instruction = decoder.decode();

        if instruction.is_invalid() {
            let error = decoder.last_error();
            return Err(DebugError::InvalidInstruction(error));
        }

        Ok(instruction)
    }
    
}
