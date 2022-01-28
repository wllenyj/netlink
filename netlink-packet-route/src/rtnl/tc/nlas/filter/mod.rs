// SPDX-License-Identifier: MIT

use crate::{
    nlas::{self, NlaBuffer},
    traits::ParseableParametrized,
    DecodeError,
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Filter {
    Other(Vec<u8>),
}

impl nlas::Nla for Filter {
    fn value_len(&self) -> usize {
        match self {
            Self::Other(o) => o.len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::Other(o) => buffer.copy_from_slice(o.as_slice()),
        }
    }

    fn kind(&self) -> u16 {
        unreachable!("the parent nla will return TCA_OPTIONS")
    }
}

impl<'a, S> ParseableParametrized<NlaBuffer<&'a [u8]>, S> for Filter
where
    S: AsRef<str>,
{
    fn parse_with_param(buf: &NlaBuffer<&'a [u8]>, _kind: S) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(Self::Other(payload.to_vec()))
    }
}
