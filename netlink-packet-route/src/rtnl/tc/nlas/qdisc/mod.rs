// SPDX-License-Identifier: MIT

mod ingress;
pub use self::ingress::*;

use crate::{
    nlas::{self, NlaBuffer},
    traits::{ParseableParametrized},
    DecodeError,
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Qdisc {
    Ingress(Ingress),
    Other(Vec<u8>),
}

impl Qdisc {
    pub fn new<S: AsRef<str>>(kind: S) -> Self {
        match kind.as_ref() {
            Ingress::KIND => Qdisc::Ingress(Ingress::default()),
            _ => unimplemented!("{} is unimplemented", kind.as_ref()),
        }
    }
}

impl nlas::Nla for Qdisc {
    fn value_len(&self) -> usize {
        match self {
            Self::Ingress(_ingress) => 0,
            Self::Other(o) => o.len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::Ingress(_ingress) => {},
            Self::Other(o) => buffer.copy_from_slice(o.as_slice()),
        }
    }

    fn kind(&self) -> u16 {
        unreachable!("the parent nla will return TCA_OPTIONS")
    }
}

impl<'a, S> ParseableParametrized<NlaBuffer<&'a [u8]>, S> for Qdisc 
where
    S: AsRef<str>, 
{
    fn parse_with_param(buf: &NlaBuffer<&'a [u8]>, kind: S) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match kind.as_ref() {
            Ingress::KIND => Self::Ingress(Ingress()),
            _ => Self::Other(payload.to_vec()),
        })
    }
}
